#![forbid(missing_docs)]
#![forbid(unsafe_code)]
#![forbid(future_incompatible)]
#![forbid(dead_code)]
//! Database Plugin for Bevy
//!
//! This module provides persistent storage capabilities for Bevy ECS components using the fjall database.
//! It automatically serializes and deserializes components, maintaining persistence across application restarts.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use fjall::{Config, Keyspace, PartitionCreateOptions};
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

// ===== Core Plugin Structure =====

/// Resource to specify the database file location
#[derive(Resource)]
pub struct DatabaseLocation(pub String);

/// Resource wrapper around fjall Keyspace
#[derive(Resource, Clone, bevy_derive::Deref, bevy_derive::DerefMut)]
pub struct KeyspaceWrapper(pub Keyspace);

#[derive(Default, Resource, bevy_derive::Deref, bevy_derive::DerefMut)]
struct DatabaseLoadMapper(pub HashMap<Entity, Entity>);

/// Main plugin struct for database functionality
pub struct DatabasePlugin;

impl Plugin for DatabasePlugin {
    fn build(&self, app: &mut App) {
        // Initialize database early
        app.add_systems(PreStartup, setup_database);
        app.add_systems(PostUpdate, cleanup_update_markers);
        app.init_resource::<DatabaseLoadMapper>();
    }
}

// ===== Component Markers =====

/// Marker component indicating the entity was just updated from the database
#[derive(Component)]
pub struct DatabaseJustUpdated;

/// Marker component to exclude an entity from database operations
#[derive(Component)]
pub struct DatabaseIgnore;

// ===== Database Setup and Management =====

/// Initializes the database connection and creates the KeyspaceWrapper resource
fn setup_database(mut commands: Commands, database_location: Option<Res<DatabaseLocation>>) {
    let database_location = database_location
        .map(|a| a.0.clone())
        .unwrap_or("./database".to_string());

    let keyspace = Config::new(database_location)
        .open()
        .expect("Failed to open database keyspace");

    commands.insert_resource(KeyspaceWrapper(keyspace));
}

/// Removes DatabaseJustUpdated markers after database operations
fn cleanup_update_markers(mut commands: Commands, query: Query<Entity, With<DatabaseJustUpdated>>) {
    for entity in query.iter() {
        commands.entity(entity).remove::<DatabaseJustUpdated>();
    }
}

// ===== Component Persistence Trait =====

/// Trait to add database mapping capabilities for components
pub trait AddDatabaseMapping {
    /// Adds database persistence for a component type. This will automatically handle
    /// saving and loading the component to/from the database.
    ///
    /// # Type Parameters
    /// * `T`: Component type that implements Serialize, Deserialize, and Component
    ///
    /// # Example
    /// ```
    /// use bevy_app::prelude::*;
    /// use bevy_ecs::prelude::*;
    /// use bevy_easy_database::*;
    ///
    /// #[derive(Component, serde::Serialize, serde::Deserialize)]
    /// pub struct Player(pub String);
    ///
    /// #[derive(Component, serde::Serialize, serde::Deserialize)]
    /// pub struct Score(pub u32);
    ///
    /// fn main() {
    ///     App::new()
    ///         //...
    ///         .add_plugins(DatabasePlugin)
    ///         // Register as many components as you need
    ///         .add_database_mapping::<Player>()
    ///         .add_database_mapping::<Score>()
    ///         .run();
    /// }
    /// ```
    fn add_database_mapping<T: Serialize + for<'de> Deserialize<'de> + Component>(
        &mut self,
    ) -> &mut Self;
}

impl AddDatabaseMapping for App {
    fn add_database_mapping<T: Serialize + for<'de> Deserialize<'de> + Component + Any>(
        &mut self,
    ) -> &mut Self {
        // Add system for loading components from database on startup
        self.add_systems(Startup, load_components::<T>);

        // Add system for saving component changes during runtime
        self.add_systems(Update, save_component_changes::<T>);

        // Add system for handling component removal
        self.add_systems(Update, handle_component_removal::<T>);

        self
    }
}

// ===== Database Operations =====

/// Loads components from the database during startup
fn load_components<T: Serialize + for<'de> Deserialize<'de> + Component>(
    mut commands: Commands,
    mut database_load_mapper: ResMut<DatabaseLoadMapper>,
    keyspace: Res<KeyspaceWrapper>,
) {
    let partition_id = get_type_partition_id::<T>();
    let partition = keyspace
        .open_partition(&partition_id, PartitionCreateOptions::default())
        .expect("Failed to open partition");

    for record in partition.iter() {
        let Ok((key, value)) = record else { continue };

        // Convert key bytes to entity ID
        // I know this is weird dunno how to do it differently though
        let mut bytes = [0; 4];
        for (i, byte) in key.as_ref().iter().enumerate() {
            bytes[i] = *byte;
        }
        
        if let Some(database_entity) = Entity::from_raw_u32(u32::from_be_bytes(bytes)) {
            // Deserialize and insert component
            let component =
                bincode::deserialize::<T>(value.as_ref()).expect("Failed to deserialize component");

            match database_load_mapper.0.get(&database_entity).cloned() {
                None => {
                    let entity = commands.spawn((component, DatabaseJustUpdated));
                    database_load_mapper.insert(database_entity, entity.id());
                }
                Some(entity) => {
                    commands.entity(entity).insert((component, DatabaseJustUpdated));
                }
            }
        }
    }
}

/// Saves changed components to the database
fn save_component_changes<T: Serialize + Component>(
    keyspace: Res<KeyspaceWrapper>,
    query: Query<
        (Entity, &T),
        (
            Changed<T>,
            (Without<DatabaseJustUpdated>, Without<DatabaseIgnore>),
        ),
    >,
) {
    let partition_id = get_type_partition_id::<T>();
    let partition = keyspace
        .open_partition(&partition_id, PartitionCreateOptions::default())
        .expect("Failed to open partition");

    for (entity, component) in query.iter() {
        let serialized = bincode::serialize(&component).expect("Failed to serialize component");

        partition
            .insert(entity.index().to_be_bytes(), serialized)
            .expect("Failed to insert into database");
    }
}

/// Handles removal of components from the database
fn handle_component_removal<T: Component>(
    keyspace: Res<KeyspaceWrapper>,
    mut removed: RemovedComponents<T>,
) {
    let partition_id = get_type_partition_id::<T>();
    let partition = keyspace
        .open_partition(&partition_id, PartitionCreateOptions::default())
        .expect("Failed to open partition");

    for entity in removed.read() {
        partition
            .remove(entity.index().to_be_bytes())
            .expect("Failed to remove from database");
    }
}

// ===== Utility Functions =====

/// Generates a unique partition ID for a given type
fn get_type_partition_id<T: Any>() -> String {
    let mut hasher = DefaultHasher::new();
    TypeId::of::<T>().hash(&mut hasher);
    format!("{}", hasher.finish())
}
