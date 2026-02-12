use bevy::prelude::*;

use crate::GameSet;
use crate::beatmap::SlideDirection;
use crate::conductor::SongConductor;
use crate::path::SplinePath;

pub struct NotesPlugin;

impl Plugin for NotesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_notes.in_set(GameSet::SpawnNotes))
            .add_systems(Update, move_notes.in_set(GameSet::MoveNotes))
            .add_systems(Update, render_notes.in_set(GameSet::Render));
    }
}

// --- Components ---

#[derive(Debug, Clone, Copy)]
pub enum NoteKind {
    Tap,
    Slide(SlideDirection),
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
        let entity = commands.spawn((
            NoteType(note.kind),
            NoteTiming {
                target_beat: note.target_beat,
                spawn_beat,
                travel_beats: queue.travel_beats,
            },
            NoteProgress(0.0),
            NoteAlive,
        )).id();
        if let NoteKind::Slide(dir) = note.kind {
            commands.entity(entity).insert(NoteDirection(dir));
        }
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

fn render_notes(
    query: Query<(&NoteProgress, &NoteType, Option<&NoteDirection>), With<NoteAlive>>,
    spline: Option<Res<SplinePath>>,
    mut gizmos: Gizmos,
) {
    let Some(spline) = spline else { return };

    let tap_color = Color::srgb(1.0, 0.4, 0.7);
    let tangent_color = Color::srgb(1.0, 0.8, 0.3);
    let slide_color = Color::srgb(0.0, 0.9, 1.0);

    for (progress, note_type, note_dir) in &query {
        let pos = spline.position_at_progress(progress.0);

        match note_type.0 {
            NoteKind::Tap => {
                let tangent = spline.tangent_at_progress(progress.0).normalize_or_zero();
                gizmos.circle_2d(pos, 12.0, tap_color);
                gizmos.line_2d(pos, pos + tangent * 20.0, tangent_color);
            }
            NoteKind::Slide(_) => {
                let dir_vec = note_dir
                    .map(|d| d.0.to_vec2())
                    .unwrap_or(Vec2::X);
                let size = 14.0;

                // Diamond outline
                let up = pos + Vec2::Y * size;
                let down = pos - Vec2::Y * size;
                let left = pos - Vec2::X * size;
                let right = pos + Vec2::X * size;
                gizmos.line_2d(up, right, slide_color);
                gizmos.line_2d(right, down, slide_color);
                gizmos.line_2d(down, left, slide_color);
                gizmos.line_2d(left, up, slide_color);

                // Arrow shaft
                let shaft_len = 10.0;
                let shaft_start = pos - dir_vec * shaft_len * 0.5;
                let shaft_end = pos + dir_vec * shaft_len * 0.5;
                gizmos.line_2d(shaft_start, shaft_end, slide_color);

                // Arrowhead
                let perp = Vec2::new(-dir_vec.y, dir_vec.x);
                let head_size = 5.0;
                let head_base = shaft_end - dir_vec * head_size;
                gizmos.line_2d(shaft_end, head_base + perp * head_size * 0.5, slide_color);
                gizmos.line_2d(shaft_end, head_base - perp * head_size * 0.5, slide_color);
            }
        }
    }
}
