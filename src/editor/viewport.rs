use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::beatmap::PathSegment;
use crate::path::SplinePath;

use super::{EditorElement, EditorEntity, EditorState};

const PATH_COLOR: Color = Color::srgb(0.0, 0.9, 0.9);
const PATH_WIDTH: f32 = 3.0;
const CONTROL_POINT_RADIUS: f32 = 8.0;
const CONTROL_POINT_COLOR: Color = Color::srgb(1.0, 0.4, 0.7);
const CONTROL_POINT_SELECTED: Color = Color::srgb(0.0, 1.0, 0.4);
const NOTE_PREVIEW_RADIUS: f32 = 6.0;

/// Marker for path visual entities in the editor viewport.
#[derive(Component)]
pub struct EditorPathVisual;

/// Marker for control point visual entities.
#[derive(Component)]
pub struct EditorControlPoint;

/// Marker for note position preview dots on the path.
#[derive(Component)]
pub struct EditorNotePreview;

/// Renders the path spline and control points in the Bevy viewport.
/// In Path mode this is the primary workspace; in Chart mode it's a background preview.
pub fn render_viewport(
    mut commands: Commands,
    state: Option<Res<EditorState>>,
    path_visuals: Query<Entity, With<EditorPathVisual>>,
    cp_visuals: Query<Entity, With<EditorControlPoint>>,
    note_previews: Query<Entity, With<EditorNotePreview>>,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }

    // Despawn old visuals
    for e in &path_visuals {
        commands.entity(e).despawn();
    }
    for e in &cp_visuals {
        commands.entity(e).despawn();
    }
    for e in &note_previews {
        commands.entity(e).despawn();
    }

    // Collect all CatmullRom points
    let mut all_points: Vec<Vec2> = Vec::new();
    for seg in &state.chart.path_segments {
        if let PathSegment::CatmullRom { points, .. } = seg {
            for &(x, y) in points {
                all_points.push(Vec2::new(x, y));
            }
        }
    }

    // Draw the spline curve if we have enough points
    if all_points.len() >= 4 {
        let spline = CubicCardinalSpline::new_catmull_rom(all_points.clone());
        if let Ok(curve) = spline.to_curve() {
            let resolution = 200;
            let num_segments = curve.segments().len();
            let t_max = num_segments as f32;

            let mut path_builder = ShapePath::new().move_to(curve.position(0.0));
            for i in 1..=resolution {
                let t = (i as f32 / resolution as f32) * t_max;
                path_builder = path_builder.line_to(curve.position(t));
            }

            commands.spawn((
                EditorEntity,
                EditorPathVisual,
                ShapeBuilder::with(&path_builder)
                    .stroke((PATH_COLOR, PATH_WIDTH))
                    .build(),
                Transform::from_translation(Vec3::Z * 0.0),
            ));
        }

        // Draw note position previews on the path
        let spline_path = SplinePath::from_catmull_rom_points(all_points.clone());
        let total_beats = state.total_beats;
        for note in &state.chart.notes {
            let progress = (note.beat / total_beats).clamp(0.0, 1.0) as f32;
            let pos = spline_path.position_at_progress(progress);
            let dot = shapes::Circle {
                radius: NOTE_PREVIEW_RADIUS,
                ..default()
            };
            commands.spawn((
                EditorEntity,
                EditorNotePreview,
                ShapeBuilder::with(&dot)
                    .fill(Color::srgba(1.0, 0.4, 0.7, 0.3))
                    .stroke((Color::srgba(1.0, 0.4, 0.7, 0.6), 1.0))
                    .build(),
                Transform::from_translation(Vec3::new(pos.x, pos.y, 0.5)),
            ));
        }
    }

    // Draw control points (always visible)
    let mut global_idx = 0usize;
    for (seg_idx, seg) in state.chart.path_segments.iter().enumerate() {
        if let PathSegment::CatmullRom { points, .. } = seg {
            for (pt_idx, &(x, y)) in points.iter().enumerate() {
                let is_selected = state
                    .selected
                    .contains(&EditorElement::PathControlPoint {
                        segment: seg_idx,
                        index: pt_idx,
                    });
                let color = if is_selected {
                    CONTROL_POINT_SELECTED
                } else {
                    CONTROL_POINT_COLOR
                };

                let circle = shapes::Circle {
                    radius: CONTROL_POINT_RADIUS,
                    ..default()
                };

                commands.spawn((
                    EditorEntity,
                    EditorControlPoint,
                    ShapeBuilder::with(&circle)
                        .fill(color.with_alpha(0.3))
                        .stroke((color, 2.0))
                        .build(),
                    Transform::from_translation(Vec3::new(x, y, 2.0)),
                ));

                global_idx += 1;
            }
        }
    }
    // Suppress unused variable warning
    let _ = global_idx;
}
