//! Example: one plane with two cubic Bezier curves.
//!
//! The plane lies in the XZ plane (normal = +Y).

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

fn setup(mut commands: Commands) {
    // Angled top-down camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 3.5, 3.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let curves: Vec<BezierCurve> = [
        // Orange S-curve
        BezierCurve::cubic(
            Vec2::new(0.10, 0.20),
            Vec2::new(0.40, 0.90),
            Vec2::new(0.60, 0.10),
            Vec2::new(0.90, 0.80),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(1.00, 0.45, 0.10, 1.0))
                .with_width(0.005)
        }),
        // Cyan arch
        BezierCurve::cubic(
            Vec2::new(0.10, 0.50),
            Vec2::new(0.35, 0.95),
            Vec2::new(0.65, 0.95),
            Vec2::new(0.90, 0.50),
        )
        .map(|c| {
            c.with_color(LinearRgba::new(0.20, 0.80, 1.00, 1.0))
                .with_width(0.005)
        }),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Sketch takes care of the mesh + material; no manual Assets<Mesh> etc.
    commands.spawn(
        Sketch {
            position: Vec2::ZERO,
            plane: Plane::top(),
            curves,
        }
        .into_bundle(),
    );
}
