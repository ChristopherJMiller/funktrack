use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::GameSet;
use crate::action::GameAction;
use crate::beatmap::SlideDirection;
use crate::conductor::SongConductor;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TapInput>();
        app.add_message::<SlideInput>();
        app.add_systems(
            Update,
            (read_tap_input, read_slide_input).in_set(GameSet::ReadInput),
        );
    }
}

#[derive(Message, Debug, Clone)]
pub struct TapInput {
    pub beat: f64,
}

#[derive(Message, Debug, Clone)]
pub struct SlideInput {
    pub beat: f64,
    pub direction: SlideDirection,
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

fn read_slide_input(
    action: Res<ActionState<GameAction>>,
    conductor: Option<Res<SongConductor>>,
    mut slide_writer: MessageWriter<SlideInput>,
) {
    let Some(conductor) = conductor else { return };

    // Emit on any directional just_pressed (keyboard arrows, d-pad, or left stick)
    let any_dir_just_pressed = action.just_pressed(&GameAction::Up)
        || action.just_pressed(&GameAction::Down)
        || action.just_pressed(&GameAction::Left)
        || action.just_pressed(&GameAction::Right);

    if !any_dir_just_pressed {
        return;
    }

    // Read all currently pressed directions to compose a vector (enables diagonals)
    let mut dir = Vec2::ZERO;
    if action.pressed(&GameAction::Up) {
        dir.y += 1.0;
    }
    if action.pressed(&GameAction::Down) {
        dir.y -= 1.0;
    }
    if action.pressed(&GameAction::Right) {
        dir.x += 1.0;
    }
    if action.pressed(&GameAction::Left) {
        dir.x -= 1.0;
    }

    if let Some(slide_dir) = SlideDirection::from_vec2(dir) {
        slide_writer.write(SlideInput {
            beat: conductor.current_beat,
            direction: slide_dir,
        });
    }
}
