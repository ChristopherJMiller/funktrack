use bevy::prelude::*;

use crate::GameSet;
use crate::conductor::SongConductor;
use crate::path::SplinePath;

pub struct NotesPlugin;

impl Plugin for NotesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_note_queue)
            .add_systems(Update, spawn_notes.in_set(GameSet::SpawnNotes))
            .add_systems(Update, move_notes.in_set(GameSet::MoveNotes))
            .add_systems(
                Update,
                (render_notes, despawn_completed_notes).in_set(GameSet::Render),
            );
    }
}

// --- Components ---

#[derive(Debug, Clone, Copy)]
pub enum NoteKind {
    Tap,
}

#[derive(Component)]
pub struct NoteType(pub NoteKind);

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

// --- Resources ---

struct ChartNote {
    target_beat: f64,
    kind: NoteKind,
}

#[derive(Resource)]
pub struct NoteQueue {
    notes: Vec<ChartNote>,
    next_index: usize,
    look_ahead_beats: f64,
    travel_beats: f64,
}

// --- Systems ---

fn setup_note_queue(mut commands: Commands) {
    let mut notes = Vec::with_capacity(40);
    for i in 0..40 {
        notes.push(ChartNote {
            target_beat: 4.0 + i as f64,
            kind: NoteKind::Tap,
        });
    }
    commands.insert_resource(NoteQueue {
        notes,
        next_index: 0,
        look_ahead_beats: 3.0,
        travel_beats: 3.0,
    });
}

fn spawn_notes(
    mut commands: Commands,
    conductor: Res<SongConductor>,
    mut queue: ResMut<NoteQueue>,
) {
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
        commands.spawn((
            NoteType(note.kind),
            NoteTiming {
                target_beat: note.target_beat,
                spawn_beat,
                travel_beats: queue.travel_beats,
            },
            NoteProgress(0.0),
            NoteAlive,
        ));
        queue.next_index += 1;
    }
}

fn move_notes(conductor: Res<SongConductor>, mut query: Query<(&NoteTiming, &mut NoteProgress)>) {
    for (timing, mut progress) in &mut query {
        let p = (conductor.current_beat - timing.spawn_beat) / timing.travel_beats;
        progress.0 = p.clamp(0.0, 1.0) as f32;
    }
}

fn render_notes(
    query: Query<&NoteProgress, With<NoteAlive>>,
    spline: Res<SplinePath>,
    mut gizmos: Gizmos,
) {
    let note_color = Color::srgb(1.0, 0.4, 0.7);
    let tangent_color = Color::srgb(1.0, 0.8, 0.3);

    for progress in &query {
        let pos = spline.position_at_progress(progress.0);
        let tangent = spline.tangent_at_progress(progress.0).normalize_or_zero();

        gizmos.circle_2d(pos, 12.0, note_color);
        gizmos.line_2d(pos, pos + tangent * 20.0, tangent_color);
    }
}

fn despawn_completed_notes(
    mut commands: Commands,
    query: Query<(Entity, &NoteProgress), With<NoteAlive>>,
) {
    for (entity, progress) in &query {
        if progress.0 >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}
