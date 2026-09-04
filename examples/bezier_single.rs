//! Example: one plane with two cubic Bezier curves.
//!
//! The plane lies in the XZ plane (normal = +Y).
//! Control points are in UV space: (0,0) = one corner, (1,1) = opposite corner.

use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use xad::bezier::{BezierCurve, BezierMaterial, BezierPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    // Angled top-down camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 3.5, 3.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let material = materials.add(BezierMaterial::from_curves(
        LinearRgba::new(0.06, 0.06, 0.10, 1.0),
        [
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
        .collect(),
        &mut storage_buffers,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(2.5)))),
        MeshMaterial3d(material),
    ));
}
