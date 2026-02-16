use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameScreen {
    #[default]
    SongSelect,
    Playing,
    Paused,
    Results,
    Settings,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameScreen>();
    }
}
