//! Example: three planes with several cubic Bezier curves each.
//!
//! Bezier curves are drawn on planes, that can be oriented in
//! whatever direction.

use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bezier::BezierCurve;
use sketch::Sketch;
use xad::material::{
    BezierMaterial, BezierMaterialPlugin, construct_bezier_material,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierMaterialPlugin))
        .add_systems(Startup, setup)
        .run();
}

const HALF_EXTENT: f32 = 1.6;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 1.0, 4.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let left = Sketch {
        curves: [
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
        .collect(),
        position: Vec3::new(-HALF_EXTENT, 0.0, 0.0),
        background_color: LinearRgba::new(0.05, 0.04, 0.10, 1.0),
    };
    let mesh_left =
        meshes.add(Plane3d::new(Vec3::X, Vec2::splat(HALF_EXTENT)));
    let mat_left = materials
        .add(construct_bezier_material(&left, &mut storage_buffers));
    commands.spawn((
        Mesh3d(mesh_left),
        MeshMaterial3d(mat_left),
        Transform::from_translation(left.position),
    ));

    let bottom = Sketch {
        curves: [
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
        .collect(),
        position: Vec3::new(0.0, -HALF_EXTENT, 0.0),
        background_color: LinearRgba::new(0.04, 0.10, 0.06, 1.0),
    };
    let mesh_bottom =
        meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(HALF_EXTENT)));
    let mat_bottom = materials.add(construct_bezier_material(
        &bottom,
        &mut storage_buffers,
    ));
    commands.spawn((
        Mesh3d(mesh_bottom),
        MeshMaterial3d(mat_bottom),
        Transform::from_translation(bottom.position),
    ));

    let right = Sketch {
        curves: [
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
        .collect(),
        position: Vec3::new(HALF_EXTENT, 0.0, 0.0),
        background_color: LinearRgba::new(0.10, 0.06, 0.04, 1.0),
    };
    let mesh_right = meshes
        .add(Plane3d::new(Vec3::NEG_X, Vec2::splat(HALF_EXTENT)));
    let mat_right = materials
        .add(construct_bezier_material(&right, &mut storage_buffers));
    commands.spawn((
        Mesh3d(mesh_right),
        MeshMaterial3d(mat_right),
        Transform::from_translation(right.position),
    ));
}
