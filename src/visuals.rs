use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::GameSet;
use crate::beatmap::SlideDirection;
use crate::conductor::SongConductor;
use crate::judgment::{Judgment, JudgmentFeedback};
use crate::notes::{
    DualSlideDirections, HoldEndBeat, HoldState, NoteAlive, NoteDirection, NoteKind,
    NoteTiming, NoteType, Playhead, SplineProgress,
};
use crate::path::SplinePath;
use crate::scoring::{ChainTier, ScoreState};
use crate::state::GameScreen;

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ShapePlugin)
            .add_systems(
                Update,
                spawn_path_visual
                    .run_if(in_state(GameScreen::Playing))
                    .before(GameSet::Render),
            )
            .add_systems(
                Update,
                (
                    update_playhead_visual,
                    update_note_visuals,
                    update_hold_visuals,
                    update_feedback_visuals,
                    update_chain_visuals,
                )
                    .in_set(GameSet::Render),
            );
    }
}

// --- Y2K Future Punk palette ---

const PATH_COLOR: Color = Color::srgb(0.0, 0.9, 0.9);
const PATH_WIDTH: f32 = 3.0;
const JUDGMENT_COLOR: Color = Color::WHITE;

const TAP_COLOR: Color = Color::srgb(1.0, 0.4, 0.7);
const TAP_FILL: Color = Color::srgba(1.0, 0.4, 0.7, 0.15);
const TANGENT_COLOR: Color = Color::srgb(1.0, 0.8, 0.3);
const SLIDE_COLOR: Color = Color::srgb(0.0, 0.9, 1.0);
const SLIDE_FILL: Color = Color::srgba(0.0, 0.9, 1.0, 0.1);
const HOLD_COLOR: Color = Color::srgb(1.0, 0.85, 0.15);
const HOLD_HELD_COLOR: Color = Color::srgb(1.0, 0.95, 0.5);
const HOLD_DROPPED_COLOR: Color = Color::srgb(0.5, 0.4, 0.1);
const BEAT_COLOR: Color = Color::srgb(0.8, 0.3, 1.0);
const SCRATCH_COLOR: Color = Color::srgb(1.0, 0.5, 0.1);
const CRITICAL_COLOR: Color = Color::srgb(1.0, 0.95, 0.8);
const CRITICAL_FILL: Color = Color::srgba(1.0, 0.95, 0.8, 0.2);
const DUAL_SLIDE_COLOR: Color = Color::srgb(0.4, 0.9, 1.0);
const DUAL_SLIDE_FILL: Color = Color::srgba(0.4, 0.9, 1.0, 0.1);

// --- Marker components ---

#[derive(Component)]
pub struct PathVisual;

#[derive(Component)]
struct PlayheadVisual;

#[derive(Component)]
struct NoteVisual;

#[derive(Component)]
struct TangentLine;

#[derive(Component)]
struct ArrowVisual;

#[derive(Component)]
struct ScratchLine(u8);

#[derive(Component)]
struct BeatOuterRing;

#[derive(Component)]
struct HoldRibbon;

// Feedback markers
#[derive(Component)]
struct FeedbackOuterRing;

#[derive(Component)]
struct FeedbackInnerRing;

#[derive(Component)]
struct FeedbackRay(u8);

#[derive(Component)]
struct FeedbackDiamond;

#[derive(Component)]
struct FeedbackGhost;

// --- Path visual ---

fn spawn_path_visual(
    mut commands: Commands,
    spline: Option<Res<SplinePath>>,
    existing: Query<(), With<PathVisual>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(spline) = spline else { return };

    // Build path from spline samples
    let resolution = 200;
    let mut shape_path = ShapePath::new().move_to(spline.position_at_progress(0.0));
    for i in 1..=resolution {
        let p = i as f32 / resolution as f32;
        shape_path = shape_path.line_to(spline.position_at_progress(p));
    }

    commands.spawn((
        PathVisual,
        DespawnOnExit(GameScreen::Playing),
        ShapeBuilder::with(&shape_path)
            .stroke((PATH_COLOR, PATH_WIDTH))
            .build(),
        Transform::from_translation(Vec3::Z * 0.0),
    ));

    // Playhead visual — double white circle that moves along the track
    let playhead_pos = spline.position_at_progress(0.0);

    let circle_inner = shapes::Circle {
        radius: 20.0,
        center: Vec2::ZERO,
    };
    commands.spawn((
        PlayheadVisual,
        DespawnOnExit(GameScreen::Playing),
        ShapeBuilder::with(&circle_inner)
            .stroke((JUDGMENT_COLOR, 2.0))
            .build(),
        Transform::from_translation(playhead_pos.extend(0.1)),
    ));

    let circle_outer = shapes::Circle {
        radius: 22.0,
        center: Vec2::ZERO,
    };
    commands.spawn((
        PlayheadVisual,
        DespawnOnExit(GameScreen::Playing),
        ShapeBuilder::with(&circle_outer)
            .stroke((JUDGMENT_COLOR, 1.0))
            .build(),
        Transform::from_translation(playhead_pos.extend(0.1)),
    ));
}

