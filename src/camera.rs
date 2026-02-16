use bevy::prelude::*;

use crate::GameSet;
use crate::beatmap::{EventType, SelectedSong};
use crate::conductor::SongConductor;
use crate::notes::Playhead;
use crate::path::SplinePath;
use crate::state::GameScreen;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameScreen::Playing), init_camera_state)
            .add_systems(
                Update,
                (process_camera_events, update_camera, apply_camera_transform)
                    .chain()
                    .in_set(GameSet::Render),
            )
            .add_systems(OnExit(GameScreen::Playing), cleanup_camera_state);
    }
}

// --- Easing ---

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

// --- Constants ---

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 2.0;
const ROTATION_LIMIT_RAD: f32 = 30.0 * std::f32::consts::PI / 180.0;

/// How far ahead of the playhead the camera looks (in spline progress units).
const LOOK_AHEAD_OFFSET: f32 = 0.04;
/// Weight of look-ahead position vs playhead position (0.0 = all playhead, 1.0 = all look-ahead).
const LOOK_AHEAD_WEIGHT: f32 = 0.35;

/// Exponential smoothing factor for camera position (higher = snappier).
const POSITION_SMOOTHING: f32 = 6.0;
/// Exponential smoothing factor for track-following rotation.
const ROTATION_SMOOTHING: f32 = 3.0;
/// Max angular speed in radians/sec to prevent jarring snaps.
const MAX_ANGULAR_SPEED: f32 = 2.5;
/// Rotation intensity: 0.0 = never rotate, 1.0 = full track-following.
const ROTATION_INTENSITY: f32 = 0.6;

/// Window (in progress units) over which we sample tangent change for curvature.
const CURVATURE_SAMPLE_WINDOW: f32 = 0.02;
/// When curvature exceeds this threshold, smoothing increases.
const HIGH_CURVATURE_THRESHOLD: f32 = 1.5;
/// Extra smoothing multiplier during high curvature.
const CURVATURE_SMOOTHING_BOOST: f32 = 0.3;

// --- Camera state resource ---

#[derive(Resource)]
struct CameraState {
    // Chart event queue
    pending_events: Vec<QueuedCameraEvent>,
    next_event_index: usize,

    // Active animations
    zoom_anim: Option<ZoomAnim>,
    pan_anim: Option<PanAnim>,
    rotate_anim: Option<RotateAnim>,

    // Current values from chart events (additive overlays)
    event_zoom: f32,
    event_pan: Vec2,
    event_rotation: f32,

    // Playhead tracking state
    camera_position: Vec2,
    playhead_angle: f32,
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
            camera_position: Vec2::ZERO,
            playhead_angle: 0.0,
        }
    }
}

// --- Systems ---

fn init_camera_state(
    mut commands: Commands,
    selected: Option<Res<SelectedSong>>,
    spline: Option<Res<SplinePath>>,
) {
    let mut state = CameraState::default();

    // Initialize camera to spline start
    if let Some(ref spline) = spline {
        state.camera_position = spline.position_at_progress(0.0);
        let tangent = spline.tangent_at_progress(0.0);
        state.playhead_angle = tangent.y.atan2(tangent.x);
    }

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

fn update_camera(
    conductor: Option<Res<SongConductor>>,
    time: Res<Time>,
    playhead: Option<Res<Playhead>>,
    spline: Option<Res<SplinePath>>,
    mut state: Option<ResMut<CameraState>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(ref mut state) = state else { return };

    let beat = conductor.current_beat;
    let dt = time.delta_secs();

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

    // --- Playhead tracking ---

    if let (Some(playhead), Some(spline)) = (playhead, spline) {
        let progress = playhead.progress(beat);
        let playhead_pos = spline.position_at_progress(progress);

        // Look ahead slightly for better note visibility
        let look_ahead_progress = (progress + LOOK_AHEAD_OFFSET).min(1.0);
        let look_ahead_pos = spline.position_at_progress(look_ahead_progress);

        // Blend playhead and look-ahead positions
        let target_pos = playhead_pos.lerp(look_ahead_pos, LOOK_AHEAD_WEIGHT);

        // Compute curvature factor to increase smoothing on sharp turns
        let curvature = compute_curvature(&spline, progress);
        let curvature_factor = if curvature > HIGH_CURVATURE_THRESHOLD {
            CURVATURE_SMOOTHING_BOOST
        } else {
            1.0
        };
        let effective_smoothing = POSITION_SMOOTHING * curvature_factor;

        // Smooth-follow camera position
        let alpha = (effective_smoothing * dt).min(1.0);
        state.camera_position = state.camera_position.lerp(target_pos, alpha);

        // Track-following rotation
        let tangent = spline.tangent_at_progress(progress).normalize_or_zero();
        if tangent.length_squared() > 0.01 {
            let target_angle = tangent.y.atan2(tangent.x) * ROTATION_INTENSITY;

            // Angular difference with wrapping
            let mut angle_diff = target_angle - state.playhead_angle;
            // Wrap to [-PI, PI]
            angle_diff = (angle_diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;

            // Cap angular speed
            let max_delta = MAX_ANGULAR_SPEED * dt;
            let clamped_diff = angle_diff.clamp(-max_delta, max_delta);

            let rot_alpha = (ROTATION_SMOOTHING * dt).min(1.0);
            state.playhead_angle += clamped_diff * rot_alpha;
        }
    }
}

/// Estimate path curvature at a given progress by comparing tangent directions
/// over a small window.
fn compute_curvature(spline: &SplinePath, progress: f32) -> f32 {
    let half = CURVATURE_SAMPLE_WINDOW * 0.5;
    let p0 = (progress - half).max(0.0);
    let p1 = (progress + half).min(1.0);

    let t0 = spline.tangent_at_progress(p0).normalize_or_zero();
    let t1 = spline.tangent_at_progress(p1).normalize_or_zero();

    if t0.length_squared() < 0.01 || t1.length_squared() < 0.01 {
        return 0.0;
    }

    // Angle between the two tangent vectors
    let dot = t0.dot(t1).clamp(-1.0, 1.0);
    let angle = dot.acos();

    let dp = p1 - p0;
    if dp > 0.0 { angle / dp } else { 0.0 }
}

fn apply_camera_transform(
    state: Option<Res<CameraState>>,
    mut camera_q: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Some(state) = state else { return };

    for (mut transform, mut projection) in &mut camera_q {
        // Combine playhead tracking + event pan overlay
        let final_pos = state.camera_position + state.event_pan;
        transform.translation.x = final_pos.x;
        transform.translation.y = final_pos.y;

        // Combine playhead rotation + event rotation overlay
        let final_rotation = state.playhead_angle + state.event_rotation;
        transform.rotation = Quat::from_rotation_z(final_rotation);

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
        let a = ease_in_out_cubic(0.25);
        let b = ease_in_out_cubic(0.75);
        assert!((a + b - 1.0).abs() < 1e-6);
    }
}
