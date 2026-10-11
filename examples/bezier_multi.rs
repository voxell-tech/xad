//! Example: three planes with several cubic Bezier curves each.
//!
//! Bezier curves are drawn on planes, that can be oriented in
//! whatever direction.

use bevy::prelude::*;
use bezier::BezierCurve;
use sketch::{Plane, Sketch};
use xad::material::BezierMaterialPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierMaterialPlugin))
        .add_systems(Startup, setup)
        .run();
}

const HALF_EXTENT: f32 = 1.6;

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 1.0, 4.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let left_curves: Vec<BezierCurve> = [
        BezierCurve::cubic(
            Vec2::new(0.10, 0.20),
            Vec2::new(0.40, 0.90),
            Vec2::new(0.60, 0.10),
            Vec2::new(0.90, 0.80),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(1.00, 0.30, 0.30, 1.0))
                .with_width(0.001)
        }),
        BezierCurve::cubic(
            Vec2::new(0.10, 0.85),
            Vec2::new(0.35, 0.65),
            Vec2::new(0.65, 0.35),
            Vec2::new(0.90, 0.15),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(1.00, 0.85, 0.10, 1.0))
                .with_width(0.001)
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    // Sketch takes care of the mesh + material; only the world offset is manual.
    commands
        .spawn(
            Sketch {
                position: Vec2::ZERO,
                plane: Plane::front(),
                curves: left_curves,
            }
            .into_bundle(),
        )
        .insert(Transform::from_translation(Vec3::new(
            -HALF_EXTENT,
            0.0,
            0.0,
        )));

    let bottom_curves: Vec<BezierCurve> = [
        BezierCurve::cubic(
            Vec2::new(0.10, 0.50),
            Vec2::new(0.30, 0.95),
            Vec2::new(0.70, 0.95),
            Vec2::new(0.90, 0.50),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(0.30, 1.00, 0.45, 1.0))
                .with_width(0.016)
        }),
        BezierCurve::cubic(
            Vec2::new(0.05, 0.50),
            Vec2::new(0.30, 0.10),
            Vec2::new(0.70, 0.90),
            Vec2::new(0.95, 0.50),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(0.20, 0.80, 1.00, 1.0))
                .with_width(0.011)
        }),
        BezierCurve::cubic(
            Vec2::new(0.10, 0.15),
            Vec2::new(0.40, 0.50),
            Vec2::new(0.60, 0.50),
            Vec2::new(0.90, 0.15),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(0.90, 0.40, 1.00, 1.0))
                .with_width(0.011)
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    commands
        .spawn(
            Sketch {
                position: Vec2::ZERO,
                plane: Plane::top(),
                curves: bottom_curves,
            }
            .into_bundle(),
        )
        .insert(Transform::from_translation(Vec3::new(
            0.0,
            -HALF_EXTENT,
            0.0,
        )));

    let right_curves: Vec<BezierCurve> = [
        BezierCurve::cubic(
            Vec2::new(0.10, 0.10),
            Vec2::new(0.90, 0.10),
            Vec2::new(0.10, 0.90),
            Vec2::new(0.90, 0.90),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(1.00, 0.55, 0.10, 1.0))
                .with_width(0.016)
        }),
        BezierCurve::cubic(
            Vec2::new(0.10, 0.90),
            Vec2::new(0.90, 0.90),
            Vec2::new(0.10, 0.10),
            Vec2::new(0.90, 0.10),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(1.00, 1.00, 1.00, 0.65))
                .with_width(0.009)
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    commands
        .spawn(
            Sketch {
                position: Vec2::ZERO,
                plane: Plane::right(),
                curves: right_curves,
            }
            .into_bundle(),
        )
        .insert(Transform::from_translation(Vec3::new(
            HALF_EXTENT,
            0.0,
            0.0,
        )));
}
