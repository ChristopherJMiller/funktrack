mod action;
mod audio;
mod beatmap;
mod calibration;
mod camera;
mod conductor;
mod config;
mod editor;
mod hud;
mod input;
mod judgment;
mod notes;
mod particles;
mod pause;
mod path;
mod results;
mod scoring;
mod settings;
mod song_select;
mod state;
mod visuals;

use bevy::prelude::*;
use bevy::window::PresentMode;

use action::ActionPlugin;
use audio::KiraPlugin;
use beatmap::BeatMapPlugin;
use calibration::CalibrationPlugin;
use camera::CameraPlugin;
use conductor::ConductorPlugin;
use config::ConfigPlugin;
use editor::EditorPluginBundle;
use hud::HudPlugin;
use input::InputPlugin;
use judgment::JudgmentPlugin;
use notes::NotesPlugin;
use particles::ParticlePlugin;
use pause::PausePlugin;
use path::PathPlugin;
use results::ResultsPlugin;
use scoring::ScoringPlugin;
use settings::SettingsPlugin;
use song_select::SongSelectPlugin;
use state::{GameScreen, GameStatePlugin};
use visuals::VisualsPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum GameSet {
    UpdateConductor,
    SpawnNotes,
    ReadInput,
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
        .add_plugins(GameStatePlugin)
        .configure_sets(
            Update,
            (
                GameSet::UpdateConductor,
                GameSet::SpawnNotes,
                GameSet::ReadInput,
                GameSet::CheckHits,
                GameSet::UpdateScore,
                GameSet::Render,
            )
                .chain()
                .run_if(in_state(GameScreen::Playing)),
        )
        .add_systems(Startup, spawn_camera)
        .add_plugins((
            ActionPlugin,
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
        .add_plugins((
            BeatMapPlugin,
            SongSelectPlugin,
            SettingsPlugin,
            PausePlugin,
            ParticlePlugin,
            VisualsPlugin,
            CameraPlugin,
            ConfigPlugin,
            CalibrationPlugin,
        ))
        .add_plugins(EditorPluginBundle)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
