use bevy::prelude::*;

use crate::GameSet;
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
    pub spawn_time: f32,
    pub travel_duration: f32,
}

#[derive(Component)]
pub struct NoteProgress(pub f32);

#[derive(Component)]
pub struct NoteAlive;

// --- Resources ---

struct QueuedNote {
    spawn_time: f32,
    travel_duration: f32,
    kind: NoteKind,
}

#[derive(Resource)]
pub struct NoteSpawnState {
    queue: Vec<QueuedNote>,
    next_index: usize,
    elapsed: f32,
}

// --- Systems ---

fn setup_note_queue(mut commands: Commands) {
    let mut queue = Vec::with_capacity(20);
    for i in 0..20 {
        queue.push(QueuedNote {
            spawn_time: i as f32 * 0.5,
            travel_duration: 3.0,
            kind: NoteKind::Tap,
        });
    }
    commands.insert_resource(NoteSpawnState {
        queue,
        next_index: 0,
        elapsed: 0.0,
    });
}

fn spawn_notes(mut commands: Commands, time: Res<Time>, mut state: ResMut<NoteSpawnState>) {
    state.elapsed += time.delta_secs();

    while state.next_index < state.queue.len() {
        let note = &state.queue[state.next_index];
        if state.elapsed < note.spawn_time {
            break;
        }
        commands.spawn((
            NoteType(note.kind),
            NoteTiming {
                spawn_time: note.spawn_time,
                travel_duration: note.travel_duration,
            },
            NoteProgress(0.0),
            NoteAlive,
        ));
        state.next_index += 1;
    }
}

fn move_notes(state: Res<NoteSpawnState>, mut query: Query<(&NoteTiming, &mut NoteProgress)>) {
    for (timing, mut progress) in &mut query {
        let p = (state.elapsed - timing.spawn_time) / timing.travel_duration;
        progress.0 = p.clamp(0.0, 1.0);
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
