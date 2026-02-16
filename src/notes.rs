use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::GameSet;
use crate::beatmap::SlideDirection;
use crate::conductor::SongConductor;
use crate::path::SplinePath;

pub struct NotesPlugin;

impl Plugin for NotesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_notes.in_set(GameSet::SpawnNotes))
            .add_systems(Update, move_notes.in_set(GameSet::MoveNotes))
            .add_systems(Update, render_notes.in_set(GameSet::Render));
    }
}

// --- Components ---

#[derive(Debug, Clone, Copy)]
pub enum NoteKind {
    Tap,
    Slide(SlideDirection),
    Hold { end_beat: f64 },
    AdLib,
    Beat,
    Scratch,
    Critical,
    DualSlide(SlideDirection, SlideDirection),
}

#[derive(Component)]
pub struct HoldEndBeat(pub f64);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldState {
    Pending,
    Held,
    Completed,
    Dropped,
}

#[derive(Component)]
pub struct NoteType(pub NoteKind);

#[derive(Component)]
pub struct NoteDirection(pub SlideDirection);

#[derive(Component)]
pub struct NoteTiming {
    pub target_beat: f64,
    pub spawn_beat: f64,
    pub travel_beats: f64,
}

#[derive(Component)]
pub struct NoteProgress(pub f32);

#[derive(Component)]
pub struct NoteAlive;

/// Marker for Ad-Lib notes: silently despawn on miss (no penalty).
#[derive(Component)]
pub struct AdLibMarker;

/// Tracks rapid tap count for Beat notes (needs 2+ taps to clear).
#[derive(Component)]
pub struct BeatTapCount {
    pub count: u8,
    pub first_tap_ms: f64,
}

/// Stores both directions for a Dual Slide note.
#[derive(Component)]
pub struct DualSlideDirections(pub SlideDirection, pub SlideDirection);

// --- Resources ---

pub struct ChartNote {
    pub target_beat: f64,
    pub kind: NoteKind,
}

#[derive(Resource)]
pub struct NoteQueue {
    pub notes: Vec<ChartNote>,
    pub next_index: usize,
    pub look_ahead_beats: f64,
    pub travel_beats: f64,
}

// --- Systems ---

fn spawn_notes(
    mut commands: Commands,
    conductor: Option<Res<SongConductor>>,
    queue: Option<ResMut<NoteQueue>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(mut queue) = queue else { return };

    if !conductor.playing {
        return;
    }

    let horizon = conductor.current_beat + queue.look_ahead_beats;

    while queue.next_index < queue.notes.len() {
        let note = &queue.notes[queue.next_index];
        let spawn_beat = note.target_beat - queue.travel_beats;
        if spawn_beat > horizon {
            break;
        }
        let entity = commands.spawn((
            NoteType(note.kind),
            NoteTiming {
                target_beat: note.target_beat,
                spawn_beat,
                travel_beats: queue.travel_beats,
            },
            NoteProgress(0.0),
            NoteAlive,
        )).id();
        match note.kind {
            NoteKind::Slide(dir) => {
                commands.entity(entity).insert(NoteDirection(dir));
            }
            NoteKind::Hold { end_beat } => {
                commands.entity(entity).insert((HoldEndBeat(end_beat), HoldState::Pending));
            }
            NoteKind::AdLib => {
                commands.entity(entity).insert(AdLibMarker);
            }
            NoteKind::Beat => {
                commands.entity(entity).insert(BeatTapCount { count: 0, first_tap_ms: 0.0 });
            }
            NoteKind::DualSlide(a, b) => {
                commands.entity(entity).insert(DualSlideDirections(a, b));
            }
            _ => {}
        }
        queue.next_index += 1;
    }
}

fn move_notes(
    conductor: Option<Res<SongConductor>>,
    mut query: Query<(&NoteTiming, &mut NoteProgress)>,
) {
    let Some(conductor) = conductor else { return };

    for (timing, mut progress) in &mut query {
        let p = (conductor.current_beat - timing.spawn_beat) / timing.travel_beats;
        progress.0 = p.max(0.0) as f32;
    }
}

