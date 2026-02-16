use bevy::prelude::*;

use leafwing_input_manager::prelude::*;

use crate::GameSet;
use crate::action::GameAction;
use crate::conductor::SongConductor;
use crate::input::{CriticalInput, DualSlideInput, ScratchInput, SlideInput, TapInput};
use crate::notes::{AdLibMarker, BeatTapCount, DualSlideDirections, HoldEndBeat, HoldState, NoteAlive, NoteDirection, NoteKind, NoteTiming, NoteType, Playhead};
use crate::path::SplinePath;
use crate::state::GameScreen;
use crate::visuals::spawn_feedback_visual;

pub struct JudgmentPlugin;

impl Plugin for JudgmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<JudgmentResult>();
        app.add_systems(
            Update,
            (check_hits, check_holds, despawn_missed)
                .chain()
                .in_set(GameSet::CheckHits),
        )
        .add_systems(
            Update,
            spawn_feedback.in_set(GameSet::UpdateScore),
        )
        .add_systems(
            Update,
            cleanup_feedback.in_set(GameSet::Render),
        );
    }
}

// --- Timing windows (milliseconds) ---

const GREAT_WINDOW_MS: f64 = 20.0;
const COOL_WINDOW_MS: f64 = 50.0;
const GOOD_WINDOW_MS: f64 = 100.0;
const MISS_WINDOW_MS: f64 = 100.0;

const FEEDBACK_LIFETIME: f32 = 0.6;

// --- Y2K Future Punk palette (Jet Set Radio vibes) ---

/// Neon green — electric, triumphant
const GREAT_COLOR: Color = Color::srgb(0.0, 1.0, 0.4);
/// Electric cyan-blue — slick, stylish
const COOL_COLOR: Color = Color::srgb(0.0, 0.7, 1.0);
/// Hot yellow-orange — warning flare
const GOOD_COLOR: Color = Color::srgb(1.0, 0.85, 0.0);
/// Aggressive magenta-red — spray paint slash
const MISS_COLOR: Color = Color::srgb(1.0, 0.15, 0.3);

// --- Types ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Judgment {
    Great,
    Cool,
    Good,
    Miss,
}

impl Judgment {
    pub fn color(&self) -> Color {
        match self {
            Judgment::Great => GREAT_COLOR,
            Judgment::Cool => COOL_COLOR,
            Judgment::Good => GOOD_COLOR,
            Judgment::Miss => MISS_COLOR,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Judgment::Great => "GREAT",
            Judgment::Cool => "COOL",
            Judgment::Good => "GOOD",
            Judgment::Miss => "MISS",
        }
    }
}

/// Message emitted by check_hits/despawn_missed, consumed by spawn_feedback and update_score.
#[derive(Message)]
pub struct JudgmentResult {
    pub judgment: Judgment,
    pub position: Vec2,
}

#[derive(Component)]
pub struct JudgmentFeedback {
    pub judgment: Judgment,
    pub position: Vec2,
    pub timer: f32,
    pub max_time: f32,
}

// --- Helpers ---

pub(crate) fn beats_to_ms(beat_diff: f64, bpm: f64) -> f64 {
    beat_diff * 60_000.0 / bpm
}

pub(crate) fn ms_to_beats(ms: f64, bpm: f64) -> f64 {
    ms * bpm / 60_000.0
}

fn grade_timing(abs_diff_ms: f64) -> Option<Judgment> {
    if abs_diff_ms <= GREAT_WINDOW_MS {
        Some(Judgment::Great)
    } else if abs_diff_ms <= COOL_WINDOW_MS {
        Some(Judgment::Cool)
    } else if abs_diff_ms <= GOOD_WINDOW_MS {
        Some(Judgment::Good)
    } else {
        None
    }
}

// --- Systems ---

