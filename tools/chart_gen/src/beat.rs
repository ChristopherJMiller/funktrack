use crate::onset::OnsetEvent;
use crate::stft::Spectrogram;

/// Result of beat tracking.
#[derive(Debug, Clone)]
pub struct BeatGrid {
    /// Beat positions in seconds.
    pub beats: Vec<f64>,
    /// Estimated BPM.
    pub bpm: f64,
}

impl BeatGrid {
    /// Convert a time in seconds to a beat position (0-indexed).
    pub fn time_to_beat(&self, time_seconds: f64) -> f64 {
        if self.beats.is_empty() {
            return 0.0;
        }

        // Find which beat interval this time falls in
        if time_seconds <= self.beats[0] {
            // Before first beat — extrapolate backward
            let period = 60.0 / self.bpm;
            return (time_seconds - self.beats[0]) / period;
        }

        for i in 1..self.beats.len() {
            if time_seconds <= self.beats[i] {
                let t0 = self.beats[i - 1];
                let t1 = self.beats[i];
                let frac = (time_seconds - t0) / (t1 - t0);
                return (i - 1) as f64 + frac;
            }
        }

        // After last beat — extrapolate forward
        let last = *self.beats.last().unwrap();
        let period = 60.0 / self.bpm;
        let beats_past = (time_seconds - last) / period;
        (self.beats.len() - 1) as f64 + beats_past
    }

    /// Total duration covered by the beat grid in seconds.
    pub fn duration_seconds(&self) -> f64 {
        if self.beats.is_empty() {
            return 0.0;
        }
        *self.beats.last().unwrap() - self.beats[0]
    }

    /// Total number of beats.
    pub fn total_beats(&self) -> f64 {
        if self.beats.is_empty() {
            return 0.0;
        }
        (self.beats.len() - 1) as f64
    }
}

/// Track beats from onset events and spectrogram.
///
/// If `bpm_override` is Some, skip tempo detection and use the given BPM.
pub fn track_beats(
    spectrogram: &Spectrogram,
    onsets: &[OnsetEvent],
    bpm_override: Option<f64>,
) -> BeatGrid {
    let duration = spectrogram.frames.len() as f64 * spectrogram.hop_size as f64
        / spectrogram.sample_rate as f64;

    // Step 1: Build onset strength envelope (one value per STFT frame)
    let onset_envelope = build_onset_envelope(spectrogram, onsets);

    // Step 2: Estimate tempo
    let bpm = match bpm_override {
        Some(b) => b,
        None => estimate_tempo(&onset_envelope, spectrogram.sample_rate, spectrogram.hop_size),
    };

    // Step 3: Find optimal beat positions via dynamic programming
    let beats = find_beats(&onset_envelope, bpm, spectrogram.sample_rate, spectrogram.hop_size, duration);

    BeatGrid { beats, bpm }
}

/// Build an onset strength envelope: one value per spectrogram frame.
fn build_onset_envelope(spectrogram: &Spectrogram, onsets: &[OnsetEvent]) -> Vec<f32> {
    let num_frames = spectrogram.frames.len();
    let mut envelope = vec![0.0f32; num_frames];

    // Place onset strengths at their frame positions
    for onset in onsets {
        if onset.frame < num_frames {
            envelope[onset.frame] = onset.strength;
        }
    }

    // Smooth with a small Gaussian-like kernel (half-rectified Hann, ~50ms)
    let kernel_size = (0.05 * spectrogram.sample_rate as f64 / spectrogram.hop_size as f64) as usize;
    let kernel_size = kernel_size.max(3) | 1; // Ensure odd
    smooth_envelope(&mut envelope, kernel_size);

    envelope
}

/// Simple moving-average smoothing (in-place).
fn smooth_envelope(data: &mut [f32], kernel_size: usize) {
    let half = kernel_size / 2;
    let original = data.to_vec();

    for i in 0..data.len() {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(data.len());
        let sum: f32 = original[start..end].iter().sum();
        data[i] = sum / (end - start) as f32;
    }
}

