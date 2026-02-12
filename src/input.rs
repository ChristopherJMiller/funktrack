use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::GameSet;
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
    mut keys: MessageReader<KeyboardInput>,
    conductor: Option<Res<SongConductor>>,
    mut tap_writer: MessageWriter<TapInput>,
) {
    let Some(conductor) = conductor else { return };
    for ev in keys.read() {
        if ev.state == ButtonState::Pressed && !ev.repeat && ev.key_code == KeyCode::Space {
            tap_writer.write(TapInput {
                beat: conductor.current_beat,
            });
        }
    }
}