fn check_hits(
    mut commands: Commands,
    mut tap_reader: MessageReader<TapInput>,
    mut slide_reader: MessageReader<SlideInput>,
    mut scratch_reader: MessageReader<ScratchInput>,
    mut critical_reader: MessageReader<CriticalInput>,
    mut dual_slide_reader: MessageReader<DualSlideInput>,
    mut notes: Query<(Entity, &NoteTiming, &NoteType, Option<&NoteDirection>, Option<&HoldState>, Option<&mut BeatTapCount>, Option<&DualSlideDirections>), With<NoteAlive>>,
    conductor: Option<Res<SongConductor>>,
    spline: Option<Res<SplinePath>>,
    playhead: Option<Res<Playhead>>,
    mut results: MessageWriter<JudgmentResult>,
) {
    let Some(conductor) = conductor else { return };
    let Some(spline) = spline else { return };
    let Some(playhead) = playhead else { return };
    let pos = spline.position_at_progress(playhead.progress(conductor.current_beat));
    let mut consumed: Vec<Entity> = Vec::new();

    // --- Critical inputs (process first — most specific, consumes before Tap/Slide) ---
    for critical in critical_reader.read() {
        let mut best: Option<(Entity, f64)> = None;

        for (entity, timing, note_type, _, _, _, _) in &notes {
            if !matches!(note_type.0, NoteKind::Critical) { continue; }
            if consumed.contains(&entity) { continue; }

            let diff_beats = (critical.beat - timing.target_beat).abs();
            let diff_ms = beats_to_ms(diff_beats, conductor.bpm);

            if diff_ms <= GOOD_WINDOW_MS {
                if best.is_none() || diff_ms < best.unwrap().1 {
                    best = Some((entity, diff_ms));
                }
            }
        }

        if let Some((entity, diff_ms)) = best {
            consumed.push(entity);
            let grade = grade_timing(diff_ms).unwrap();
            info!("{} (Critical) — {:.1}ms", grade.label(), diff_ms);
            commands.entity(entity).despawn();
            results.write(JudgmentResult { judgment: grade, position: pos });
        }
    }

    // --- Dual Slide inputs (process before regular Slide to avoid ambiguity) ---
    for dual in dual_slide_reader.read() {
        let mut best: Option<(Entity, f64)> = None;

        for (entity, timing, note_type, _, _, _, dual_dirs) in &notes {
            if !matches!(note_type.0, NoteKind::DualSlide(_, _)) { continue; }
            if consumed.contains(&entity) { continue; }
            let Some(dd) = dual_dirs else { continue; };

            // Check both orderings
            let matches = (dual.dir_a == dd.0 && dual.dir_b == dd.1)
                || (dual.dir_a == dd.1 && dual.dir_b == dd.0);
            if !matches { continue; }

            let diff_beats = (dual.beat - timing.target_beat).abs();
            let diff_ms = beats_to_ms(diff_beats, conductor.bpm);

            if diff_ms <= GOOD_WINDOW_MS {
                if best.is_none() || diff_ms < best.unwrap().1 {
                    best = Some((entity, diff_ms));
                }
            }
        }

        if let Some((entity, diff_ms)) = best {
            consumed.push(entity);
            let grade = grade_timing(diff_ms).unwrap();
            info!("{} (DualSlide) — {:.1}ms", grade.label(), diff_ms);
            commands.entity(entity).despawn();
            results.write(JudgmentResult { judgment: grade, position: pos });
        }
    }

    // --- Scratch inputs ---
    for scratch in scratch_reader.read() {
        let mut best: Option<(Entity, f64)> = None;

        for (entity, timing, note_type, _, _, _, _) in &notes {
            if !matches!(note_type.0, NoteKind::Scratch) { continue; }
            if consumed.contains(&entity) { continue; }

            let diff_beats = (scratch.beat - timing.target_beat).abs();
            let diff_ms = beats_to_ms(diff_beats, conductor.bpm);

            if diff_ms <= GOOD_WINDOW_MS {
                if best.is_none() || diff_ms < best.unwrap().1 {
                    best = Some((entity, diff_ms));
                }
            }
        }

        if let Some((entity, diff_ms)) = best {
            consumed.push(entity);
            let grade = grade_timing(diff_ms).unwrap();
            info!("{} (Scratch) — {:.1}ms", grade.label(), diff_ms);
            commands.entity(entity).despawn();
            results.write(JudgmentResult { judgment: grade, position: pos });
        }
    }

    // --- Tap inputs hit Tap, AdLib, pending Hold heads, and Beat notes ---
    for tap in tap_reader.read() {
        // First pass: find best Tap / Hold / AdLib match
        let mut best: Option<(Entity, f64, bool)> = None; // (entity, diff_ms, is_hold)

        for (entity, timing, note_type, _, hold_state, _, _) in &notes {
            let is_tap = matches!(note_type.0, NoteKind::Tap);
            let is_adlib = matches!(note_type.0, NoteKind::AdLib);
            let is_pending_hold = matches!(note_type.0, NoteKind::Hold { .. })
                && hold_state.map_or(false, |s| *s == HoldState::Pending);

            if !is_tap && !is_pending_hold && !is_adlib {
                continue;
            }
            if consumed.contains(&entity) { continue; }

            let diff_beats = (tap.beat - timing.target_beat).abs();
            let diff_ms = beats_to_ms(diff_beats, conductor.bpm);

            if diff_ms <= GOOD_WINDOW_MS {
                if best.is_none() || diff_ms < best.unwrap().1 {
                    best = Some((entity, diff_ms, is_pending_hold));
                }
            }
        }

        if let Some((entity, diff_ms, is_hold)) = best {
            consumed.push(entity);
            let grade = grade_timing(diff_ms).unwrap();

            if is_hold {
                info!("{} (Hold head) — {:.1}ms", grade.label(), diff_ms);
                commands.entity(entity).insert(HoldState::Held);
            } else {
                info!("{} — {:.1}ms", grade.label(), diff_ms);
                commands.entity(entity).despawn();
            }

            results.write(JudgmentResult { judgment: grade, position: pos });
        }

        // Second pass: also increment Beat notes in window (Beat notes aren't consumed on first tap)
        for (entity, timing, note_type, _, _, mut beat_count, _) in &mut notes {
            if !matches!(note_type.0, NoteKind::Beat) { continue; }
            if consumed.contains(&entity) { continue; }

            let diff_beats = (tap.beat - timing.target_beat).abs();
            let diff_ms = beats_to_ms(diff_beats, conductor.bpm);

            if diff_ms <= GOOD_WINDOW_MS {
                if let Some(ref mut bc) = beat_count {
                    if bc.count == 0 {
                        bc.first_tap_ms = diff_ms;
                    }
                    bc.count += 1;
                    if bc.count >= 2 {
                        consumed.push(entity);
                        let grade = grade_timing(bc.first_tap_ms).unwrap();
                        info!("{} (Beat) — {:.1}ms", grade.label(), bc.first_tap_ms);
                        commands.entity(entity).despawn();
                        results.write(JudgmentResult { judgment: grade, position: pos });
                    }
                }
            }
        }
    }

    // --- Slide inputs hit only matching-direction Slide notes ---
    for slide in slide_reader.read() {
        let mut best: Option<(Entity, f64)> = None;

        for (entity, timing, note_type, note_dir, _, _, _) in &notes {
            if !matches!(note_type.0, NoteKind::Slide(_)) { continue; }
            if consumed.contains(&entity) { continue; }
            if let Some(nd) = note_dir {
                if nd.0 != slide.direction { continue; }
            }

            let diff_beats = (slide.beat - timing.target_beat).abs();
            let diff_ms = beats_to_ms(diff_beats, conductor.bpm);

            if diff_ms <= GOOD_WINDOW_MS {
                if best.is_none() || diff_ms < best.unwrap().1 {
                    best = Some((entity, diff_ms));
                }
            }
        }

        if let Some((entity, diff_ms)) = best {
            consumed.push(entity);
            let grade = grade_timing(diff_ms).unwrap();
            info!("{} (Slide {:?}) — {:.1}ms", grade.label(), slide.direction, diff_ms);
            commands.entity(entity).despawn();
            results.write(JudgmentResult { judgment: grade, position: pos });
        }
    }
}

