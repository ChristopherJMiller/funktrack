use crate::chart::Difficulty;
use crate::quantize::QuantizedNote;

/// A note scored by importance for difficulty filtering.
#[derive(Debug, Clone)]
pub struct ScoredNote {
    pub beat: f64,
    pub strength: f32,
    pub importance: f64,
}

/// Filter quantized notes by difficulty, keeping only the most important ones.
///
/// Returns notes sorted by beat position, filtered according to:
/// - Onset strength weighting
/// - Beat position weighting (downbeats > beats > off-beats > subdivisions)
/// - Percentile thresholding per difficulty
pub fn filter_by_difficulty(
    notes: &[QuantizedNote],
    difficulty: Difficulty,
    bpm: f64,
) -> Vec<ScoredNote> {
    if notes.is_empty() {
        return Vec::new();
    }

    // Score each note by importance
    let mut scored: Vec<ScoredNote> = notes
        .iter()
        .map(|n| {
            let beat_weight = compute_beat_weight(n.beat);
            let importance = n.strength as f64 * beat_weight;
            ScoredNote {
                beat: n.beat,
                strength: n.strength,
                importance,
            }
        })
        .collect();

    // Determine percentile threshold
    let percentile = difficulty.importance_percentile();

    if percentile > 0.0 {
        let mut importances: Vec<f64> = scored.iter().map(|n| n.importance).collect();
        importances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let threshold_idx = (importances.len() as f64 * percentile) as usize;
        let threshold = importances.get(threshold_idx).copied().unwrap_or(0.0);

        scored.retain(|n| n.importance >= threshold);
    }

    // Post-filter rules
    apply_post_filter_rules(&mut scored, difficulty, bpm);

    scored
}

/// Weight by beat position: downbeats are most important.
fn compute_beat_weight(beat: f64) -> f64 {
    let frac = beat - beat.floor();
    let tolerance = 0.01;

    if frac < tolerance || frac > 1.0 - tolerance {
        // On a whole beat
        let whole = beat.round() as u32;
        if whole % 4 == 0 {
            1.0 // Downbeat (start of measure in 4/4)
        } else {
            0.8 // Other beats
        }
    } else if (frac - 0.5).abs() < tolerance {
        0.5 // Off-beat (half-beat)
    } else {
        0.3 // Subdivision
    }
}

/// Apply post-filter rules to ensure chart quality.
fn apply_post_filter_rules(notes: &mut Vec<ScoredNote>, difficulty: Difficulty, bpm: f64) {
    if notes.is_empty() {
        return;
    }

    // Minimum inter-note interval (in beats)
    let min_interval = match difficulty {
        Difficulty::Easy => 1.0,
        Difficulty::Normal => 0.5,
        Difficulty::Hard => 0.25,
        Difficulty::Expert => 0.125,
    };

    // Remove notes that are too close together (keep the stronger one)
    let mut i = 1;
    while i < notes.len() {
        let gap = notes[i].beat - notes[i - 1].beat;
        if gap < min_interval - 0.001 {
            // Remove the weaker note
            if notes[i].importance < notes[i - 1].importance {
                notes.remove(i);
            } else {
                notes.remove(i - 1);
            }
        } else {
            i += 1;
        }
    }

    // Ensure at least one note per 4-beat measure
    if notes.len() >= 2 {
        let first_beat = notes.first().unwrap().beat;
        let last_beat = notes.last().unwrap().beat;
        let total_measures = ((last_beat - first_beat) / 4.0).ceil() as usize;

        for measure in 0..total_measures {
            let measure_start = first_beat + measure as f64 * 4.0;
            let measure_end = measure_start + 4.0;

            let has_note = notes.iter().any(|n| n.beat >= measure_start && n.beat < measure_end);

            if !has_note {
                // Insert a note on the downbeat of this measure
                let insert_beat = measure_start;
                let pos = notes.partition_point(|n| n.beat < insert_beat);
                notes.insert(pos, ScoredNote {
                    beat: insert_beat,
                    strength: 0.5,
                    importance: 0.5,
                });
            }
        }
    }

    // Compute difficulty rating based on notes per second
    let _ = compute_difficulty_rating(notes, bpm);
}

/// Compute a 1-10 difficulty rating based on note density.
pub fn compute_difficulty_rating(notes: &[ScoredNote], bpm: f64) -> u32 {
    if notes.is_empty() {
        return 1;
    }

    let first_beat = notes.first().unwrap().beat;
    let last_beat = notes.last().unwrap().beat;
    let duration_beats = last_beat - first_beat;

    if duration_beats <= 0.0 {
        return 1;
    }

    let duration_seconds = duration_beats * 60.0 / bpm;
    let nps = notes.len() as f64 / duration_seconds; // notes per second

    // Map NPS to 1-10 rating
    // ~0.5 NPS = 1, ~1 NPS = 2, ~2 NPS = 4, ~4 NPS = 6, ~8 NPS = 8, ~12+ NPS = 10
    let rating = (nps * 1.5).log2().max(0.0) * 2.5 + 1.0;
    (rating.round() as u32).clamp(1, 10)
}
