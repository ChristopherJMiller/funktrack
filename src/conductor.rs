use std::collections::VecDeque;

use bevy::prelude::*;

use crate::GameSet;
use crate::audio::KiraContext;
use crate::config::GameSettings;

pub struct ConductorPlugin;

impl Plugin for ConductorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_conductor.in_set(GameSet::UpdateConductor));
    }
}

#[derive(Debug, Clone)]
pub struct TimingPoint {
    pub beat: f64,
    pub bpm: f64,
}

const MAX_SAMPLES: usize = 15;
const DRIFT_THRESHOLD_SECS: f64 = 0.050;
const DRIFT_FRAME_LIMIT: u32 = 3;

#[derive(Resource)]
pub struct SongConductor {
    pub current_beat: f64,
    pub bpm: f64,
    pub playing: bool,
    time_samples: VecDeque<(f64, f64)>,
    slope: f64,
    intercept: f64,
    pub timing_points: Vec<TimingPoint>,
    drift_frames: u32,
}

impl SongConductor {
    pub fn new(bpm: f64) -> Self {
        Self {
            current_beat: 0.0,
            bpm,
            playing: false,
            time_samples: VecDeque::with_capacity(MAX_SAMPLES),
            slope: bpm / 60.0,
            intercept: 0.0,
            timing_points: Vec::new(),
            drift_frames: 0,
        }
    }
}

fn clock_time_to_beats(clock: &kira::clock::ClockHandle) -> f64 {
    let t = clock.time();
    t.ticks as f64 + t.fraction
}

fn linear_regression(samples: &VecDeque<(f64, f64)>) -> (f64, f64) {
    let n = samples.len() as f64;
    if n < 2.0 {
        if let Some(&(x, y)) = samples.back() {
            return if x.abs() < f64::EPSILON {
                (0.0, y)
            } else {
                (y / x, 0.0)
            };
        }
        return (0.0, 0.0);
    }

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xx = 0.0;
    let mut sum_xy = 0.0;

    for &(x, y) in samples {
        sum_x += x;
        sum_y += y;
        sum_xx += x * x;
        sum_xy += x * y;
    }

    let mean_x = sum_x / n;
    let mean_y = sum_y / n;
    let variance = sum_xx / n - mean_x * mean_x;

    if variance.abs() < f64::EPSILON {
        return (0.0, mean_y);
    }

    let covariance = sum_xy / n - mean_x * mean_y;
    let slope = covariance / variance;
    let intercept = mean_y - slope * mean_x;

    (slope, intercept)
}

fn update_conductor(
    time: Res<Time<Real>>,
    ctx: NonSend<KiraContext>,
    conductor: Option<ResMut<SongConductor>>,
    settings: Option<Res<GameSettings>>,
) {
    let Some(mut conductor) = conductor else { return };
    let Some(ref clock) = ctx.clock else {
        return;
    };

    conductor.playing = true;

    let game_time = time.elapsed_secs_f64();
    // Apply audio offset: positive offset means audio is late, so shift beats forward
    let offset_beats = if let Some(ref settings) = settings {
        settings.audio_offset_ms as f64 * conductor.bpm / 60_000.0
    } else {
        0.0
    };
    let audio_beats = clock_time_to_beats(clock) + offset_beats;

    // Push sample into rolling window.
    if conductor.time_samples.len() >= MAX_SAMPLES {
        conductor.time_samples.pop_front();
    }
    conductor.time_samples.push_back((game_time, audio_beats));

    // Compute linear regression.
    let (slope, intercept) = linear_regression(&conductor.time_samples);
    conductor.slope = slope;
    conductor.intercept = intercept;

    let predicted_beat = slope * game_time + intercept;

    // Drift check: compare predicted vs raw audio beats.
    let drift_beats = (predicted_beat - audio_beats).abs();
    let drift_secs = drift_beats / (conductor.bpm / 60.0);

    if drift_secs > DRIFT_THRESHOLD_SECS {
        conductor.drift_frames += 1;
        if conductor.drift_frames >= DRIFT_FRAME_LIMIT {
            warn!(
                "Audio drift {drift_secs:.3}s exceeded threshold for {} frames, hard resyncing",
                conductor.drift_frames
            );
            conductor.time_samples.clear();
            conductor.time_samples.push_back((game_time, audio_beats));
            conductor.slope = conductor.bpm / 60.0;
            conductor.intercept = audio_beats - conductor.slope * game_time;
            conductor.drift_frames = 0;
            conductor.current_beat = audio_beats;
            return;
        }
    } else {
        conductor.drift_frames = 0;
    }

    // Monotonicity guarantee.
    conductor.current_beat = predicted_beat.max(conductor.current_beat);

    // Sanity check slope against expected bpm/60.
    let expected_slope = conductor.bpm / 60.0;
    if conductor.time_samples.len() >= 5 {
        let deviation = ((slope - expected_slope) / expected_slope).abs();
        if deviation > 0.10 {
            warn!(
                "Conductor slope {slope:.4} deviates {:.1}% from expected {expected_slope:.4}",
                deviation * 100.0
            );
        }
    }

    // Advance timing points if we crossed a BPM change boundary.
    if conductor
        .timing_points
        .first()
        .is_some_and(|tp| conductor.current_beat >= tp.beat)
    {
        let tp = conductor.timing_points.remove(0);
        conductor.bpm = tp.bpm;
        conductor.time_samples.clear();
        conductor.time_samples.push_back((game_time, audio_beats));
        conductor.slope = conductor.bpm / 60.0;
        conductor.intercept = audio_beats - conductor.slope * game_time;
        info!("BPM changed to {}", conductor.bpm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regression_two_points() {
        let mut samples = VecDeque::new();
        samples.push_back((0.0, 0.0));
        samples.push_back((1.0, 2.0));
        let (slope, intercept) = linear_regression(&samples);
        assert!((slope - 2.0).abs() < 1e-10);
        assert!(intercept.abs() < 1e-10);
    }

    #[test]
    fn regression_perfect_line() {
        let mut samples = VecDeque::new();
        // y = 2x + 1
        for i in 0..10 {
            let x = i as f64 * 0.5;
            samples.push_back((x, 2.0 * x + 1.0));
        }
        let (slope, intercept) = linear_regression(&samples);
        assert!((slope - 2.0).abs() < 1e-10);
        assert!((intercept - 1.0).abs() < 1e-10);
    }

    #[test]
    fn regression_single_sample() {
        let mut samples = VecDeque::new();
        samples.push_back((1.0, 2.0));
        let (slope, intercept) = linear_regression(&samples);
        // With single sample at (1, 2): slope = 2/1 = 2, intercept = 0
        assert!((slope - 2.0).abs() < 1e-10);
        assert!(intercept.abs() < 1e-10);
    }
}
