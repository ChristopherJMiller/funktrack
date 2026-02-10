use bevy::prelude::*;

pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_path)
            .add_systems(Update, render_path);
    }
}

/// Lookup table mapping arc-length distance to spline parameter `t`.
///
/// Raw spline parameters don't produce uniform speed — notes would bunch up
/// in high-curvature regions. This LUT lets us convert a desired distance
/// along the curve into the correct parameter via binary search + lerp.
pub struct ArcLengthLut {
    /// (accumulated_distance, parameter_t) pairs, monotonically increasing.
    entries: Vec<(f32, f32)>,
}

impl ArcLengthLut {
    /// Build a LUT by sampling the curve at `samples_per_segment` points per segment.
    fn build(curve: &CubicCurve<Vec2>, samples_per_segment: usize) -> Self {
        let num_segments = curve.segments().len();
        let total_samples = num_segments * samples_per_segment;
        let mut entries = Vec::with_capacity(total_samples + 1);

        let t_max = num_segments as f32;
        let mut accumulated = 0.0_f32;
        let mut prev_pos = curve.position(0.0);

        entries.push((0.0, 0.0));

        for i in 1..=total_samples {
            let t = (i as f32 / total_samples as f32) * t_max;
            let pos = curve.position(t);
            accumulated += prev_pos.distance(pos);
            entries.push((accumulated, t));
            prev_pos = pos;
        }

        Self { entries }
    }

    /// Total arc length of the curve.
    fn total_length(&self) -> f32 {
        self.entries.last().map(|(d, _)| *d).unwrap_or(0.0)
    }

    /// Convert a distance along the curve to a spline parameter `t`.
    /// Uses binary search + linear interpolation.
    fn distance_to_parameter(&self, distance: f32) -> f32 {
        let distance = distance.clamp(0.0, self.total_length());

        // Binary search for the segment containing this distance.
        let idx = self
            .entries
            .partition_point(|(d, _)| *d < distance)
            .min(self.entries.len() - 1);

        if idx == 0 {
            return self.entries[0].1;
        }

        let (d0, t0) = self.entries[idx - 1];
        let (d1, t1) = self.entries[idx];

        if (d1 - d0).abs() < f32::EPSILON {
            return t0;
        }

        let frac = (distance - d0) / (d1 - d0);
        t0 + frac * (t1 - t0)
    }
}

/// The spline path that notes travel along.
#[derive(Resource)]
pub struct SplinePath {
    curve: CubicCurve<Vec2>,
    lut: ArcLengthLut,
}

impl SplinePath {
    /// Sample position at normalized progress (0.0 = start, 1.0 = end).
    pub fn position_at_progress(&self, progress: f32) -> Vec2 {
        let distance = progress.clamp(0.0, 1.0) * self.lut.total_length();
        let t = self.lut.distance_to_parameter(distance);
        self.curve.position(t)
    }

    /// Sample tangent (velocity direction) at normalized progress.
    pub fn tangent_at_progress(&self, progress: f32) -> Vec2 {
        let distance = progress.clamp(0.0, 1.0) * self.lut.total_length();
        let t = self.lut.distance_to_parameter(distance);
        self.curve.velocity(t)
    }
}

fn setup_path(mut commands: Commands) {
    // Hardcoded S-curve with 6 control points.
    let control_points = vec![
        Vec2::new(-500.0, -200.0),
        Vec2::new(-250.0, 200.0),
        Vec2::new(-50.0, -150.0),
        Vec2::new(50.0, 150.0),
        Vec2::new(250.0, -200.0),
        Vec2::new(500.0, 200.0),
    ];

    let spline = CubicCardinalSpline::new_catmull_rom(control_points);
    let curve = spline.to_curve().expect("need at least 4 control points");
    let lut = ArcLengthLut::build(&curve, 1000);

    commands.insert_resource(SplinePath { curve, lut });
}

fn render_path(spline: Res<SplinePath>, mut gizmos: Gizmos) {
    let resolution = 100 * spline.curve.segments().len();
    gizmos.linestrip_2d(
        spline.curve.iter_positions(resolution),
        Color::srgb(0.0, 0.9, 0.9),
    );

    // Judgment point — double white circle at end of path
    let judgment_pos = spline.position_at_progress(1.0);
    gizmos.circle_2d(judgment_pos, 20.0, Color::WHITE);
    gizmos.circle_2d(judgment_pos, 22.0, Color::WHITE);
}