fn check_holds(
    mut commands: Commands,
    action: Res<ActionState<GameAction>>,
    holds: Query<(Entity, &HoldEndBeat, &HoldState), With<NoteAlive>>,
    conductor: Option<Res<SongConductor>>,
    spline: Option<Res<SplinePath>>,
    playhead: Option<Res<Playhead>>,
    mut results: MessageWriter<JudgmentResult>,
) {
    let Some(conductor) = conductor else { return };
    let Some(spline) = spline else { return };
    let Some(playhead) = playhead else { return };
    let pos = spline.position_at_progress(playhead.progress(conductor.current_beat));
    let tap_held = action.pressed(&GameAction::Tap);

    for (entity, hold_end, hold_state) in &holds {
        if *hold_state != HoldState::Held {
            continue;
        }

        let end_beat = hold_end.0;
        let diff_beats = (conductor.current_beat - end_beat).abs();
        let diff_ms = beats_to_ms(diff_beats, conductor.bpm);
        let past_end = conductor.current_beat > end_beat;

        if !tap_held {
            // Player released — check if within tail window
            if diff_ms <= GOOD_WINDOW_MS {
                let grade = grade_timing(diff_ms).unwrap();
                info!(
                    "{} (Hold tail) — {:.1}ms",
                    grade.label(),
                    diff_ms
                );
                commands.entity(entity).despawn();
                results.write(JudgmentResult {
                    judgment: grade,
                    position: pos,
                });
            } else {
                // Released too early — MISS the tail
                info!("MISS (Hold tail) — released early");
                commands.entity(entity).insert(HoldState::Dropped);
                commands.entity(entity).despawn();
                results.write(JudgmentResult {
                    judgment: Judgment::Miss,
                    position: pos,
                });
            }
        } else if past_end && diff_ms > GOOD_WINDOW_MS {
            // Held past the tail + miss window — auto-GREAT
            info!("GREAT (Hold tail) — held through");
            commands.entity(entity).despawn();
            results.write(JudgmentResult {
                judgment: Judgment::Great,
                position: pos,
            });
        }
    }
}

