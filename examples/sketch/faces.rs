//! Example: faces are filled automatically for a sketch.
//!
//! Spawning a `Sketch` is enough: the `FaceMaterialPlugin` splits the curves
//! at their intersections, detects the enclosed regions, and fills them in a
//! single layer in front of the strokes. The scene exercises the interesting
//! cases: a hash sign (one face, eight dangling tails), a concave star, a
//! pentagon with a triangle island floating inside, a lens enclosed by two
//! arcs, and a circle cut in two by a chord.

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
        center + r * Vec2::new(angle.cos(), angle.sin())
    };
    (0..count).map(|i| line(vertex(i), vertex(i + 1))).collect()
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 3.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let mut curves = Vec::new();

    // Hash sign: four lines whose center square is the only enclosed
    // region; the eight tails are filaments.
    let center = Vec2::new(-1.5, 0.7);
    let (s, tail) = (0.35, 0.25);
    for side in [-s, s] {
        let reach = s + tail;
        curves.push(line(
            center + Vec2::new(-reach, side),
            center + Vec2::new(reach, side),
        ));
        curves.push(line(
            center + Vec2::new(side, -reach),
            center + Vec2::new(side, reach),
        ));
    }

    // Four-pointed star: one concave face.
    curves.extend(polygon(Vec2::new(-1.5, -0.7), 0.55, 8, 0.4));

    // Pentagon with a triangle island floating inside.
    curves.extend(polygon(Vec2::new(0.0, 0.7), 0.5, 5, 1.0));
    curves.extend(polygon(Vec2::new(0.0, 0.7), 0.2, 3, 1.0));

    // Lens: two arcs crossing twice.
    let lens = Vec2::new(0.0, -0.75);
    curves.push(
        BezierCurve::new(
            lens + Vec2::new(-0.45, 0.25),
            lens + Vec2::new(0.0, -0.45),
            lens + Vec2::new(0.45, 0.25),
        )
        .with_width(STROKE)
        .with_color(gen_color()),
    );
    curves.push(
        BezierCurve::new(
            lens + Vec2::new(-0.45, -0.25),
            lens + Vec2::new(0.0, 0.45),
            lens + Vec2::new(0.45, -0.25),
        )
        .with_width(STROKE)
        .with_color(gen_color()),
    );

    // Circle cut in two by a chord; the chord's ends dangle outside.
    curves.extend(circle(
        Vec2::new(1.6, 0.0),
        0.5,
        gen_color(),
        STROKE,
    ));
    curves.push(line(Vec2::new(1.0, -0.65), Vec2::new(2.2, 0.55)));

    commands.spawn(
        Sketch {
            position: Vec2::ZERO,
            plane: Plane::right(),
            curves,
        }
        .into_bundle(),
    );
}
