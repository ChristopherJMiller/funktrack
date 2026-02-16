use bevy::prelude::*;

use crate::GameSet;
use crate::conductor::SongConductor;
use crate::path::SplinePath;
use crate::visuals::spawn_note_visual;

pub struct NotesPlugin;

impl Plugin for NotesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_notes.in_set(GameSet::SpawnNotes));
    }
}

// --- Components ---

#[derive(Debug, Clone, Copy)]
pub enum NoteKind {
    Tap,
    Slide(crate::beatmap::SlideDirection),
    Hold { end_beat: f64 },
    AdLib,
    Beat,
    Scratch,
    Critical,
    DualSlide(crate::beatmap::SlideDirection, crate::beatmap::SlideDirection),
}

#[derive(Component)]
pub struct HoldEndBeat(pub f64);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldState {
    Pending,
    Held,
    Completed,
    Dropped,
}

#[derive(Component)]
pub struct NoteType(pub NoteKind);

#[derive(Component)]
pub struct NoteDirection(pub crate::beatmap::SlideDirection);

#[derive(Component)]
pub struct NoteTiming {
    pub target_beat: f64,
}

/// Fixed spline progress for a stationary note (0.0 = start, 1.0 = end of spline).
#[derive(Component)]
pub struct SplineProgress(pub f32);

#[derive(Component)]
pub struct NoteAlive;

/// Marker for Ad-Lib notes: silently despawn on miss (no penalty).
#[derive(Component)]
pub struct AdLibMarker;

/// Tracks rapid tap count for Beat notes (needs 2+ taps to clear).
#[derive(Component)]
pub struct BeatTapCount {
    pub count: u8,
    pub first_tap_ms: f64,
}

/// Stores both directions for a Dual Slide note.
#[derive(Component)]
pub struct DualSlideDirections(pub crate::beatmap::SlideDirection, pub crate::beatmap::SlideDirection);

// --- Resources ---

pub struct ChartNote {
    pub target_beat: f64,
    pub kind: NoteKind,
}

#[derive(Resource)]
pub struct NoteQueue {
    pub notes: Vec<ChartNote>,
    pub next_index: usize,
}

/// Maps song beats to spline progress (0.0→1.0).
/// The playhead rides along the track; camera follows it.
#[derive(Resource)]
pub struct Playhead {
    pub song_start_beat: f64,
    pub song_end_beat: f64,
}

impl Playhead {
    /// Convert a beat to normalized spline progress, clamped 0.0→1.0.
    pub fn progress(&self, beat: f64) -> f32 {
        let range = self.song_end_beat - self.song_start_beat;
        if range <= 0.0 {
            return 0.0;
        }
        ((beat - self.song_start_beat) / range).clamp(0.0, 1.0) as f32
    }
}

// --- Systems ---

/// Spawn window: notes become visible when the playhead is within this
/// fraction of the spline from their position.
const SPAWN_VISIBILITY_RANGE: f32 = 0.25;

fn spawn_notes(
    mut commands: Commands,
    conductor: Option<Res<SongConductor>>,
    queue: Option<ResMut<NoteQueue>>,
    playhead: Option<Res<Playhead>>,
    spline: Option<Res<SplinePath>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(mut queue) = queue else { return };
    let Some(playhead) = playhead else { return };
    let Some(_spline) = spline else { return };

    if !conductor.playing {
        return;
    }

    let current_progress = playhead.progress(conductor.current_beat);

    while queue.next_index < queue.notes.len() {
        let note = &queue.notes[queue.next_index];
        let note_progress = playhead.progress(note.target_beat);

        // Spawn notes that are within the visibility range ahead of the playhead
        if note_progress > current_progress + SPAWN_VISIBILITY_RANGE {
            break;
        }

        let kind = note.kind;
        let entity = commands.spawn((
            NoteType(kind),
            NoteTiming {
                target_beat: note.target_beat,
            },
            SplineProgress(note_progress),
            NoteAlive,
            Transform::default(),
            Visibility::default(),
        )).id();
        match kind {
            NoteKind::Slide(dir) => {
                commands.entity(entity).insert(NoteDirection(dir));
            }
            NoteKind::Hold { end_beat } => {
                commands.entity(entity).insert((HoldEndBeat(end_beat), HoldState::Pending));
            }
            NoteKind::AdLib => {
                commands.entity(entity).insert(AdLibMarker);
            }
            NoteKind::Beat => {
                commands.entity(entity).insert(BeatTapCount { count: 0, first_tap_ms: 0.0 });
            }
            NoteKind::DualSlide(a, b) => {
                commands.entity(entity).insert(DualSlideDirections(a, b));
            }
            _ => {}
        }
        spawn_note_visual(&mut commands, entity, &kind);
        queue.next_index += 1;
    }
}
