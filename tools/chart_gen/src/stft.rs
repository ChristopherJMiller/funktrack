use realfft::RealFftPlanner;

/// Parameters for STFT computation.
pub const WINDOW_SIZE: usize = 2048;
pub const HOP_SIZE: usize = 512;

/// Result of STFT: a sequence of magnitude spectra.
pub struct Spectrogram {
    /// Each inner Vec has `WINDOW_SIZE / 2 + 1` magnitude values.
    pub frames: Vec<Vec<f32>>,
    /// Hop size in samples (for converting frame index to time).
    pub hop_size: usize,
    /// Sample rate of the input audio.
    pub sample_rate: u32,
}

impl Spectrogram {
    /// Convert a frame index to time in seconds.
    pub fn frame_to_seconds(&self, frame: usize) -> f64 {
        frame as f64 * self.hop_size as f64 / self.sample_rate as f64
    }

    /// Number of frequency bins per frame.
    pub fn num_bins(&self) -> usize {
        WINDOW_SIZE / 2 + 1
    }

    /// Frequency in Hz for a given bin index.
    pub fn bin_to_hz(&self, bin: usize) -> f64 {
        bin as f64 * self.sample_rate as f64 / WINDOW_SIZE as f64
    }

    /// Get the bin index for a given frequency (rounded down).
    pub fn hz_to_bin(&self, hz: f64) -> usize {
        (hz * WINDOW_SIZE as f64 / self.sample_rate as f64) as usize
    }

    /// Compute RMS energy in a frequency band for a given frame.
    pub fn band_energy(&self, frame: usize, low_hz: f64, high_hz: f64) -> f32 {
        let lo = self.hz_to_bin(low_hz).max(1);
        let hi = self.hz_to_bin(high_hz).min(self.num_bins() - 1);
        if lo >= hi {
            return 0.0;
        }
        let magnitudes = &self.frames[frame];
        let sum: f32 = magnitudes[lo..=hi].iter().map(|m| m * m).sum();
        (sum / (hi - lo + 1) as f32).sqrt()
    }

    /// Compute total RMS energy for a given frame.
    pub fn frame_energy(&self, frame: usize) -> f32 {
        let magnitudes = &self.frames[frame];
        let sum: f32 = magnitudes.iter().map(|m| m * m).sum();
        (sum / magnitudes.len() as f32).sqrt()
    }
}

/// Compute the STFT of mono audio samples.
///
/// Uses a Hann window with 2048-sample frames and 512-sample hop.
/// Returns magnitude spectra (not complex) for each frame.
pub fn compute_stft(samples: &[f32], sample_rate: u32) -> Spectrogram {
    let num_bins = WINDOW_SIZE / 2 + 1;

    // Pre-compute Hann window
    let window: Vec<f32> = (0..WINDOW_SIZE)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (WINDOW_SIZE - 1) as f32).cos())
        })
        .collect();

    // Set up FFT
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);
    let mut scratch = fft.make_scratch_vec();

    let mut frames = Vec::new();
    let mut pos = 0;

    while pos + WINDOW_SIZE <= samples.len() {
        // Apply window
        let mut input: Vec<f32> = samples[pos..pos + WINDOW_SIZE]
            .iter()
            .zip(window.iter())
            .map(|(s, w)| s * w)
            .collect();

        // FFT
        let mut spectrum = fft.make_output_vec();
        fft.process_with_scratch(&mut input, &mut spectrum, &mut scratch)
            .expect("FFT processing failed");

        // Convert to magnitudes
        let magnitudes: Vec<f32> = spectrum[..num_bins]
            .iter()
            .map(|c| (c.re * c.re + c.im * c.im).sqrt())
            .collect();

        frames.push(magnitudes);
        pos += HOP_SIZE;
    }

    Spectrogram {
        frames,
        hop_size: HOP_SIZE,
        sample_rate,
    }
}
