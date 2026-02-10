use bevy::prelude::*;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend,
    clock::{ClockHandle, ClockSpeed},
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
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
        });

        app.add_systems(Startup, start_song);
    }
}

pub struct KiraContext {
    pub manager: AudioManager,
    pub clock: Option<ClockHandle>,
    pub sound: Option<StaticSoundHandle>,
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

fn start_song(mut ctx: NonSendMut<KiraContext>) {
    play_song(&mut ctx, "assets/songs/test_120bpm/click.ogg", 120.0);
}
