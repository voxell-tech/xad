//! Example: interactively draw cubic Bezier curves with the command system.
//!
//! - A sketch is created on a camera-facing plane via a `Timeline`.
//! - The cursor is raycast from the screen onto the sketch plane.
//! - Left-click on empty space drops a control point (a draggable sphere).
//! - Every 4 points appends a `CreateCurve` command and regenerates the sketch.
//! - Dragging any sphere moves its point and regenerates the timeline.
//!
//! All bevy/input logic lives here and in the `xad` crate; the `sketch` and
//! `bezier` crates stay free of windowing/input concerns.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use command_system::Timeline;
use sketch::features::CreateCurve;
use sketch::{CreateSketch, Plane};
use xad::material::SketchRenderPlugin;

const POINT_RADIUS: f32 = 0.04;
const POINT_Z: f32 = 0.05; // in front of the sketch plane (z = 0)
const PICK_RADIUS: f32 = 0.12;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SketchRenderPlugin))
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, interact)
        .run();
}

// Persistent editor state. Held as a non-send resource because the command
// `Timeline` stores boxed cleanup closures over `&mut World`.
struct Editor {
    timeline: Timeline<World, Entity>,
    points: Vec<Vec2>,
    point_entities: Vec<Entity>,
    dragging: Option<usize>,
    sphere: Handle<Mesh>,
    idle: Handle<StandardMaterial>,
    active: Handle<StandardMaterial>,
}

fn setup(world: &mut World) {
    world.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let sphere = world
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(POINT_RADIUS));
    let idle = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(unlit(Color::srgb(0.9, 0.92, 1.0)));
    let active = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(unlit(Color::srgb(1.0, 0.85, 0.2)));

    let mut editor = Editor {
        timeline: Timeline::new(),
        points: Vec::new(),
        point_entities: Vec::new(),
        dragging: None,
        sphere,
        idle,
        active,
    };

    // Create the (empty) sketch up front.
    rebuild(&mut editor, world);
    world.insert_non_send(editor);

    info!(
        "Left-click to place control points; every 4 make a curve. Drag points to edit. L toggles face labels."
    );
}

fn unlit(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        unlit: true,
        ..default()
    }
}

fn interact(world: &mut World) {
    let hit = cursor_on_plane(world);

    let (pressed, held, released) = {
        let mouse = world.resource::<ButtonInput<MouseButton>>();
        (
            mouse.just_pressed(MouseButton::Left),
            mouse.pressed(MouseButton::Left),
            mouse.just_released(MouseButton::Left),
        )
    };

    let mut editor = world.remove_non_send::<Editor>().unwrap();
    let mut changed = false;

    if let Some(hit) = hit {
        if pressed {
            match nearest_point(&editor.points, hit) {
                Some(index) => editor.dragging = Some(index),
                None => {
                    let entity = spawn_point(world, &editor, hit);
                    editor.points.push(hit);
                    editor.point_entities.push(entity);
                    // A point placed may complete a 4-point curve.
                    changed = true;
                }
            }
        }

        if held
            && let Some(index) = editor.dragging
            && editor.points[index] != hit
        {
            editor.points[index] = hit;
            changed = true;
        }
    }

    if released {
        editor.dragging = None;
    }

    update_point_visuals(world, &editor);

    if changed {
        rebuild(&mut editor, world);
    }

    world.insert_non_send(editor);
}

// Rebuilds the timeline from the current points and regenerates the sketch:
// one `CreateCurve` per completed group of four control points.
fn rebuild(editor: &mut Editor, world: &mut World) {
    editor.timeline.cleanup(world);

    let mut timeline = Timeline::new();
    let sketch = timeline.push(CreateSketch {
        plane: Plane::right(),
    });
    for group in editor.points.chunks_exact(4) {
        timeline.push(CreateCurve {
            sketch,
            points: [group[0], group[1], group[2], group[3]],
        });
    }
    timeline.regen(world);

    editor.timeline = timeline;
}

fn spawn_point(
    world: &mut World,
    editor: &Editor,
    point: Vec2,
) -> Entity {
    world
        .spawn((
            Mesh3d(editor.sphere.clone()),
            MeshMaterial3d(editor.idle.clone()),
            Transform::from_translation(Vec3::new(
                point.x, point.y, POINT_Z,
            )),
        ))
        .id()
}

fn update_point_visuals(world: &mut World, editor: &Editor) {
    for (i, &entity) in editor.point_entities.iter().enumerate() {
        if let Some(mut transform) =
            world.get_mut::<Transform>(entity)
        {
            transform.translation = Vec3::new(
                editor.points[i].x,
                editor.points[i].y,
                POINT_Z,
            );
        }
        let material = if editor.dragging == Some(i) {
            editor.active.clone()
        } else {
            editor.idle.clone()
        };
        world.entity_mut(entity).insert(MeshMaterial3d(material));
    }
}

fn nearest_point(points: &[Vec2], hit: Vec2) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.distance_squared(hit)))
        .filter(|(_, d)| *d <= PICK_RADIUS * PICK_RADIUS)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

// Raycasts the cursor onto the z = 0 sketch plane, returning local coordinates.
fn cursor_on_plane(world: &mut World) -> Option<Vec2> {
    let cursor = {
        let mut query =
            world.query_filtered::<&Window, With<PrimaryWindow>>();
        query.iter(world).next()?.cursor_position()?
    };

    let (origin, direction) = {
        let mut query = world.query::<(&Camera, &GlobalTransform)>();
        let (camera, transform) = query.iter(world).next()?;
        let ray = camera.viewport_to_world(transform, cursor).ok()?;
        (ray.origin, *ray.direction)
    };

    if direction.z.abs() < 1e-6 {
        return None;
    }
    let t = -origin.z / direction.z;
    if t < 0.0 {
        return None;
    }
    let hit = origin + direction * t;
    Some(Vec2::new(hit.x, hit.y))
}
