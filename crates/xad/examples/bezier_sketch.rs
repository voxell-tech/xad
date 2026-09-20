//! Example: a simple 2D sketch that draws some basic shapes (square, hexagon, a heart <3, and a circle)

use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use xad::bezier::{BezierCurve, BezierMaterial, BezierPlugin};
use xad::sketch::color::gen_color;
use xad::sketch::features::circle::circle;
use xad::sketch::features::line::line as feature_line;
use xad::sketch::features::polygon::polygon as feature_polygon;
use xad::sketch::features::square::square as feature_square;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierPlugin))
        .add_systems(Startup, setup)
        .run();
}

const HALF_EXTENT: f32 = 1.0;
const LINE_WIDTH: f32 = 0.005;
const ENDPOINT_RADIUS: f32 = 0.015;

// Wraps the shared `line` helper with this example's endpoint-dot styling.
fn line(a: Vec2, b: Vec2, color: LinearRgba) -> BezierCurve {
    feature_line(a, b, color, LINE_WIDTH)
        .with_draw_endpoints(true)
        .with_endpoint_radius(ENDPOINT_RADIUS)
}

fn square(color: LinearRgba) -> Vec<BezierCurve> {
    feature_square(Vec2::splat(0.5), 0.6, color, LINE_WIDTH)
        .into_iter()
        .map(|c| {
            c.with_draw_endpoints(true)
                .with_endpoint_radius(ENDPOINT_RADIUS)
        })
        .collect()
}

fn hexagon(color: LinearRgba) -> Vec<BezierCurve> {
    let center = Vec2::new(0.5, 0.5);
    let radius = 0.35;
    let vertices: Vec<Vec2> = (0..6)
        .map(|i| {
            // i/6 * 2PI + (PI/2)
            // PI/2 offset is so that its a flat top hexagon
            let angle = std::f32::consts::TAU * (i as f32) / 6.0
                + std::f32::consts::FRAC_PI_2;
            center + radius * Vec2::new(angle.cos(), angle.sin())
        })
        .collect();
    feature_polygon(&vertices, color, LINE_WIDTH)
}

// Two mirrored cubic curves sharing a bottom tip and a top notch.
fn heart(color: LinearRgba) -> Vec<BezierCurve> {
    let tip = Vec2::new(0.5, 0.85);
    let notch = Vec2::new(0.5, 0.4);
    vec![
        BezierCurve::new(
            tip,
            Vec2::new(0.05, 0.7),
            Vec2::new(0.05, 0.25),
            notch,
        )
        .with_color(color)
        .with_width(LINE_WIDTH)
        .with_draw_endpoints(true)
        .with_endpoint_radius(0.015),
        BezierCurve::new(
            tip,
            Vec2::new(0.95, 0.7),
            Vec2::new(0.95, 0.25),
            notch,
        )
        .with_color(color)
        .with_width(LINE_WIDTH)
        .with_draw_endpoints(true)
        .with_endpoint_radius(0.015),
    ]
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BezierMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 9.0, 9.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let background = LinearRgba::new(0.06, 0.06, 0.09, 1.0);
    let mesh =
        meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(HALF_EXTENT)));

    let shapes = [
        (Vec3::new(-3.75, 0.0, 0.0), square(gen_color())),
        (Vec3::new(-1.25, 0.0, 0.0), hexagon(gen_color())),
        (Vec3::new(1.25, 0.0, 0.0), heart(gen_color())),
        (
            Vec3::new(3.75, 0.0, 0.0),
            circle(Vec2::splat(0.5), 0.32, gen_color(), LINE_WIDTH)
                .into_iter()
                .map(|c| {
                    c.with_draw_endpoints(true)
                        .with_endpoint_radius(ENDPOINT_RADIUS)
                })
                .collect(),
        ),
    ];

    for (translation, curves) in shapes {
        let material = materials.add(BezierMaterial::from_curves(
            background,
            curves,
            &mut storage_buffers,
        ));
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(translation),
        ));
    }
}
