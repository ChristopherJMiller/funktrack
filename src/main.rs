mod audio;
mod conductor;
mod hud;
mod input;
mod judgment;
mod notes;
mod path;
mod results;
mod scoring;

use bevy::prelude::*;
use bevy::window::PresentMode;

use audio::KiraPlugin;
use conductor::ConductorPlugin;
use hud::HudPlugin;
use input::InputPlugin;
use judgment::JudgmentPlugin;
use notes::NotesPlugin;
use path::PathPlugin;
use results::ResultsPlugin;
use scoring::ScoringPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum GameSet {
    UpdateConductor,
    SpawnNotes,
    ReadInput,
    MoveNotes,
    CheckHits,
    UpdateScore,
    Render,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Rhythm Rail".into(),
                resolution: (1280, 720).into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .configure_sets(
            Update,
            (
                GameSet::UpdateConductor,
                GameSet::SpawnNotes,
                GameSet::ReadInput,
                GameSet::MoveNotes,
                GameSet::CheckHits,
                GameSet::UpdateScore,
                GameSet::Render,
            )
                .chain(),
        )
        .add_systems(Startup, spawn_camera)
        .add_plugins((
            KiraPlugin,
            ConductorPlugin,
            PathPlugin,
            NotesPlugin,
            InputPlugin,
            JudgmentPlugin,
            ScoringPlugin,
            HudPlugin,
            ResultsPlugin,
        ))
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
