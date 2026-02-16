use std::time::Duration;

use bevy::prelude::*;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
    clock::{ClockHandle, ClockSpeed},
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
};

pub struct KiraPlugin;

impl Plugin for KiraPlugin {
    fn build(&self, app: &mut App) {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .expect("failed to create Kira AudioManager");

        app.insert_non_send_resource(KiraContext {
            manager,
            clock: None,
            sound: None,
            preview: None,
        });
    }
}

pub struct KiraContext {
    pub manager: AudioManager,
    pub clock: Option<ClockHandle>,
    pub sound: Option<StaticSoundHandle>,
    pub preview: Option<StaticSoundHandle>,
}

/// Convert a 0.0–1.0 amplitude to decibels (f32).
fn amplitude_to_db(amp: f64) -> f32 {
    if amp <= 0.0 {
        -60.0 // silence
    } else {
        (20.0 * (amp as f32).log10()).max(-60.0)
    }
}

pub fn play_song(ctx: &mut KiraContext, path: &str, bpm: f64) {
    let mut clock = ctx
        .manager
        .add_clock(ClockSpeed::TicksPerMinute(bpm))
        .expect("failed to create clock");
    clock.start();

    let sound_data = StaticSoundData::from_file(path)
        .expect("failed to load audio file")
        .start_time(clock.time());

    let sound = ctx
        .manager
        .play(sound_data)
        .expect("failed to play sound");

    ctx.clock = Some(clock);
    ctx.sound = Some(sound);
}

pub fn stop_song(ctx: &mut KiraContext) {
    if let Some(ref mut sound) = ctx.sound {
        let _ = sound.stop(Default::default());
    }
    ctx.sound = None;
    ctx.clock = None;
}

pub fn play_preview(ctx: &mut KiraContext, path: &str, start_ms: u64, duration_ms: u64, volume: f64) {
    stop_preview(ctx);

    let Ok(sound_data) = StaticSoundData::from_file(path) else {
        warn!("Failed to load preview audio: {}", path);
        return;
    };

    let start_secs = start_ms as f64 / 1000.0;
    let end_secs = start_secs + duration_ms as f64 / 1000.0;
    let db = amplitude_to_db(volume);

    let settings = StaticSoundSettings::new()
        .start_position(start_secs)
        .loop_region(start_secs..end_secs)
        .volume(db)
        .fade_in_tween(Tween {
            duration: Duration::from_millis(500),
            ..default()
        });

    let sound_data = sound_data.with_settings(settings);

    match ctx.manager.play(sound_data) {
        Ok(handle) => {
            ctx.preview = Some(handle);
        }
        Err(e) => {
            warn!("Failed to play preview: {}", e);
        }
    }
}

pub fn stop_preview(ctx: &mut KiraContext) {
    if let Some(ref mut preview) = ctx.preview {
        let _ = preview.stop(Tween {
            duration: Duration::from_millis(300),
            ..default()
        });
    }
    ctx.preview = None;
}

pub fn set_song_volume(ctx: &mut KiraContext, amplitude: f64) {
    // StaticSoundHandle in Kira 0.11 doesn't expose set_volume.
    // Volume is set at construction time or via tracks. This is a no-op for now.
    let _ = (ctx, amplitude);
}
