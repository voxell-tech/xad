//! Example: `split_intersections` shown side by side, both facing the camera.
//!
//! Left sketch: the pre-processed curves with debug on (control points + hull).
//! Right sketch: the same curves after `split_intersections`, with every piece
//! recolored and endpoint dots marking the new split points at each crossing.

use bevy::prelude::*;
use bezier::{BezierCurve, split_intersections};
use sketch::{Plane, Sketch};
use xad::material::BezierMaterialPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierMaterialPlugin))
        .add_systems(Startup, setup)
        .run();
}

const HALF_EXTENT: f32 = 1.1;
const ROW_OFFSET: f32 = 1.1;

// so that its clear whats what
const PALETTE: [LinearRgba; 6] = [
    LinearRgba::new(1.00, 0.30, 0.30, 1.0),
    LinearRgba::new(0.20, 0.80, 1.00, 1.0),
    LinearRgba::new(0.30, 1.00, 0.45, 1.0),
    LinearRgba::new(1.00, 0.85, 0.10, 1.0),
    LinearRgba::new(0.90, 0.40, 1.00, 1.0),
    LinearRgba::new(1.00, 0.55, 0.10, 1.0),
];

fn setup(mut commands: Commands) {
    // Pulled back far enough to frame the 2x2 grid of sketches.
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 6.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Row 1: two arcs that cross twice.
    let arc_up = BezierCurve::new(
        Vec2::new(0.20, 0.70),
        Vec2::new(0.50, 0.15),
        Vec2::new(0.80, 0.70),
    );
    let arc_down = BezierCurve::new(
        Vec2::new(0.20, 0.30),
        Vec2::new(0.50, 0.85),
        Vec2::new(0.80, 0.30),
    );
    spawn_pair(&mut commands, vec![arc_up, arc_down], ROW_OFFSET);

    // Row 2: three curves (arc + two lines) that cross to enclose a shape.
    let arc = BezierCurve::new(
        Vec2::new(0.42, 0.82),
        Vec2::new(0.08, 0.42),
        Vec2::new(0.34, 0.05),
    );
    let line_flat = BezierCurve::new(
        Vec2::new(0.20, 0.61),
        Vec2::new(0.55, 0.595),
        Vec2::new(0.90, 0.58),
    );
    let line_diag = BezierCurve::new(
        Vec2::new(0.80, 0.85),
        Vec2::new(0.49, 0.465),
        Vec2::new(0.18, 0.08),
    );
    spawn_pair(
        &mut commands,
        vec![arc, line_flat, line_diag],
        -ROW_OFFSET,
    );
}

// Spawns a before/after pair on one row: pre-processed (debug on) at the left,
// split result (endpoints dotted) at the right.
fn spawn_pair(
    commands: &mut Commands,
    curves: Vec<BezierCurve>,
    y_offset: f32,
) {
    let before: Vec<BezierCurve> = curves
        .iter()
        .enumerate()
        .map(|(i, c)| {
            style(*c, PALETTE[i % PALETTE.len()]).with_debug(true)
        })
        .collect();

    let after: Vec<BezierCurve> = split_intersections(curves)
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            style(c, PALETTE[i % PALETTE.len()])
                .with_draw_endpoints(true)
        })
        .collect();

    spawn_sketch(commands, before, -HALF_EXTENT, y_offset);
    spawn_sketch(commands, after, HALF_EXTENT, y_offset);
}

fn style(curve: BezierCurve, color: LinearRgba) -> BezierCurve {
    curve
        .with_color(color)
        .with_width(0.005)
        .with_endpoint_radius(0.01)
}

fn spawn_sketch(
    commands: &mut Commands,
    curves: Vec<BezierCurve>,
    x_offset: f32,
    y_offset: f32,
) {
    commands
        .spawn(
            Sketch {
                plane: Plane::right(),
                curves,
            }
            .into_bundle(),
        )
        .insert(Transform::from_translation(Vec3::new(
            x_offset, y_offset, 0.0,
        )));
}
