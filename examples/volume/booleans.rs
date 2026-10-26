//! Example: boolean operations between two perpendicular sketches.
//!
//! A gear profile on one plane and a six-pointed star on a plane at
//! right angles to it are each swept into a body, then folded together
//! three ways: union, intersection, difference.
//!
//! Each boolean mutates its target in place instead of making a fourth
//! body, so the `Extrude` id and the `Boolean` id name one entity and
//! nothing has to be hidden.
//!
//! WASDQE to move, right-click drag to look.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bezier::BezierCurve;
use command_system::Timeline;
use sketch::features::CreateProfile;
use sketch::{CreateSketch, Plane};
use volume::features::{Boolean, Extrude};
use volume::{BoolOp, Extent};
use xad::camera::{CameraController, CameraPlugin};
use xad::material::{
    SketchRenderPlugin, VolumeColor, VolumeMaterialPlugin,
};

// Spacing between the three pairs, along the axis both planes share.
const SPACING: f32 = 1.5;
// Both sweeps straddle their plane, so the bodies actually overlap.
const DEPTH: f32 = 1.5;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CameraPlugin,
            VolumeMaterialPlugin,
            SketchRenderPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn line(a: Vec2, b: Vec2) -> BezierCurve {
    BezierCurve::new(a, (a + b) * 0.5, b)
}

/// Closed loop of lines through `points`.
fn loop_of(points: &[Vec2]) -> Vec<BezierCurve> {
    (0..points.len())
        .map(|i| line(points[i], points[(i + 1) % points.len()]))
        .collect()
}

/// Circle as four quadrant arcs.
fn circle(center: Vec2, radius: f32) -> Vec<BezierCurve> {
    // Control distance putting each arc's midpoint on the circle.
    let control = radius * (2.0 * 0.5f32.sqrt() - 0.5);
    let axes = [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y];
    (0..4)
        .map(|i| {
            let (from, to) = (axes[i], axes[(i + 1) % 4]);
            BezierCurve::new(
                center + from * radius,
                center + (from + to) * control,
                center + to * radius,
            )
        })
        .collect()
}

/// Toothed rim with a bore: flat flanks between root and tip radius.
fn gear(center: Vec2) -> Vec<BezierCurve> {
    let (teeth, root, tip) = (10, 0.38, 0.52);
    let pitch = TAU / teeth as f32;
    let vertex = |tooth: usize, fraction: f32, radius: f32| {
        let angle = (tooth as f32 + fraction) * pitch;
        center + radius * Vec2::new(angle.cos(), angle.sin())
    };
    let mut points = Vec::new();
    for tooth in 0..teeth {
        points.push(vertex(tooth, 0.1, root));
        points.push(vertex(tooth, 0.2, tip));
        points.push(vertex(tooth, 0.4, tip));
        points.push(vertex(tooth, 0.5, root));
    }
    loop_of(&points)
        .into_iter()
        .chain(circle(center, 0.15))
        .collect()
}

/// Six-pointed star with a bore.
fn star(center: Vec2) -> Vec<BezierCurve> {
    let (tips, inner, outer) = (6u32, 0.24, 0.56);
    let step = TAU / (tips * 2) as f32;
    let points = (0..tips * 2)
        .map(|i| {
            let angle = i as f32 * step;
            let radius =
                if i.is_multiple_of(2) { outer } else { inner };
            center + radius * Vec2::new(angle.cos(), angle.sin())
        })
        .collect::<Vec<_>>();
    loop_of(&points)
        .into_iter()
        .chain(circle(center, 0.1))
        .collect()
}

fn setup(world: &mut World) {
    world.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(2.84, 2.07, 4.92)),
        CameraController {
            speed: 2.0,
            yaw: 30.0,
            pitch: -20.0,
            ..default()
        },
    ));

    let ops = [
        (BoolOp::Union, LinearRgba::new(0.95, 0.62, 0.22, 1.0)),
        (BoolOp::Intersect, LinearRgba::new(0.35, 0.78, 0.95, 1.0)),
        (BoolOp::Subtract, LinearRgba::new(0.88, 0.33, 0.52, 1.0)),
    ];

    let mut timeline = Timeline::<World, Entity>::new();
    let mut bodies = Vec::new();

    for (row, (op, _)) in ops.iter().enumerate() {
        // Both planes take world Y as their second basis vector, so a
        // single offset moves a whole pair together.
        let center = Vec2::new(0.0, (1.0 - row as f32) * SPACING);

        // Gear on the Z-normal plane, swept along world Z.
        let gear_sketch = timeline.push(CreateSketch {
            plane: Plane::right(),
        });
        timeline.push(CreateProfile {
            sketch: gear_sketch,
            curves: gear(center),
        });

        // Star on the X-normal plane, swept along world X.
        let star_sketch = timeline.push(CreateSketch {
            plane: Plane::front(),
        });
        timeline.push(CreateProfile {
            sketch: star_sketch,
            curves: star(center),
        });

        let gear_body = timeline.push(Extrude {
            sketch: gear_sketch,
            face: 0,
            extent: Extent::symmetric(DEPTH),
        });
        let star_body = timeline.push(Extrude {
            sketch: star_sketch,
            face: 0,
            extent: Extent::symmetric(DEPTH),
        });

        bodies.push(timeline.push(Boolean {
            target: gear_body,
            tool: star_body,
            op: *op,
        }));
    }

    timeline.regen(world);

    for (body, (_, color)) in bodies.iter().zip(&ops) {
        // The boolean reports the target it folded into, so this is the
        // same entity the extrude produced.
        if let Some(&entity) = timeline.output(*body) {
            world.entity_mut(entity).insert(VolumeColor(*color));
        }
    }

    world.insert_non_send(timeline);
}
