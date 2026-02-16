use bevy::prelude::*;

use crate::GameSet;
use crate::beatmap::SlideDirection;
use crate::conductor::SongConductor;
use crate::visuals::spawn_note_visual;

pub struct NotesPlugin;

impl Plugin for NotesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_notes.in_set(GameSet::SpawnNotes))
            .add_systems(Update, move_notes.in_set(GameSet::MoveNotes));
    }
}

// --- Components ---

#[derive(Debug, Clone, Copy)]
pub enum NoteKind {
    Tap,
    Slide(SlideDirection),
    Hold { end_beat: f64 },
    AdLib,
    Beat,
    Scratch,
    Critical,
    DualSlide(SlideDirection, SlideDirection),
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
pub struct NoteDirection(pub SlideDirection);

#[derive(Component)]
pub struct NoteTiming {
    pub target_beat: f64,
    pub spawn_beat: f64,
    pub travel_beats: f64,
}

#[derive(Component)]
pub struct NoteProgress(pub f32);

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
pub struct DualSlideDirections(pub SlideDirection, pub SlideDirection);

// --- Resources ---

pub struct ChartNote {
    pub target_beat: f64,
    pub kind: NoteKind,
}

#[derive(Resource)]
pub struct NoteQueue {
    pub notes: Vec<ChartNote>,
    pub next_index: usize,
    pub look_ahead_beats: f64,
    pub travel_beats: f64,
}

// --- Systems ---

fn spawn_notes(
    mut commands: Commands,
    conductor: Option<Res<SongConductor>>,
    queue: Option<ResMut<NoteQueue>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(mut queue) = queue else { return };

    if !conductor.playing {
        return;
    }

    let horizon = conductor.current_beat + queue.look_ahead_beats;

    while queue.next_index < queue.notes.len() {
        let note = &queue.notes[queue.next_index];
        let spawn_beat = note.target_beat - queue.travel_beats;
        if spawn_beat > horizon {
            break;
        }
        let kind = note.kind;
        let entity = commands.spawn((
            NoteType(kind),
            NoteTiming {
                target_beat: note.target_beat,
                spawn_beat,
                travel_beats: queue.travel_beats,
            },
            NoteProgress(0.0),
            NoteAlive,
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

fn move_notes(
    conductor: Option<Res<SongConductor>>,
    mut query: Query<(&NoteTiming, &mut NoteProgress)>,
) {
    let Some(conductor) = conductor else { return };

    for (timing, mut progress) in &mut query {
        let p = (conductor.current_beat - timing.spawn_beat) / timing.travel_beats;
        progress.0 = p.max(0.0) as f32;
    }
}

