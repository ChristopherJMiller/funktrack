use bevy::prelude::*;

use crate::GameSet;
use crate::beatmap::{EventType, SelectedSong};
use crate::conductor::SongConductor;
use crate::notes::NoteQueue;
use crate::path::SplinePath;
use crate::state::GameScreen;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::Playing), init_camera_state)
            .add_systems(
                Update,
                (process_camera_events, update_camera_animations, apply_camera_transform)
                    .chain()
                    .in_set(GameSet::Render),
            )
            .add_systems(OnExit(GameScreen::Playing), cleanup_camera_state);
    }
}

// --- Easing ---

/// Cubic ease-in-out: smooth acceleration and deceleration.
fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

// --- Animation types ---

#[derive(Debug, Clone)]
struct ZoomAnim {
    start_beat: f64,
    duration_beats: f64,
    from: f32,
    to: f32,
}

#[derive(Debug, Clone)]
struct PanAnim {
    start_beat: f64,
    duration_beats: f64,
    from: Vec2,
    to: Vec2,
}

#[derive(Debug, Clone)]
struct RotateAnim {
    start_beat: f64,
    duration_beats: f64,
    from: f32,
    to: f32,
}

// --- Queued event (not yet triggered) ---

#[derive(Debug, Clone)]
struct QueuedCameraEvent {
    beat: f64,
    event: CameraEventKind,
}

#[derive(Debug, Clone)]
enum CameraEventKind {
    Zoom { scale: f32, duration_beats: f64 },
    Pan { offset: Vec2, duration_beats: f64 },
    Rotate { angle_rad: f32, duration_beats: f64 },
}

// --- Camera state resource ---

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 2.0;
const ROTATION_LIMIT_RAD: f32 = 30.0 * std::f32::consts::PI / 180.0;
const LOOK_AHEAD_BEATS: f64 = 2.0;
const LOOK_AHEAD_SMOOTHING: f32 = 3.0;

#[derive(Resource)]
struct CameraState {
    /// Queued chart events, sorted by beat (ascending). Consumed as beat passes.
    pending_events: Vec<QueuedCameraEvent>,
    /// Index into pending_events: everything before this has already been triggered.
    next_event_index: usize,

    // Active animations (at most one of each type active; new one replaces old)
    zoom_anim: Option<ZoomAnim>,
    pan_anim: Option<PanAnim>,
    rotate_anim: Option<RotateAnim>,

    // Current values from chart events
    event_zoom: f32,
    event_pan: Vec2,
    event_rotation: f32,

    // Look-ahead pan (smoothed)
    look_ahead_target: Vec2,
    look_ahead_current: Vec2,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pending_events: Vec::new(),
            next_event_index: 0,
            zoom_anim: None,
            pan_anim: None,
            rotate_anim: None,
            event_zoom: 1.0,
            event_pan: Vec2::ZERO,
            event_rotation: 0.0,
            look_ahead_target: Vec2::ZERO,
            look_ahead_current: Vec2::ZERO,
        }
    }
}

// --- Systems ---

fn init_camera_state(mut commands: Commands, selected: Option<Res<SelectedSong>>) {
    let mut state = CameraState::default();

    if let Some(selected) = selected {
        let mut events: Vec<QueuedCameraEvent> = selected
            .chart
            .events
            .iter()
            .filter_map(|e| {
                let kind = match &e.event {
                    EventType::CameraZoom {
                        scale,
                        duration_beats,
                    } => Some(CameraEventKind::Zoom {
                        scale: *scale,
                        duration_beats: *duration_beats,
                    }),
                    EventType::CameraPan {
                        offset,
                        duration_beats,
                    } => Some(CameraEventKind::Pan {
                        offset: Vec2::new(offset.0, offset.1),
                        duration_beats: *duration_beats,
                    }),
                    EventType::CameraRotate {
                        angle_degrees,
                        duration_beats,
                    } => Some(CameraEventKind::Rotate {
                        angle_rad: angle_degrees.to_radians(),
                        duration_beats: *duration_beats,
                    }),
                    _ => None,
                };
                kind.map(|k| QueuedCameraEvent {
                    beat: e.beat,
                    event: k,
                })
            })
            .collect();

        events.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap());
        state.pending_events = events;
    }

    commands.insert_resource(state);
}

