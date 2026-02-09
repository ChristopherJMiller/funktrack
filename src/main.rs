mod notes;
mod path;

use bevy::prelude::*;

use notes::NotesPlugin;
use path::PathPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSet {
    SpawnNotes,
    MoveNotes,
    Render,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rhythm Rail".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .configure_sets(
            Update,
            (GameSet::SpawnNotes, GameSet::MoveNotes, GameSet::Render).chain(),
        )
        .add_systems(Startup, spawn_camera)
        .add_plugins((PathPlugin, NotesPlugin))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
