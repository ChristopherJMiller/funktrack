use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

use crate::GameSet;
use crate::judgment::{Judgment, JudgmentResult};
use crate::state::GameScreen;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_particles, update_particles)
                .chain()
                .in_set(GameSet::Render),
        );
    }
}

// --- Config ---

const DRAG: f32 = 0.97;
const PARTICLE_SIZE: f32 = 2.5;

// --- Components ---

#[derive(Component)]
struct Particle {
    velocity: Vec2,
    lifetime: f32,
    max_lifetime: f32,
    base_color: Color,
}

// --- Systems ---

fn spawn_particles(
    mut commands: Commands,
    mut results: MessageReader<JudgmentResult>,
) {
    for result in results.read() {
        let (count, base_color, speed_range, spread) = match result.judgment {
            Judgment::Great => (24, Color::srgb(0.0, 1.0, 0.4), (80.0, 200.0), TAU),
            Judgment::Cool => (16, Color::srgb(0.0, 0.7, 1.0), (60.0, 160.0), TAU),
            Judgment::Good => (10, Color::srgb(1.0, 0.85, 0.0), (40.0, 120.0), TAU),
            Judgment::Miss => (8, Color::srgb(1.0, 0.15, 0.3), (30.0, 80.0), TAU * 0.5),
        };

        let pos = result.position;
        let lifetime = match result.judgment {
            Judgment::Great => 0.7,
            Judgment::Cool => 0.55,
            Judgment::Good => 0.4,
            Judgment::Miss => 0.35,
        };

        for i in 0..count {
            let angle = (i as f32 / count as f32) * spread
                + pseudo_random(i, 0) * 0.3
                - spread * 0.5 + TAU * 0.25; // center spread upward for Miss
            let angle = if result.judgment == Judgment::Miss {
                angle
            } else {
                (i as f32 / count as f32) * TAU + pseudo_random(i, 0) * 0.5
            };

            let speed = speed_range.0
                + (speed_range.1 - speed_range.0) * pseudo_random(i, 1);
            let dir = Vec2::new(angle.cos(), angle.sin());

            let particle_lifetime = lifetime * (0.6 + 0.4 * pseudo_random(i, 2));

            let dot = shapes::Circle {
                radius: PARTICLE_SIZE * (0.5 + pseudo_random(i, 3) * 0.5),
                center: Vec2::ZERO,
            };

            commands.spawn((
                DespawnOnExit(GameScreen::Playing),
                Particle {
                    velocity: dir * speed,
                    lifetime: particle_lifetime,
                    max_lifetime: particle_lifetime,
                    base_color,
                },
                ShapeBuilder::with(&dot)
                    .fill(base_color)
                    .build(),
                Transform::from_translation(pos.extend(3.0)),
            ));
        }
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Shape)>,
) {
    let dt = time.delta_secs();

    for (entity, mut p, mut transform, mut shape) in &mut particles {
        p.lifetime -= dt;
        if p.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Move
        transform.translation.x += p.velocity.x * dt;
        transform.translation.y += p.velocity.y * dt;

        // Drag
        p.velocity *= DRAG;

        // Gravity (slight downward drift)
        p.velocity.y -= 60.0 * dt;

        // Fade alpha
        let t = p.lifetime / p.max_lifetime;
        let alpha = t * t; // quadratic fade
        if let Some(ref mut fill) = shape.fill {
            fill.color = p.base_color.with_alpha(alpha);
        }

        // Shrink
        let scale = 0.3 + 0.7 * t;
        transform.scale = Vec3::splat(scale);
    }
}

/// Deterministic pseudo-random 0..1 based on particle index and seed.
fn pseudo_random(index: u32, seed: u32) -> f32 {
    let n = index.wrapping_mul(1103515245).wrapping_add(seed.wrapping_mul(12345));
    let n = n ^ (n >> 16);
    let n = n.wrapping_mul(2654435761);
    (n & 0xFFFF) as f32 / 65535.0
}
