//! Example: faces are filled automatically for a sketch.
//!
//! Spawning a `Sketch` is enough: the `FaceMaterialPlugin` splits the curves
//! at their intersections, detects the enclosed regions, and fills them in a
//! single layer in front of the strokes. The scene leans on the hard cases:
//! a pentagram (five self-crossing lines carve six faces), a three-circle
//! Venn diagram (seven curved faces), an eight-pointed star (sixteen sharp
//! corners), three blobs nested inside each other, and a circle cut in two
//! by a chord whose ends dangle.

use bevy::prelude::*;
use bezier::shapes::circle::circle;
use bezier::{BezierCurve, gen_color};
use sketch::{Plane, Sketch};
use xad::material::{BezierMaterialPlugin, FaceMaterialPlugin};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BezierMaterialPlugin,
            FaceMaterialPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

const STROKE: f32 = 0.012;

fn line(a: Vec2, b: Vec2) -> BezierCurve {
    BezierCurve::new(a, a.lerp(b, 0.5), b)
        .with_width(STROKE)
        .with_color(gen_color())
}

fn dir(angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin())
}

/// Closed loop of lines through `count` points of `radius` around `center`,
/// with every second point pulled in to `pinch` of the radius (1.0 = regular
/// polygon, less = star).
fn polygon(
    center: Vec2,
    radius: f32,
    count: usize,
    pinch: f32,
) -> Vec<BezierCurve> {
    let vertex = |i: usize| {
        let angle = std::f32::consts::TAU * (i % count) as f32
            / count as f32
            + std::f32::consts::FRAC_PI_2;
        let r = if i.is_multiple_of(2) {
            radius
        } else {
            radius * pinch
        };
        center + r * dir(angle)
    };
    (0..count).map(|i| line(vertex(i), vertex(i + 1))).collect()
}

/// Lopsided closed blob: two cubics joined at the sides.
fn blob(center: Vec2, size: f32) -> Vec<BezierCurve> {
    let at = |x: f32, y: f32| center + size * Vec2::new(x, y);
    let (left, right) = (at(-1.0, 0.1), at(1.0, -0.15));
    [
        BezierCurve::cubic(left, at(-0.9, 1.5), at(1.2, 0.9), right),
        BezierCurve::cubic(
            right,
            at(1.0, -1.3),
            at(-1.6, -1.0),
            left,
        ),
    ]
    .into_iter()
    .flatten()
    .map(|c| c.with_width(STROKE).with_color(gen_color()))
    .collect()
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let tau = std::f32::consts::TAU;
    let up = std::f32::consts::FRAC_PI_2;
    let mut curves = Vec::new();

    // Pentagram: five lines, each crossing the other four. The splits carve
    // out the five points and the central pentagon: six faces from five
    // curves, with ten crossing vertices and five sharp tips.
    let center = Vec2::new(-1.9, 0.95);
    let point: Vec<Vec2> = (0..5)
        .map(|i| center + 0.8 * dir(up + tau * i as f32 / 5.0))
        .collect();
    for i in 0..5 {
        curves.push(line(point[i], point[(i + 2) % 5]));
    }

    // Three-circle Venn diagram: seven curved faces.
    let hub = Vec2::new(1.5, 0.95);
    for k in 0..3 {
        let c = hub + 0.33 * dir(up + tau * k as f32 / 3.0);
        curves.extend(circle(c, 0.58, gen_color(), STROKE));
    }

    // Eight-pointed star: sixteen sharp corners in one concave face.
    curves.extend(polygon(Vec2::new(-1.9, -1.05), 0.8, 16, 0.42));

    // Blob nest: each blob floats inside the previous one, so every ring
    // inherits the next blob's outside.
    for size in [0.62, 0.36, 0.16] {
        curves.extend(blob(Vec2::new(0.0, -1.0), size));
    }

    // Circle cut in two by a chord; the chord's ends dangle outside.
    curves.extend(circle(
        Vec2::new(1.6, -1.05),
        0.5,
        gen_color(),
        STROKE,
    ));
    curves.push(line(Vec2::new(1.0, -1.7), Vec2::new(2.2, -0.5)));

    commands.spawn(
        Sketch {
            position: Vec2::ZERO,
            plane: Plane::right(),
            curves,
        }
        .into_bundle(),
    );
}
