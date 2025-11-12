# Bevy Easy Database

A persistent storage solution for the Bevy game engine that automatically serializes and persists your components to disk using fjall as the underlying database.

## Features

- 🚀 Seamless integration with Bevy ECS
- 💾 Automatic component persistence
- 🔄 Hot-reloading of component data
- 🎯 Selective persistence with ignore markers
- 🛠 Simple setup and configuration

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
bevy_database = "0.1.1"
bevy = "0.15.2"
```

## Quick Start

```rust
use bevy::prelude::*;
use bevy_database::{DatabasePlugin, DatabaseIgnore};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DatabasePlugin)  // Add the database plugin
        .add_database_mapping::<Transform>()  // Register components you want to persist
        .run();
}
```

## Usage Guide

### Basic Setup

1. Add the `DatabasePlugin` to your Bevy app
2. Register components you want to persist using `add_database_mapping`
3. That's it! Your components will now automatically persist between sessions

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DatabasePlugin)
        // Register as many components as you need
        .add_database_mapping::<Transform>()
        .add_database_mapping::<Player>()
        .add_database_mapping::<Score>()
        .run();
}
```

### Custom Database Location

By default, the database is stored in `./database`. You can customize this by adding the `DatabaseLocation` resource:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DatabasePlugin)
        .insert_resource(DatabaseLocation("./my_game_save".to_string()))
        .add_database_mapping::<Transform>()
        .run();
}
```

### Excluding Entities from Persistence

Some entities (like cameras or temporary effects) shouldn't be persisted. Add the `DatabaseIgnore` component to exclude them:

```rust
fn setup(mut commands: Commands) {
    // This camera won't be persisted
    commands.spawn((Camera2dBundle::default(), DatabaseIgnore));
    
    // This entity will be persisted
    commands.spawn(Transform::default());
}
```

### Working with Components

Components are automatically saved when they change:

```rust
fn update_position(mut query: Query<&mut Transform>) {
    for mut transform in query.iter_mut() {
        transform.translation.x += 1.0;  // This change will be automatically persisted
    }
}
```

### Hot Reloading

The plugin automatically loads persisted components when your app starts. This means you can:

1. Start your app
2. Modify entities and components
3. Stop your app
4. Start it again - all your changes will be restored!

## Technical Details

- Components must implement `Serialize` and `Deserialize` from serde
- Entity IDs are mapped between sessions
- Changes are persisted immediately by default
- The database uses fjall for reliable storage

## Example: Game with Persistent Transforms

Here's a complete example showing how to create a simple game with persistent entity positions:

```rust
use bevy::prelude::*;
use bevy_database::{DatabasePlugin, DatabaseIgnore};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DatabasePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos)
        .add_database_mapping::<Transform>()
        .run();
}

fn setup(mut commands: Commands) {
    // Camera won't be persisted
    commands.spawn((Camera2dBundle::default(), DatabaseIgnore));
    
    // These entities will be persisted
    commands.spawn(Transform::from_xyz(0.0, 0.0, 0.0));
    commands.spawn(Transform::from_xyz(2.0, 2.0, 0.0));
}

fn draw_gizmos(mut gizmos: Gizmos, transforms: Query<&Transform, Without<Camera>>) {
    for transform in transforms.iter() {
        gizmos.sphere(transform.translation, 5.0, Color::RED);
    }
}
```

## Bevy support table

| bevy | bevy_easy_database |
|------|--------------------|
| 0.17 | 0.3                |
| 0.16 | 0.2                |
| 0.15 | 0.1                |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT + Apache License

## Credits

Built with ❤️ for the Bevy community. Uses fjall for database operations.
