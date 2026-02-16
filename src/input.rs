use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::GameSet;
use crate::action::GameAction;
use crate::beatmap::SlideDirection;
use crate::conductor::SongConductor;
use crate::judgment::beats_to_ms;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TapInput>();
        app.add_message::<SlideInput>();
        app.add_message::<ScratchInput>();
        app.add_message::<CriticalInput>();
        app.add_message::<DualSlideInput>();
        app.init_resource::<ScratchState>();
        app.init_resource::<CriticalDetector>();
        app.add_systems(
            Update,
            (read_tap_input, read_slide_input, read_scratch_input, read_dual_slide_input, detect_critical_input)
                .in_set(GameSet::ReadInput),
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

#[derive(Message, Debug, Clone)]
pub struct ScratchInput {
    pub beat: f64,
}

#[derive(Message, Debug, Clone)]
pub struct CriticalInput {
    pub beat: f64,
}

#[derive(Message, Debug, Clone)]
pub struct DualSlideInput {
    pub beat: f64,
    pub dir_a: SlideDirection,
    pub dir_b: SlideDirection,
}

#[derive(Resource, Default)]
struct ScratchState {
    prev_direction: Option<SlideDirection>,
}

#[derive(Resource, Default)]
struct CriticalDetector {
    last_tap_beat: Option<f64>,
    last_slide_beat: Option<f64>,
    last_emitted_beat: Option<f64>,
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

fn read_scratch_input(
    action: Res<ActionState<GameAction>>,
    conductor: Option<Res<SongConductor>>,
    mut state: ResMut<ScratchState>,
    mut scratch_writer: MessageWriter<ScratchInput>,
) {
    let Some(conductor) = conductor else { return };

    // Read current directional state
    let mut dir = Vec2::ZERO;
    if action.pressed(&GameAction::Up) { dir.y += 1.0; }
    if action.pressed(&GameAction::Down) { dir.y -= 1.0; }
    if action.pressed(&GameAction::Right) { dir.x += 1.0; }
    if action.pressed(&GameAction::Left) { dir.x -= 1.0; }

    let current = SlideDirection::from_vec2(dir);

    // Detect direction change (zero-crossing gesture)
    if let (Some(prev), Some(curr)) = (state.prev_direction, current) {
        if prev != curr {
            scratch_writer.write(ScratchInput {
                beat: conductor.current_beat,
            });
        }
    }

    state.prev_direction = current;
}

fn read_dual_slide_input(
    action: Res<ActionState<GameAction>>,
    conductor: Option<Res<SongConductor>>,
    mut writer: MessageWriter<DualSlideInput>,
) {
    let Some(conductor) = conductor else { return };

    let any_just = action.just_pressed(&GameAction::Up)
        || action.just_pressed(&GameAction::Down)
        || action.just_pressed(&GameAction::Left)
        || action.just_pressed(&GameAction::Right);
    if !any_just { return; }

    // Collect all currently pressed cardinal directions
    let mut pressed_dirs: Vec<SlideDirection> = Vec::new();
    if action.pressed(&GameAction::Up) { pressed_dirs.push(SlideDirection::N); }
    if action.pressed(&GameAction::Down) { pressed_dirs.push(SlideDirection::S); }
    if action.pressed(&GameAction::Left) { pressed_dirs.push(SlideDirection::W); }
    if action.pressed(&GameAction::Right) { pressed_dirs.push(SlideDirection::E); }

    // Dual slide requires exactly 2 directions pressed simultaneously
    if pressed_dirs.len() == 2 {
        writer.write(DualSlideInput {
            beat: conductor.current_beat,
            dir_a: pressed_dirs[0],
            dir_b: pressed_dirs[1],
        });
    }
}

const CRITICAL_WINDOW_MS: f64 = 30.0;

fn detect_critical_input(
    action: Res<ActionState<GameAction>>,
    conductor: Option<Res<SongConductor>>,
    mut detector: ResMut<CriticalDetector>,
    mut critical_writer: MessageWriter<CriticalInput>,
) {
    let Some(conductor) = conductor else { return };

    if action.just_pressed(&GameAction::Tap) {
        detector.last_tap_beat = Some(conductor.current_beat);
    }

    let any_dir = action.just_pressed(&GameAction::Up)
        || action.just_pressed(&GameAction::Down)
        || action.just_pressed(&GameAction::Left)
        || action.just_pressed(&GameAction::Right);
    if any_dir {
        detector.last_slide_beat = Some(conductor.current_beat);
    }

    // Check co-occurrence within ±30ms
    if let (Some(tap_beat), Some(slide_beat)) = (detector.last_tap_beat, detector.last_slide_beat) {
        let diff_ms = beats_to_ms((tap_beat - slide_beat).abs(), conductor.bpm);
        if diff_ms <= CRITICAL_WINDOW_MS {
            let emit_beat = tap_beat.min(slide_beat);
            // Avoid re-emitting for the same pair
            if detector.last_emitted_beat.map_or(true, |b| (b - emit_beat).abs() > 0.01) {
                critical_writer.write(CriticalInput { beat: emit_beat });
                detector.last_emitted_beat = Some(emit_beat);
                detector.last_tap_beat = None;
                detector.last_slide_beat = None;
            }
        }
    }

    // Expire stale timestamps
    let expire_beats = (CRITICAL_WINDOW_MS + 5.0) * conductor.bpm / 60_000.0;
    if let Some(tap_beat) = detector.last_tap_beat {
        if conductor.current_beat - tap_beat > expire_beats {
            detector.last_tap_beat = None;
        }
    }
    if let Some(slide_beat) = detector.last_slide_beat {
        if conductor.current_beat - slide_beat > expire_beats {
            detector.last_slide_beat = None;
        }
    }
}
