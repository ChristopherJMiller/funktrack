use crate::stft::Spectrogram;

/// A detected onset event.
#[derive(Debug, Clone)]
pub struct OnsetEvent {
    /// Frame index in the spectrogram.
    pub frame: usize,
    /// Onset strength (spectral flux value, normalized 0-1).
    pub strength: f32,
    /// Time in seconds.
    pub time_seconds: f64,
}

/// Detect onsets using spectral flux with adaptive peak picking.
///
/// - `sensitivity`: threshold multiplier (default 1.5). Higher = fewer onsets.
/// - `min_interval_ms`: minimum time between onsets in milliseconds (default 50).
pub fn detect_onsets(
    spectrogram: &Spectrogram,
    sensitivity: f64,
    min_interval_ms: f64,
) -> Vec<OnsetEvent> {
    let num_frames = spectrogram.frames.len();
    if num_frames < 2 {
        return Vec::new();
    }

    // Stage 1: Compute spectral flux
    let mut flux: Vec<f32> = Vec::with_capacity(num_frames);
    flux.push(0.0); // First frame has no predecessor

    for i in 1..num_frames {
        let prev = &spectrogram.frames[i - 1];
        let curr = &spectrogram.frames[i];
        let sf: f32 = curr
            .iter()
            .zip(prev.iter())
            .map(|(c, p)| (c - p).max(0.0))
            .sum();
        flux.push(sf);
    }

    // Normalize flux to 0-1
    let max_flux = flux.iter().cloned().fold(0.0f32, f32::max);
    if max_flux > 0.0 {
        for v in &mut flux {
            *v /= max_flux;
        }
    }

    // Stage 2: Adaptive peak picking
    // Moving average window (~0.5 seconds)
    let avg_window = (0.5 * spectrogram.sample_rate as f64 / spectrogram.hop_size as f64) as usize;
    let avg_window = avg_window.max(3);

    let min_interval_frames =
        (min_interval_ms / 1000.0 * spectrogram.sample_rate as f64 / spectrogram.hop_size as f64)
            as usize;
    let min_interval_frames = min_interval_frames.max(1);

    // Silence gate: -74 dB ≈ 10^(-74/20) ≈ 0.0002
    let silence_threshold = 0.0002f32;

    let mut onsets = Vec::new();
    let mut last_onset_frame: Option<usize> = None;

    for i in 1..num_frames {
        // Compute local mean and stddev
        let window_start = i.saturating_sub(avg_window / 2);
        let window_end = (i + avg_window / 2 + 1).min(num_frames);
        let window_slice = &flux[window_start..window_end];

        let n = window_slice.len() as f64;
        let mean = window_slice.iter().map(|&v| v as f64).sum::<f64>() / n;
        let variance =
            window_slice.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        let threshold = mean + sensitivity * std_dev;

        // Check if this frame is a peak above threshold
        if (flux[i] as f64) < threshold {
            continue;
        }

        // Must be a local maximum
        if i > 0 && flux[i] <= flux[i - 1] {
            continue;
        }
        if i + 1 < num_frames && flux[i] < flux[i + 1] {
            continue;
        }

        // Silence gate: check frame energy
        let energy = spectrogram.frame_energy(i);
        if energy < silence_threshold {
            continue;
        }

        // Minimum inter-onset interval
        if let Some(last) = last_onset_frame {
            if i - last < min_interval_frames {
                continue;
            }
        }

        let time = spectrogram.frame_to_seconds(i);
        onsets.push(OnsetEvent {
            frame: i,
            strength: flux[i],
            time_seconds: time,
        });
        last_onset_frame = Some(i);
    }

    onsets
}