/// Estimate tempo via autocorrelation of the onset envelope.
fn estimate_tempo(envelope: &[f32], sample_rate: u32, hop_size: usize) -> f64 {
    let frame_rate = sample_rate as f64 / hop_size as f64;

    // BPM search range: 60-200 BPM
    let min_bpm = 60.0;
    let max_bpm = 200.0;

    // Convert to lag range (in frames)
    let min_lag = (frame_rate * 60.0 / max_bpm) as usize;
    let max_lag = (frame_rate * 60.0 / min_bpm) as usize;
    let max_lag = max_lag.min(envelope.len() / 2);

    if min_lag >= max_lag {
        return 120.0; // Fallback
    }

    // Autocorrelation
    let mut best_lag = min_lag;
    let mut best_score = f64::NEG_INFINITY;

    for lag in min_lag..=max_lag {
        let mut correlation = 0.0f64;
        let n = envelope.len() - lag;
        for i in 0..n {
            correlation += envelope[i] as f64 * envelope[i + lag] as f64;
        }
        correlation /= n as f64;

        // Apply perceptual weighting: Gaussian centered at 120 BPM
        let bpm = frame_rate * 60.0 / lag as f64;
        let weight = (-0.5 * ((bpm - 120.0) / 40.0).powi(2)).exp();
        let score = correlation * weight;

        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }

    let bpm = frame_rate * 60.0 / best_lag as f64;

    // Round to nearest 0.5 BPM for cleaner values
    (bpm * 2.0).round() / 2.0
}

/// Find optimal beat positions using dynamic programming.
///
/// Places beats at the estimated tempo, adjusting positions to align with onsets.
fn find_beats(
    envelope: &[f32],
    bpm: f64,
    sample_rate: u32,
    hop_size: usize,
    duration: f64,
) -> Vec<f64> {
    let frame_rate = sample_rate as f64 / hop_size as f64;
    let period_frames = frame_rate * 60.0 / bpm;
    let period_seconds = 60.0 / bpm;

    // Expected number of beats
    let num_beats = (duration / period_seconds) as usize + 1;
    if num_beats < 2 {
        return vec![0.0];
    }

    // DP: for each beat position, find the best frame within a search window
    let search_radius = (period_frames * 0.25) as usize; // Allow ±25% of beat period

    let mut beats = Vec::with_capacity(num_beats);

    // Find best starting position (first beat)
    let first_search_end = (period_frames * 2.0) as usize;
    let first_search_end = first_search_end.min(envelope.len());
    let mut best_start = 0;
    let mut best_start_score = 0.0f32;

    for i in 0..first_search_end {
        if envelope[i] > best_start_score {
            best_start_score = envelope[i];
            best_start = i;
        }
    }

    beats.push(best_start as f64 / frame_rate);

    // Place subsequent beats
    for beat_idx in 1..num_beats {
        let expected_time = beats[0] + beat_idx as f64 * period_seconds;
        let expected_frame = (expected_time * frame_rate) as usize;

        if expected_frame >= envelope.len() {
            break;
        }

        let search_start = expected_frame.saturating_sub(search_radius);
        let search_end = (expected_frame + search_radius + 1).min(envelope.len());

        let mut best_frame = expected_frame.min(envelope.len() - 1);
        let mut best_score = f64::NEG_INFINITY;

        for frame in search_start..search_end {
            // Score = onset strength - penalty for deviation from expected position
            let onset_score = envelope[frame] as f64;
            let deviation = (frame as f64 - expected_frame as f64).abs() / period_frames;
            let regularity_penalty = deviation * deviation * 2.0; // Quadratic penalty
            let score = onset_score - regularity_penalty;

            if score > best_score {
                best_score = score;
                best_frame = frame;
            }
        }

        beats.push(best_frame as f64 / frame_rate);
    }

    beats
}
