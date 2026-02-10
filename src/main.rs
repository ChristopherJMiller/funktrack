mod audio;
mod conductor;
mod notes;
mod path;

use bevy::prelude::*;

use audio::KiraPlugin;
use conductor::ConductorPlugin;
use notes::NotesPlugin;
use path::PathPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSet {
    UpdateConductor,
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
            (
                GameSet::UpdateConductor,
                GameSet::SpawnNotes,
                GameSet::MoveNotes,
                GameSet::Render,
            )
                .chain(),
        )
        .add_systems(Startup, spawn_camera)
        .add_plugins((KiraPlugin, ConductorPlugin, PathPlugin, NotesPlugin))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