fn process_camera_events(
    conductor: Option<Res<SongConductor>>,
    mut state: Option<ResMut<CameraState>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(ref mut state) = state else { return };

    let beat = conductor.current_beat;

    // Trigger any pending events whose beat has arrived
    while state.next_event_index < state.pending_events.len() {
        let event = &state.pending_events[state.next_event_index];
        if event.beat > beat {
            break;
        }

        match &event.event {
            CameraEventKind::Zoom {
                scale,
                duration_beats,
            } => {
                state.zoom_anim = Some(ZoomAnim {
                    start_beat: beat,
                    duration_beats: *duration_beats,
                    from: state.event_zoom,
                    to: scale.clamp(ZOOM_MIN, ZOOM_MAX),
                });
            }
            CameraEventKind::Pan {
                offset,
                duration_beats,
            } => {
                state.pan_anim = Some(PanAnim {
                    start_beat: beat,
                    duration_beats: *duration_beats,
                    from: state.event_pan,
                    to: *offset,
                });
            }
            CameraEventKind::Rotate {
                angle_rad,
                duration_beats,
            } => {
                state.rotate_anim = Some(RotateAnim {
                    start_beat: beat,
                    duration_beats: *duration_beats,
                    from: state.event_rotation,
                    to: angle_rad.clamp(-ROTATION_LIMIT_RAD, ROTATION_LIMIT_RAD),
                });
            }
        }

        state.next_event_index += 1;
    }
}

fn update_camera_animations(
    conductor: Option<Res<SongConductor>>,
    time: Res<Time>,
    queue: Option<Res<NoteQueue>>,
    spline: Option<Res<SplinePath>>,
    mut state: Option<ResMut<CameraState>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(ref mut state) = state else { return };

    let beat = conductor.current_beat;

    // --- Update chart-driven animations ---

    // Zoom
    if let Some(ref anim) = state.zoom_anim {
        let t = if anim.duration_beats > 0.0 {
            ((beat - anim.start_beat) / anim.duration_beats).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        let eased = ease_in_out_cubic(t);
        state.event_zoom = anim.from + (anim.to - anim.from) * eased;
        if t >= 1.0 {
            state.zoom_anim = None;
        }
    }

    // Pan
    if let Some(ref anim) = state.pan_anim {
        let t = if anim.duration_beats > 0.0 {
            ((beat - anim.start_beat) / anim.duration_beats).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        let eased = ease_in_out_cubic(t);
        state.event_pan = anim.from.lerp(anim.to, eased);
        if t >= 1.0 {
            state.pan_anim = None;
        }
    }

    // Rotation
    if let Some(ref anim) = state.rotate_anim {
        let t = if anim.duration_beats > 0.0 {
            ((beat - anim.start_beat) / anim.duration_beats).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };
        let eased = ease_in_out_cubic(t);
        state.event_rotation = anim.from + (anim.to - anim.from) * eased;
        if t >= 1.0 {
            state.rotate_anim = None;
        }
    }

    // --- Auto look-ahead panning ---

    if let (Some(queue), Some(spline)) = (queue, spline) {
        let look_start = beat;
        let look_end = beat + LOOK_AHEAD_BEATS;

        // Find notes in the look-ahead window and compute their centroid on the path
        let mut centroid = Vec2::ZERO;
        let mut count = 0u32;

        for note in queue.notes.iter().skip(queue.next_index.saturating_sub(1)) {
            if note.target_beat > look_end {
                break;
            }
            if note.target_beat >= look_start {
                // Approximate where this note is on the path right now
                let spawn_beat = note.target_beat - queue.travel_beats;
                let p = ((beat - spawn_beat) / queue.travel_beats).clamp(0.0, 1.0) as f32;
                let pos = spline.position_at_progress(p);
                centroid += pos;
                count += 1;
            }
        }

        if count > 0 {
            state.look_ahead_target = centroid / count as f32;
        } else {
            // No upcoming notes — ease back toward origin
            state.look_ahead_target = Vec2::ZERO;
        }

        // Smooth toward target
        let dt = time.delta_secs();
        let alpha = (LOOK_AHEAD_SMOOTHING * dt).min(1.0);
        state.look_ahead_current = state
            .look_ahead_current
            .lerp(state.look_ahead_target, alpha);
    }
}

fn apply_camera_transform(
    state: Option<Res<CameraState>>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Some(state) = state else { return };

    for (mut transform, mut projection) in &mut camera_q {
        // Combine event pan + look-ahead
        let final_pan = state.event_pan + state.look_ahead_current;
        transform.translation.x = final_pan.x;
        transform.translation.y = final_pan.y;

        // Rotation
        transform.rotation = Quat::from_rotation_z(state.event_rotation);

        // Zoom via projection scale
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = state.event_zoom;
        }
    }
}

fn cleanup_camera_state(
    mut commands: Commands,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    commands.remove_resource::<CameraState>();

    // Reset camera to defaults
    for (mut transform, mut projection) in &mut camera_q {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_in_out_cubic_boundaries() {
        assert!((ease_in_out_cubic(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_cubic_clamps() {
        assert!((ease_in_out_cubic(-1.0) - 0.0).abs() < 1e-6);
        assert!((ease_in_out_cubic(2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_cubic_symmetry() {
        // f(0.25) + f(0.75) should equal 1.0 for symmetric ease
        let a = ease_in_out_cubic(0.25);
        let b = ease_in_out_cubic(0.75);
        assert!((a + b - 1.0).abs() < 1e-6);
    }
}