fn render_notes(
    query: Query<
        (&NoteProgress, &NoteType, Option<&NoteDirection>, Option<&HoldEndBeat>, Option<&HoldState>, &NoteTiming, Option<&DualSlideDirections>),
        With<NoteAlive>,
    >,
    conductor: Option<Res<SongConductor>>,
    spline: Option<Res<SplinePath>>,
    mut gizmos: Gizmos,
) {
    let Some(spline) = spline else { return };
    let Some(conductor) = conductor else { return };

    let tap_color = Color::srgb(1.0, 0.4, 0.7);
    let tangent_color = Color::srgb(1.0, 0.8, 0.3);
    let slide_color = Color::srgb(0.0, 0.9, 1.0);
    let hold_color = Color::srgb(1.0, 0.85, 0.15);
    let hold_held_color = Color::srgb(1.0, 0.95, 0.5);
    let hold_dropped_color = Color::srgb(0.5, 0.4, 0.1);
    let beat_color = Color::srgb(0.8, 0.3, 1.0);
    let scratch_color = Color::srgb(1.0, 0.5, 0.1);
    let critical_color = Color::srgb(1.0, 0.95, 0.8);
    let dual_slide_color = Color::srgb(0.4, 0.9, 1.0);

    for (progress, note_type, note_dir, _hold_end, hold_state, timing, dual_dirs) in &query {
        let pos = spline.position_at_progress(progress.0.min(1.0));

        match note_type.0 {
            NoteKind::Tap => {
                let tangent = spline.tangent_at_progress(progress.0).normalize_or_zero();
                gizmos.circle_2d(pos, 12.0, tap_color);
                gizmos.line_2d(pos, pos + tangent * 20.0, tangent_color);
            }
            NoteKind::Slide(_) => {
                let dir_vec = note_dir
                    .map(|d| d.0.to_vec2())
                    .unwrap_or(Vec2::X);
                let size = 14.0;

                // Diamond outline
                let up = pos + Vec2::Y * size;
                let down = pos - Vec2::Y * size;
                let left = pos - Vec2::X * size;
                let right = pos + Vec2::X * size;
                gizmos.line_2d(up, right, slide_color);
                gizmos.line_2d(right, down, slide_color);
                gizmos.line_2d(down, left, slide_color);
                gizmos.line_2d(left, up, slide_color);

                // Arrow shaft
                let shaft_len = 10.0;
                let shaft_start = pos - dir_vec * shaft_len * 0.5;
                let shaft_end = pos + dir_vec * shaft_len * 0.5;
                gizmos.line_2d(shaft_start, shaft_end, slide_color);

                // Arrowhead
                let perp = Vec2::new(-dir_vec.y, dir_vec.x);
                let head_size = 5.0;
                let head_base = shaft_end - dir_vec * head_size;
                gizmos.line_2d(shaft_end, head_base + perp * head_size * 0.5, slide_color);
                gizmos.line_2d(shaft_end, head_base - perp * head_size * 0.5, slide_color);
            }
            NoteKind::Hold { end_beat } => {
                let state = hold_state.copied().unwrap_or(HoldState::Pending);
                let color = match state {
                    HoldState::Held => hold_held_color,
                    HoldState::Dropped => hold_dropped_color,
                    _ => hold_color,
                };

                // Head position: clamp to 1.0 once held (stays at judgment line)
                let head_p = if state == HoldState::Held {
                    progress.0.min(1.0)
                } else {
                    progress.0
                };
                let head_pos = spline.position_at_progress(head_p.min(1.0));

                // Tail position: travels along the spline behind the head
                let tail_spawn_beat = end_beat - timing.travel_beats;
                let tail_p = ((conductor.current_beat - tail_spawn_beat) / timing.travel_beats)
                    .clamp(0.0, 1.0) as f32;
                let tail_pos = spline.position_at_progress(tail_p);

                // Body ribbon: line segments along spline between tail and head
                let segments = 16;
                let p_start = tail_p.min(head_p);
                let p_end = tail_p.max(head_p);
                if p_end > p_start {
                    let step = (p_end - p_start) / segments as f32;
                    for i in 0..segments {
                        let pa = p_start + step * i as f32;
                        let pb = p_start + step * (i + 1) as f32;
                        let a = spline.position_at_progress(pa);
                        let b = spline.position_at_progress(pb);
                        let tangent_a = spline.tangent_at_progress(pa).normalize_or_zero();
                        let tangent_b = spline.tangent_at_progress(pb).normalize_or_zero();
                        let perp_a = Vec2::new(-tangent_a.y, tangent_a.x) * 4.0;
                        let perp_b = Vec2::new(-tangent_b.y, tangent_b.x) * 4.0;
                        gizmos.line_2d(a + perp_a, b + perp_b, color);
                        gizmos.line_2d(a - perp_a, b - perp_b, color);
                    }
                }

                // Head: double circle
                gizmos.circle_2d(head_pos, 14.0, color);
                gizmos.circle_2d(head_pos, 11.0, color);

                // Tail: smaller circle
                gizmos.circle_2d(tail_pos, 8.0, color);
            }
            NoteKind::AdLib => {
                // Ghostly, near-invisible — subtle pulsing circle
                let pulse = 0.08 + 0.06 * (conductor.current_beat as f32 * TAU).sin().abs();
                let adlib_color = Color::srgba(0.9, 0.9, 1.0, pulse);
                gizmos.circle_2d(pos, 10.0, adlib_color);
            }
            NoteKind::Beat => {
                // Pulsing concentric rings — electric purple
                let pulse = 1.0 + 0.15 * (conductor.current_beat as f32 * TAU * 2.0).sin();
                gizmos.circle_2d(pos, 10.0, beat_color);
                gizmos.circle_2d(pos, 16.0 * pulse, beat_color);
            }
            NoteKind::Scratch => {
                // Spinning disc with motion lines — hot orange
                gizmos.circle_2d(pos, 13.0, scratch_color);
                let spin = conductor.current_beat as f32 * TAU * 2.0;
                for i in 0..3 {
                    let angle = spin + (i as f32 * TAU / 3.0);
                    let d = Vec2::new(angle.cos(), angle.sin());
                    gizmos.line_2d(pos + d * 10.0, pos + d * 18.0, scratch_color);
                }
            }
            NoteKind::Critical => {
                // 5-point star — white/gold
                let outer_r = 16.0;
                let inner_r = 8.0;
                let num_points = 5;
                for i in 0..num_points {
                    let a1 = (i as f32 / num_points as f32) * TAU - std::f32::consts::FRAC_PI_2;
                    let a2 = ((i as f32 + 0.5) / num_points as f32) * TAU - std::f32::consts::FRAC_PI_2;
                    let a3 = ((i + 1) as f32 / num_points as f32) * TAU - std::f32::consts::FRAC_PI_2;
                    let p1 = pos + Vec2::new(a1.cos(), a1.sin()) * outer_r;
                    let p2 = pos + Vec2::new(a2.cos(), a2.sin()) * inner_r;
                    let p3 = pos + Vec2::new(a3.cos(), a3.sin()) * outer_r;
                    gizmos.line_2d(p1, p2, critical_color);
                    gizmos.line_2d(p2, p3, critical_color);
                }
            }
            NoteKind::DualSlide(_, _) => {
                // Wider diamond with two directional arrows
                let (dir_a, dir_b) = dual_dirs
                    .map(|d| (d.0.to_vec2(), d.1.to_vec2()))
                    .unwrap_or((Vec2::X, Vec2::NEG_X));
                let size = 18.0;

                // Diamond outline
                let up = pos + Vec2::Y * size;
                let down = pos - Vec2::Y * size;
                let left = pos - Vec2::X * size;
                let right = pos + Vec2::X * size;
                gizmos.line_2d(up, right, dual_slide_color);
                gizmos.line_2d(right, down, dual_slide_color);
                gizmos.line_2d(down, left, dual_slide_color);
                gizmos.line_2d(left, up, dual_slide_color);

                // Two arrow shafts
                for dir_vec in [dir_a, dir_b] {
                    let shaft_len = 8.0;
                    let shaft_start = pos - dir_vec * shaft_len * 0.5;
                    let shaft_end = pos + dir_vec * shaft_len * 0.5;
                    gizmos.line_2d(shaft_start, shaft_end, dual_slide_color);

                    let perp = Vec2::new(-dir_vec.y, dir_vec.x);
                    let head_size = 4.0;
                    let head_base = shaft_end - dir_vec * head_size;
                    gizmos.line_2d(shaft_end, head_base + perp * head_size * 0.5, dual_slide_color);
                    gizmos.line_2d(shaft_end, head_base - perp * head_size * 0.5, dual_slide_color);
                }
            }
        }
    }
}