fn despawn_missed(
    mut commands: Commands,
    notes: Query<(Entity, &NoteTiming, &NoteType, Option<&HoldState>, Option<&AdLibMarker>), With<NoteAlive>>,
    conductor: Option<Res<SongConductor>>,
    spline: Option<Res<SplinePath>>,
    playhead: Option<Res<Playhead>>,
    mut results: MessageWriter<JudgmentResult>,
) {
    let Some(conductor) = conductor else { return };
    let Some(spline) = spline else { return };
    let Some(playhead) = playhead else { return };
    let miss_beats = ms_to_beats(MISS_WINDOW_MS, conductor.bpm);

    for (entity, timing, note_type, hold_state, adlib) in &notes {
        if conductor.current_beat > timing.target_beat + miss_beats {
            // Skip notes that are currently being held (check_holds handles those)
            if hold_state.map_or(false, |s| *s == HoldState::Held) {
                continue;
            }

            // Ad-Lib notes silently despawn — no miss penalty
            if adlib.is_some() {
                commands.entity(entity).despawn();
                continue;
            }

            let pos = spline.position_at_progress(playhead.progress(conductor.current_beat));
            let is_hold = matches!(note_type.0, NoteKind::Hold { .. });

            if is_hold {
                // Pending hold that was never pressed: 2 MISSes (head + tail)
                info!("MISS x2 — hold note at beat {:.1} auto-missed", timing.target_beat);
                commands.entity(entity).despawn();
                results.write(JudgmentResult {
                    judgment: Judgment::Miss,
                    position: pos,
                });
                results.write(JudgmentResult {
                    judgment: Judgment::Miss,
                    position: pos,
                });
            } else {
                info!("MISS — note at beat {:.1} auto-missed", timing.target_beat);
                commands.entity(entity).despawn();
                results.write(JudgmentResult {
                    judgment: Judgment::Miss,
                    position: pos,
                });
            }
        }
    }
}

/// Reads JudgmentResult messages and spawns visual feedback entities.
fn spawn_feedback(
    mut commands: Commands,
    mut results: MessageReader<JudgmentResult>,
) {
    for result in results.read() {
        let entity = commands.spawn((
            DespawnOnExit(GameScreen::Playing),
            Transform::from_translation(result.position.extend(2.0)),
            Visibility::default(),
            JudgmentFeedback {
                judgment: result.judgment,
                position: result.position,
                timer: FEEDBACK_LIFETIME,
                max_time: FEEDBACK_LIFETIME,
            },
        )).id();
        spawn_feedback_visual(&mut commands, entity, result.judgment);
    }
}

fn cleanup_feedback(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut JudgmentFeedback)>,
) {
    for (entity, mut fb) in &mut query {
        fb.timer -= time.delta_secs();
        if fb.timer <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
