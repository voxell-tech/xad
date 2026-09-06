//! Builds a scene using the timeline instead of manual command.spawn calls

use bevy::prelude::*;
use command_system::Timeline;
use sketch::features::CreateCircle;
use sketch::{CreateSketch, Plane};
use xad::XadPlugin;
use xad::sdf::SdfCamera;
use xad::sdf::boolean::BooleanSubtract;
use xad::sdf::extrude::BlindExtrude;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, XadPlugin));
    app.add_systems(Startup, (setup_camera, run_timeline));
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(-5.0, 5.0, -8.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
        SdfCamera::default(),
    ));
}

fn run_timeline(world: &mut World) {
    let mut timeline: Timeline<World, Entity> = Timeline::new();

    let sketch = timeline.push(CreateSketch {
        plane: Plane::top(),
        position: Vec2::new(2.0, 3.0),
    });

    let circle_a = timeline.push(CreateCircle {
        sketch,
        position: Vec2::ZERO,
        radius: 0.6,
    });

    let circle_b = timeline.push(CreateCircle {
        sketch,
        position: Vec2::ZERO,
        radius: 0.3,
    });

    let solid_a = timeline.push(BlindExtrude {
        face: circle_a,
        distance: 0.5,
    });

    let solid_b = timeline.push(BlindExtrude {
        face: circle_b,
        distance: 0.9,
    });

    timeline.push(BooleanSubtract {
        target: solid_a,
        tool: solid_b,
    });

    timeline.regen(world);

    world.flush();
}