// --- Note visual spawning ---

pub fn spawn_note_visual(commands: &mut Commands, entity: Entity, kind: &NoteKind) {
    match kind {
        NoteKind::Tap => spawn_tap_visual(commands, entity),
        NoteKind::Slide(dir) => {
            spawn_slide_visual(commands, entity, *dir, SLIDE_COLOR, SLIDE_FILL, 14.0)
        }
        NoteKind::Hold { .. } => spawn_hold_visual(commands, entity),
        NoteKind::AdLib => spawn_adlib_visual(commands, entity),
        NoteKind::Beat => spawn_beat_visual(commands, entity),
        NoteKind::Scratch => spawn_scratch_visual(commands, entity),
        NoteKind::Critical => spawn_critical_visual(commands, entity),
        NoteKind::DualSlide(a, b) => spawn_dual_slide_visual(commands, entity, *a, *b),
    }
}

fn spawn_tap_visual(commands: &mut Commands, parent: Entity) {
    let circle = shapes::Circle {
        radius: 12.0,
        center: Vec2::ZERO,
    };
    let shape = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&circle)
                .fill(TAP_FILL)
                .stroke((TAP_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    // Tangent indicator line
    let line_shape = shapes::Line(Vec2::ZERO, Vec2::new(20.0, 0.0));
    let tangent = commands
        .spawn((
            TangentLine,
            ShapeBuilder::with(&line_shape)
                .stroke((TANGENT_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.1),
        ))
        .id();

    commands.entity(parent).add_children(&[shape, tangent]);
}

fn spawn_slide_visual(
    commands: &mut Commands,
    parent: Entity,
    dir: SlideDirection,
    color: Color,
    fill: Color,
    size: f32,
) {
    let diamond = diamond_polygon(size);
    let shape = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&diamond)
                .fill(fill)
                .stroke((color, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    // Arrow
    let arrow = arrow_path(dir.to_vec2(), 10.0);
    let arrow_entity = commands
        .spawn((
            ArrowVisual,
            ShapeBuilder::with(&arrow)
                .stroke((color, 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 1.1),
        ))
        .id();

    commands
        .entity(parent)
        .add_children(&[shape, arrow_entity]);
}

fn spawn_hold_visual(commands: &mut Commands, parent: Entity) {
    let head_outer = shapes::Circle {
        radius: 14.0,
        center: Vec2::ZERO,
    };
    let outer = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&head_outer)
                .fill(Color::srgba(1.0, 0.85, 0.15, 0.15))
                .stroke((HOLD_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    let head_inner = shapes::Circle {
        radius: 11.0,
        center: Vec2::ZERO,
    };
    let inner = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&head_inner)
                .stroke((HOLD_COLOR, 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 1.1),
        ))
        .id();

    // Ribbon placeholder — rebuilt each frame in update_hold_visuals
    let ribbon = commands
        .spawn((
            HoldRibbon,
            ShapeBuilder::with(&shapes::Line(Vec2::ZERO, Vec2::ZERO))
                .stroke((HOLD_COLOR, 2.0))
                .build(),
        ))
        .id();

    commands
        .entity(parent)
        .add_children(&[ribbon, outer, inner]);
}

fn spawn_adlib_visual(commands: &mut Commands, parent: Entity) {
    let circle = shapes::Circle {
        radius: 10.0,
        center: Vec2::ZERO,
    };
    let child = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&circle)
                .stroke((Color::srgba(0.9, 0.9, 1.0, 0.08), 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 0.5),
        ))
        .id();

    commands.entity(parent).add_children(&[child]);
}

fn spawn_beat_visual(commands: &mut Commands, parent: Entity) {
    let inner_circle = shapes::Circle {
        radius: 10.0,
        center: Vec2::ZERO,
    };
    let inner = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&inner_circle)
                .stroke((BEAT_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    let outer_circle = shapes::Circle {
        radius: 16.0,
        center: Vec2::ZERO,
    };
    let outer = commands
        .spawn((
            BeatOuterRing,
            ShapeBuilder::with(&outer_circle)
                .stroke((BEAT_COLOR, 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 0.9),
        ))
        .id();

    commands.entity(parent).add_children(&[inner, outer]);
}

fn spawn_scratch_visual(commands: &mut Commands, parent: Entity) {
    let disc = shapes::Circle {
        radius: 13.0,
        center: Vec2::ZERO,
    };
    let disc_entity = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&disc)
                .stroke((SCRATCH_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    let mut children = vec![disc_entity];
    for i in 0..3u8 {
        let angle = i as f32 * TAU / 3.0;
        let d = Vec2::new(angle.cos(), angle.sin());
        let line = shapes::Line(d * 10.0, d * 18.0);
        let line_entity = commands
            .spawn((
                ScratchLine(i),
                ShapeBuilder::with(&line)
                    .stroke((SCRATCH_COLOR, 1.5))
                    .build(),
                Transform::from_translation(Vec3::Z * 1.1),
            ))
            .id();
        children.push(line_entity);
    }

    commands.entity(parent).add_children(&children);
}

fn spawn_critical_visual(commands: &mut Commands, parent: Entity) {
    let star = star_polygon(16.0, 8.0, 5);
    let shape = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&star)
                .fill(CRITICAL_FILL)
                .stroke((CRITICAL_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    commands.entity(parent).add_children(&[shape]);
}

fn spawn_dual_slide_visual(
    commands: &mut Commands,
    parent: Entity,
    dir_a: SlideDirection,
    dir_b: SlideDirection,
) {
    let diamond = diamond_polygon(18.0);
    let shape = commands
        .spawn((
            NoteVisual,
            ShapeBuilder::with(&diamond)
                .fill(DUAL_SLIDE_FILL)
                .stroke((DUAL_SLIDE_COLOR, 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 1.0),
        ))
        .id();

    let arrow_a_path = arrow_path(dir_a.to_vec2(), 8.0);
    let arrow_a = commands
        .spawn((
            ArrowVisual,
            ShapeBuilder::with(&arrow_a_path)
                .stroke((DUAL_SLIDE_COLOR, 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 1.1),
        ))
        .id();

    let arrow_b_path = arrow_path(dir_b.to_vec2(), 8.0);
    let arrow_b = commands
        .spawn((
            ArrowVisual,
            ShapeBuilder::with(&arrow_b_path)
                .stroke((DUAL_SLIDE_COLOR, 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 1.1),
        ))
        .id();

    commands
        .entity(parent)
        .add_children(&[shape, arrow_a, arrow_b]);
}

// --- Feedback visual spawning ---

pub fn spawn_feedback_visual(commands: &mut Commands, entity: Entity, judgment: Judgment) {
    let color = judgment.color();

    // Outer blast ring
    let outer_circle = shapes::Circle {
        radius: 20.0,
        center: Vec2::ZERO,
    };
    let outer = commands
        .spawn((
            FeedbackOuterRing,
            ShapeBuilder::with(&outer_circle)
                .stroke((color.with_alpha(0.9), 3.0))
                .build(),
            Transform::from_translation(Vec3::Z * 2.0),
        ))
        .id();

    // Inner ring
    let inner_circle = shapes::Circle {
        radius: 14.0,
        center: Vec2::ZERO,
    };
    let inner = commands
        .spawn((
            FeedbackInnerRing,
            ShapeBuilder::with(&inner_circle)
                .stroke((color.with_alpha(0.7), 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 2.1),
        ))
        .id();

    // 8 starburst rays
    let mut rays = Vec::new();
    for i in 0..8u8 {
        let angle = (i as f32 / 8.0) * TAU + 0.3;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let line = shapes::Line(dir * 10.0, dir * 32.0);
        let ray = commands
            .spawn((
                FeedbackRay(i),
                ShapeBuilder::with(&line)
                    .stroke((color.with_alpha(0.8), 1.5))
                    .build(),
                Transform::from_translation(Vec3::Z * 2.0),
            ))
            .id();
        rays.push(ray);
    }

    // Diamond flash
    let diamond = diamond_polygon(8.0);
    let diamond_entity = commands
        .spawn((
            FeedbackDiamond,
            ShapeBuilder::with(&diamond)
                .stroke((Color::WHITE.with_alpha(0.9), 2.0))
                .build(),
            Transform::from_translation(Vec3::Z * 2.2),
        ))
        .id();

    // Ghost ring
    let ghost_circle = shapes::Circle {
        radius: 15.0,
        center: Vec2::ZERO,
    };
    let ghost = commands
        .spawn((
            FeedbackGhost,
            ShapeBuilder::with(&ghost_circle)
                .stroke((color.with_alpha(0.0), 1.5))
                .build(),
            Transform::from_translation(Vec3::Z * 1.9),
        ))
        .id();

    let mut children = vec![outer, inner, diamond_entity, ghost];
    children.extend(rays);
    commands.entity(entity).add_children(&children);
}

// --- Update systems ---

fn update_playhead_visual(
    conductor: Option<Res<SongConductor>>,
    playhead: Option<Res<Playhead>>,
    spline: Option<Res<SplinePath>>,
    mut playhead_q: Query<&mut Transform, With<PlayheadVisual>>,
) {
    let Some(conductor) = conductor else { return };
    let Some(playhead) = playhead else { return };
    let Some(spline) = spline else { return };

    let progress = playhead.progress(conductor.current_beat);
    let pos = spline.position_at_progress(progress);

    for mut t in &mut playhead_q {
        t.translation = pos.extend(t.translation.z);
    }
}

fn update_note_visuals(
    notes: Query<
        (
            Entity,
            &SplineProgress,
            &NoteType,
            Option<&NoteDirection>,
            Option<&DualSlideDirections>,
            &Children,
        ),
        With<NoteAlive>,
    >,
    conductor: Option<Res<SongConductor>>,
    spline: Option<Res<SplinePath>>,
    mut transforms: Query<&mut Transform>,
    mut shapes: Query<&mut Shape>,
    scratch_lines: Query<&ScratchLine>,
    beat_rings: Query<&BeatOuterRing>,
    tangent_lines: Query<&TangentLine>,
) {
    let Some(spline) = spline else { return };
    let Some(conductor) = conductor else { return };

    for (entity, progress, note_type, _note_dir, _dual_dirs, children) in &notes {
        let p = progress.0.min(1.0);
        let pos = spline.position_at_progress(p);
        let tangent = spline.tangent_at_progress(p).normalize_or_zero();

        // Move parent entity
        if let Ok(mut t) = transforms.get_mut(entity) {
            t.translation = pos.extend(t.translation.z);
        }

        for child in children.iter() {
            // Scratch: spin lines
            if let Ok(scratch) = scratch_lines.get(child) {
                if let Ok(mut t) = transforms.get_mut(child) {
                    let spin = conductor.current_beat as f32 * TAU * 2.0;
                    let base_angle = scratch.0 as f32 * TAU / 3.0;
                    t.rotation = Quat::from_rotation_z(spin + base_angle);
                }
            }

            // Beat: pulse outer ring
            if beat_rings.get(child).is_ok() {
                if let Ok(mut t) = transforms.get_mut(child) {
                    let pulse =
                        1.0 + 0.15 * (conductor.current_beat as f32 * TAU * 2.0).sin();
                    t.scale = Vec3::splat(pulse);
                }
            }

            // Tap: rotate tangent line toward path direction
            if tangent_lines.get(child).is_ok() {
                if let Ok(mut t) = transforms.get_mut(child) {
                    let angle = tangent.y.atan2(tangent.x);
                    t.rotation = Quat::from_rotation_z(angle);
                }
            }
        }

        // AdLib: pulse alpha
        if matches!(note_type.0, NoteKind::AdLib) {
            let pulse = 0.08 + 0.06 * (conductor.current_beat as f32 * TAU).sin().abs();
            for child in children.iter() {
                if let Ok(mut shape) = shapes.get_mut(child) {
                    if let Some(ref mut stroke) = shape.stroke {
                        stroke.color = Color::srgba(0.9, 0.9, 1.0, pulse);
                    }
                }
            }
        }
    }
}

fn update_hold_visuals(
    holds: Query<
        (
            &SplineProgress,
            &NoteTiming,
            Option<&HoldEndBeat>,
            Option<&HoldState>,
            &Children,
        ),
        With<NoteAlive>,
    >,
    playhead: Option<Res<Playhead>>,
    spline: Option<Res<SplinePath>>,
    ribbons: Query<&HoldRibbon>,
    note_visuals: Query<&NoteVisual>,
    mut shapes: Query<&mut Shape>,
) {
    let Some(spline) = spline else { return };
    let Some(playhead) = playhead else { return };

    for (progress, _timing, hold_end, hold_state, children) in &holds {
        let Some(hold_end) = hold_end else {
            continue;
        };
        let state = hold_state.copied().unwrap_or(HoldState::Pending);

        let color = match state {
            HoldState::Held => HOLD_HELD_COLOR,
            HoldState::Dropped => HOLD_DROPPED_COLOR,
            _ => HOLD_COLOR,
        };

        // Head is at the note's fixed spline position
        let head_p = progress.0;
        // Tail is at the hold end beat's spline position
        let tail_p = playhead.progress(hold_end.0);

        // Parent entity's world position (used to convert to local coords)
        let parent_pos = spline.position_at_progress(head_p);

        for child in children.iter() {
            // Update ribbon — stretches from head to tail along spline
            if ribbons.get(child).is_ok() {
                let p_start = head_p.min(tail_p);
                let p_end = head_p.max(tail_p);
                if p_end > p_start {
                    let segments = 16;
                    let step = (p_end - p_start) / segments as f32;

                    let mut top_pts = Vec::with_capacity(segments + 1);
                    let mut bot_pts = Vec::with_capacity(segments + 1);
                    for i in 0..=segments {
                        let pa = p_start + step * i as f32;
                        let a = spline.position_at_progress(pa) - parent_pos;
                        let tang = spline.tangent_at_progress(pa).normalize_or_zero();
                        let perp = Vec2::new(-tang.y, tang.x) * 4.0;
                        top_pts.push(a + perp);
                        bot_pts.push(a - perp);
                    }

                    let mut points = top_pts;
                    bot_pts.reverse();
                    points.extend(bot_pts);

                    let ribbon_shape = shapes::Polygon {
                        points,
                        closed: true,
                    };

                    if let Ok(mut shape) = shapes.get_mut(child) {
                        *shape = ShapeBuilder::with(&ribbon_shape)
                            .stroke((color, 2.0))
                            .build();
                    }
                }
            }

            // Update head circle colors
            if note_visuals.get(child).is_ok() {
                if let Ok(mut shape) = shapes.get_mut(child) {
                    if let Some(ref mut stroke) = shape.stroke {
                        stroke.color = color;
                    }
                    if let Some(ref mut fill) = shape.fill {
                        fill.color = color.with_alpha(0.15);
                    }
                }
            }
        }
    }
}

fn update_feedback_visuals(
    feedbacks: Query<(&JudgmentFeedback, &Children)>,
    mut transforms: Query<&mut Transform>,
    mut shapes: Query<&mut Shape>,
    outer_rings: Query<&FeedbackOuterRing>,
    inner_rings: Query<&FeedbackInnerRing>,
    rays: Query<&FeedbackRay>,
    diamonds: Query<&FeedbackDiamond>,
    ghosts: Query<&FeedbackGhost>,
) {
    for (fb, children) in &feedbacks {
        let t = 1.0 - (fb.timer / fb.max_time);
        let color = fb.judgment.color();

        let ease_out = 1.0 - (1.0 - t) * (1.0 - t);
        let pop = if t < 0.15 { t / 0.15 } else { 1.0 };
        let alpha = if t < 0.6 {
            1.0
        } else {
            1.0 - (t - 0.6) / 0.4
        };

        for child in children.iter() {
            // Outer blast ring
            if outer_rings.get(child).is_ok() {
                let scale = (20.0 + 45.0 * ease_out) / 20.0;
                if let Ok(mut tr) = transforms.get_mut(child) {
                    tr.scale = Vec3::splat(scale);
                }
                if let Ok(mut shape) = shapes.get_mut(child) {
                    if let Some(ref mut stroke) = shape.stroke {
                        stroke.color = color.with_alpha(alpha * 0.9);
                    }
                }
            }

            // Inner ring
            if inner_rings.get(child).is_ok() {
                let inner_scale = if t < 0.2 {
                    (t / 0.2) * 1.3
                } else {
                    1.3 - 0.3 * ((t - 0.2) / 0.8).min(1.0)
                };
                let scale = inner_scale * pop;
                if let Ok(mut tr) = transforms.get_mut(child) {
                    tr.scale = Vec3::splat(scale);
                }
                if let Ok(mut shape) = shapes.get_mut(child) {
                    if let Some(ref mut stroke) = shape.stroke {
                        stroke.color = color.with_alpha(alpha * 0.7);
                    }
                }
            }

            // Rays
            if let Ok(ray) = rays.get(child) {
                let angle = (ray.0 as f32 / 8.0) * TAU + 0.3;
                let length_mult = if ray.0 % 2 == 0 { 1.0 } else { 0.7 };
                let ray_start = 10.0 + 8.0 * ease_out;
                let outer_r = 20.0 + 45.0 * ease_out;
                let ray_end = outer_r + 12.0 * ease_out;
                let total_len = ray_start + (ray_end - ray_start) * length_mult;
                let scale = total_len / 32.0;

                if let Ok(mut tr) = transforms.get_mut(child) {
                    tr.scale = Vec3::new(scale, 1.0, 1.0);
                    tr.rotation = Quat::from_rotation_z(angle);
                }
                if let Ok(mut shape) = shapes.get_mut(child) {
                    if let Some(ref mut stroke) = shape.stroke {
                        stroke.color = color.with_alpha(alpha * 0.8);
                    }
                }
            }

            // Diamond flash
            if diamonds.get(child).is_ok() {
                if let Ok(mut shape) = shapes.get_mut(child) {
                    if t < 0.3 {
                        let diamond_alpha = 1.0 - t / 0.3;
                        if let Ok(mut tr) = transforms.get_mut(child) {
                            tr.scale = Vec3::splat(pop);
                        }
                        if let Some(ref mut stroke) = shape.stroke {
                            stroke.color = Color::WHITE.with_alpha(diamond_alpha * 0.9);
                        }
                    } else if let Some(ref mut stroke) = shape.stroke {
                        stroke.color = Color::WHITE.with_alpha(0.0);
                    }
                }
            }

            // Ghost ring
            if ghosts.get(child).is_ok() {
                if t > 0.1 {
                    let ghost_t = (t - 0.1).min(1.0);
                    let ghost_ease = 1.0 - (1.0 - ghost_t) * (1.0 - ghost_t);
                    let scale = (15.0 + 55.0 * ghost_ease) / 15.0;
                    let ghost_alpha = alpha * 0.3;
                    if let Ok(mut tr) = transforms.get_mut(child) {
                        tr.scale = Vec3::splat(scale);
                    }
                    if let Ok(mut shape) = shapes.get_mut(child) {
                        if let Some(ref mut stroke) = shape.stroke {
                            stroke.color = color.with_alpha(ghost_alpha);
                        }
                    }
                }
            }
        }
    }
}

fn update_chain_visuals(
    score: Option<Res<ScoreState>>,
    mut path_q: Query<&mut Shape, With<PathVisual>>,
) {
    let Some(score) = score else { return };

    let (color, width) = match score.chain_tier() {
        ChainTier::Normal => (PATH_COLOR, PATH_WIDTH),
        ChainTier::Fever => (Color::srgb(1.0, 0.85, 0.15), 5.0),
        ChainTier::Trance => (Color::WHITE, 7.0),
    };

    for mut shape in &mut path_q {
        if let Some(ref mut stroke) = shape.stroke {
            stroke.color = color;
            stroke.options.line_width = width;
        }
    }
}

// --- Shape builder helpers ---

fn diamond_polygon(size: f32) -> shapes::Polygon {
    shapes::Polygon {
        points: vec![
            Vec2::new(0.0, size),
            Vec2::new(size, 0.0),
            Vec2::new(0.0, -size),
            Vec2::new(-size, 0.0),
        ],
        closed: true,
    }
}

fn arrow_path(dir: Vec2, shaft_len: f32) -> ShapePath {
    let half = shaft_len * 0.5;
    let start = -dir * half;
    let end = dir * half;

    let perp = Vec2::new(-dir.y, dir.x);
    let head_size = shaft_len * 0.4;
    let head_base = end - dir * head_size;

    ShapePath::new()
        .move_to(start)
        .line_to(end)
        .move_to(end)
        .line_to(head_base + perp * head_size * 0.5)
        .move_to(end)
        .line_to(head_base - perp * head_size * 0.5)
}

fn star_polygon(outer_r: f32, inner_r: f32, num_points: usize) -> shapes::Polygon {
    let offset = -std::f32::consts::FRAC_PI_2;
    let mut points = Vec::with_capacity(num_points * 2);

    for i in 0..num_points {
        let a_outer = (i as f32 / num_points as f32) * TAU + offset;
        let a_inner = ((i as f32 + 0.5) / num_points as f32) * TAU + offset;

        points.push(Vec2::new(a_outer.cos(), a_outer.sin()) * outer_r);
        points.push(Vec2::new(a_inner.cos(), a_inner.sin()) * inner_r);
    }

    shapes::Polygon {
        points,
        closed: true,
    }
}
