use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::GameSet;
use crate::action::GameAction;
use crate::conductor::SongConductor;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TapInput>();
        app.add_systems(Update, read_tap_input.in_set(GameSet::ReadInput));
    }
}

#[derive(Message, Debug, Clone)]
pub struct TapInput {
    pub beat: f64,
}

fn read_tap_input(
    action: Res<ActionState<GameAction>>,
    conductor: Option<Res<SongConductor>>,
    mut tap_writer: MessageWriter<TapInput>,
) {
    let Some(conductor) = conductor else { return };
    if action.just_pressed(&GameAction::Tap) {
        tap_writer.write(TapInput {
            beat: conductor.current_beat,
        });
    }
}
