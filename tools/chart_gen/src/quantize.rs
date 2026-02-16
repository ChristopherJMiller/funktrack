use crate::beat::BeatGrid;
use crate::onset::OnsetEvent;

/// A note quantized to the beat grid.
#[derive(Debug, Clone)]
pub struct QuantizedNote {
    /// Beat position (quantized to grid).
    pub beat: f64,
    /// Original onset strength (normalized 0-1).
    pub strength: f32,
    /// Original time in seconds (before quantization).
    pub original_time: f64,
}

/// Quantize detected onsets to a beat grid at the given resolution.
///
/// `grid_resolution`: subdivisions per beat (1 = whole, 2 = half, 4 = quarter, 8 = eighth).
pub fn quantize_onsets(
    onsets: &[OnsetEvent],
    beat_grid: &BeatGrid,
    grid_resolution: u32,
) -> Vec<QuantizedNote> {
    let grid_step = 1.0 / grid_resolution as f64;

    let mut notes: Vec<QuantizedNote> = Vec::new();

    for onset in onsets {
        let raw_beat = beat_grid.time_to_beat(onset.time_seconds);

        // Skip onsets before beat 0
        if raw_beat < 0.0 {
            continue;
        }

        // Snap to nearest grid position
        let snapped = (raw_beat / grid_step).round() * grid_step;

        // Round to avoid floating point drift
        let snapped = (snapped * 10000.0).round() / 10000.0;

        notes.push(QuantizedNote {
            beat: snapped,
            strength: onset.strength,
            original_time: onset.time_seconds,
        });
    }

    // Deduplicate: if two onsets land on the same grid position, keep the stronger one
    notes.sort_by(|a, b| a.beat.partial_cmp(&b.beat).unwrap());
    notes.dedup_by(|b, a| {
        if (a.beat - b.beat).abs() < 1e-6 {
            // Keep the one with higher strength (stored in `a` after dedup)
            if b.strength > a.strength {
                a.strength = b.strength;
                a.original_time = b.original_time;
            }
            true
        } else {
            false
        }
    });

    notes
}
