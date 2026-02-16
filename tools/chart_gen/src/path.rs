use noise::{NoiseFn, Perlin};

use crate::chart::PathSegment;
use crate::stft::Spectrogram;

/// Screen bounds for path generation.
const SCREEN_HALF_WIDTH: f32 = 600.0;
const SCREEN_HALF_HEIGHT: f32 = 360.0; // 720p / 2
const Y_CLAMP: f32 = SCREEN_HALF_HEIGHT * 0.4; // ±40% of screen height

/// Generate an audio-reactive CatmullRom path from spectrogram data.
///
/// Places one control point per beat, with Y driven by sub-band energy + Perlin noise.
pub fn generate_path(
    spectrogram: &Spectrogram,
    total_beats: f64,
    bpm: f64,
) -> PathSegment {
    let num_points = (total_beats.ceil() as usize + 1).max(4);
    let perlin = Perlin::new(42);

    let seconds_per_beat = 60.0 / bpm;

    // X advances linearly across the screen
    let x_start = -SCREEN_HALF_WIDTH;
    let x_end = SCREEN_HALF_WIDTH;
    let x_step = (x_end - x_start) / (num_points - 1) as f32;

    let mut points = Vec::with_capacity(num_points);
    let mut y = 0.0f32;

    for i in 0..num_points {
        let beat = i as f64;
        let time = beat * seconds_per_beat;
        let frame = (time * spectrogram.sample_rate as f64 / spectrogram.hop_size as f64) as usize;

        let x = x_start + i as f32 * x_step;

        if frame < spectrogram.frames.len() {
            // Bass energy (20-250 Hz) → large sweeps
            let bass = spectrogram.band_energy(frame, 20.0, 250.0);
            let bass_sweep = bass * 400.0 - 200.0; // Map to ±200px

            // High frequency (4000-20000 Hz) → oscillations
            let highs = spectrogram.band_energy(frame, 4000.0, 20000.0);
            let osc_phase = (2.0 * std::f64::consts::PI * 2.0 * beat).sin() as f32;
            let high_oscillation = highs * 100.0 * osc_phase; // ±50px effective

            // Perlin noise modulated by RMS energy
            let energy = spectrogram.frame_energy(frame);
            let noise_val = fbm(&perlin, beat * 0.3, 3, 0.5, 2.0) as f32;
            let noise_component = noise_val * energy * 150.0;

            // Combine components
            y += bass_sweep * 0.3 + high_oscillation + noise_component * 0.5;
        }

        // Mean-reversion spring
        y *= 0.97; // -3% per beat

        // Soft sigmoid clamp
        y = soft_clamp(y, Y_CLAMP);

        points.push((x, y));
    }

    PathSegment::CatmullRom {
        points,
        start_beat: 0.0,
        end_beat: total_beats,
    }
}

/// Fractal Brownian Motion (Perlin noise with multiple octaves).
fn fbm(perlin: &Perlin, x: f64, octaves: u32, persistence: f64, lacunarity: f64) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amplitude = 0.0;

    for _ in 0..octaves {
        value += perlin.get([x * frequency, 0.0]) * amplitude;
        max_amplitude += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    value / max_amplitude
}

/// Soft clamp using sigmoid-like function.
fn soft_clamp(value: f32, limit: f32) -> f32 {
    if limit <= 0.0 {
        return 0.0;
    }
    // tanh-based soft clamp: approaches ±limit asymptotically
    (value / limit).tanh() * limit
}
