use crate::chart::{ChartNoteEntry, ChartNoteType, Difficulty, SlideDirection};
use crate::difficulty::ScoredNote;

/// Assign note types to scored notes based on difficulty and simple heuristics.
///
/// Phase 7 uses basic rules. Phase 10 will add audio-feature-based assignment.
pub fn assign_note_types(
    notes: &[ScoredNote],
    difficulty: Difficulty,
    bpm: f64,
) -> Vec<ChartNoteEntry> {
    let mut entries = Vec::with_capacity(notes.len());
    let mut rng_state: u64 = 42; // Deterministic pseudo-random

    for (i, note) in notes.iter().enumerate() {
        let note_type = pick_note_type(notes, i, difficulty, bpm, &mut rng_state);
        entries.push(ChartNoteEntry {
            beat: note.beat,
            note_type,
        });
    }

    entries
}

fn pick_note_type(
    notes: &[ScoredNote],
    idx: usize,
    difficulty: Difficulty,
    _bpm: f64,
    rng: &mut u64,
) -> ChartNoteType {
    let note = &notes[idx];

    match difficulty {
        Difficulty::Easy => {
            // Easy: mostly taps, occasional holds on strong beats
            if note.strength > 0.8 && is_downbeat(note.beat) {
                // Strong downbeats get holds
                let duration = if idx + 1 < notes.len() {
                    let gap = notes[idx + 1].beat - note.beat;
                    (gap * 0.75).min(2.0).max(0.5) // Hold for 75% of gap, max 2 beats
                } else {
                    1.0
                };
                ChartNoteType::Hold {
                    duration_beats: duration,
                }
            } else {
                ChartNoteType::Tap
            }
        }

        Difficulty::Normal => {
            let roll = pseudo_random(rng) % 100;

            if note.strength > 0.8 && is_downbeat(note.beat) {
                // Strong downbeats: hold
                let duration = hold_duration(notes, idx);
                ChartNoteType::Hold {
                    duration_beats: duration,
                }
            } else if roll < 15 {
                // 15% slides
                let dir = pick_slide_direction(note.beat, rng);
                ChartNoteType::Slide { direction: dir }
            } else {
                ChartNoteType::Tap
            }
        }

        Difficulty::Hard => {
            let roll = pseudo_random(rng) % 100;

            if note.strength > 0.85 && is_downbeat(note.beat) {
                // Strong downbeats: hold or critical
                if roll < 40 {
                    ChartNoteType::Critical
                } else {
                    let duration = hold_duration(notes, idx);
                    ChartNoteType::Hold {
                        duration_beats: duration,
                    }
                }
            } else if is_rapid_pair(notes, idx) {
                ChartNoteType::Tap
            } else if roll < 25 {
                let dir = pick_slide_direction(note.beat, rng);
                ChartNoteType::Slide { direction: dir }
            } else {
                ChartNoteType::Tap
            }
        }

        Difficulty::Expert => {
            let roll = pseudo_random(rng) % 100;

            if note.strength > 0.9 && is_downbeat(note.beat) {
                // Strongest downbeats: critical or hold
                if roll < 30 {
                    ChartNoteType::Critical
                } else {
                    let duration = hold_duration(notes, idx);
                    ChartNoteType::Hold {
                        duration_beats: duration,
                    }
                }
            } else if is_rapid_pair(notes, idx) {
                ChartNoteType::Tap
            } else if roll < 20 {
                let dir = pick_slide_direction(note.beat, rng);
                ChartNoteType::Slide { direction: dir }
            } else if roll < 28 {
                let dir = pick_slide_direction(note.beat, rng);
                ChartNoteType::Slide { direction: dir }
            } else if roll < 33 {
                ChartNoteType::Critical
            } else if roll < 38 && note.strength < 0.3 {
                // Low-strength off-beat notes become rests
                ChartNoteType::Rest
            } else {
                ChartNoteType::Tap
            }
        }
    }
}

fn is_downbeat(beat: f64) -> bool {
    let frac = beat - beat.floor();
    frac < 0.01 || frac > 0.99
}

fn is_rapid_pair(notes: &[ScoredNote], idx: usize) -> bool {
    if idx + 1 >= notes.len() {
        return false;
    }
    let gap = notes[idx + 1].beat - notes[idx].beat;
    gap <= 0.25 + 0.01 // 16th note or closer
}

fn hold_duration(notes: &[ScoredNote], idx: usize) -> f64 {
    if idx + 1 < notes.len() {
        let gap = notes[idx + 1].beat - notes[idx].beat;
        (gap * 0.75).min(2.0).max(0.5)
    } else {
        1.0
    }
}

fn pick_slide_direction(seed: f64, rng: &mut u64) -> SlideDirection {
    let combined = (seed * 1000.0) as u64 ^ pseudo_random(rng);
    SlideDirection::ALL[(combined % 8) as usize]
}

/// Simple xorshift64 PRNG for deterministic note type assignment.
fn pseudo_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
