//! Example: a simple 2D sketch that draws some basic shapes (square, hexagon, a heart <3, and a circle)

use bevy::prelude::*;
use bezier::BezierCurve;
use bezier::gen_color;
use bezier::shapes::circle::circle;
use bezier::shapes::polygon::polygon as feature_polygon;
use bezier::shapes::square::square as feature_square;
use sketch::{Plane, Sketch};
use xad::material::BezierMaterialPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, BezierMaterialPlugin))
        .add_systems(Startup, setup)
        .run();
}

const LINE_WIDTH: f32 = 0.005;
const ENDPOINT_RADIUS: f32 = 0.015;

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
    [
        BezierCurve::cubic(
            tip,
            Vec2::new(0.05, 0.7),
            Vec2::new(0.05, 0.25),
            notch,
        ),
        BezierCurve::cubic(
            tip,
            Vec2::new(0.95, 0.7),
            Vec2::new(0.95, 0.25),
            notch,
        ),
    ]
    .into_iter()
    .flatten()
    .map(|c| {
        c.with_color(color)
            .with_width(LINE_WIDTH)
            .with_draw_endpoints(true)
            .with_endpoint_radius(0.015)
    })
    .collect()
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 9.0, 9.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let sketches = [
        (Vec2::new(3.75, 0.0), square(gen_color())),
        (Vec2::new(1.25, 0.0), hexagon(gen_color())),
        (Vec2::new(-1.25, 0.0), heart(gen_color())),
        (
            Vec2::new(-3.75, 0.0),
            circle(Vec2::splat(0.5), 0.32, gen_color(), LINE_WIDTH)
                .into_iter()
                .map(|c| {
                    c.with_draw_endpoints(true)
                        .with_endpoint_radius(ENDPOINT_RADIUS)
                })
                .collect(),
        ),
    ];

    for (position, curves) in sketches {
        commands.spawn(
            Sketch {
                position,
                plane: Plane::top(),
                curves,
            }
            .into_bundle(),
        );
    }
}
