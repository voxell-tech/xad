use bevy::prelude::*;
use bezier::{BezierCurve, Bvh, Polarity};
use sketch::Sketch;

use super::frame::PlaneFrame;

/// Whether the per-edge face labels are shown. Toggled with L.
#[derive(Resource)]
pub struct ShowPolarityLabels(pub bool);

impl Default for ShowPolarityLabels {
    fn default() -> Self {
        Self(true)
    }
}

pub(super) fn toggle_labels(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<ShowPolarityLabels>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        show.0 = !show.0;
    }
}

/// Whether the fill shader tints by nearest edge instead of face color.
/// Toggled with R.
#[derive(Resource, Default)]
pub struct ShowEdgeTint(pub bool);

pub(super) fn toggle_edge_tint(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<ShowEdgeTint>,
    mut sketches: Query<&mut Sketch>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        show.0 = !show.0;
        // The flag lives in each fill buffer; rebuild them.
        for mut sketch in &mut sketches {
            sketch.set_changed();
        }
    }
}

/// Whether curve bounding boxes and the BVH over them are drawn.
/// Toggled with B.
#[derive(Resource, Default)]
pub struct ShowBvh(pub bool);

pub(super) fn toggle_bvh(
    keys: Res<ButtonInput<KeyCode>>,
    mut show: ResMut<ShowBvh>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        show.0 = !show.0;
    }
}

/// BVH over a sketch's intersection-split curves.
#[derive(Component)]
pub(super) struct SketchBvh(pub Bvh);

// Gizmo offset in front of the fill layer, and the extra inflation each
// level gets over the one below so coincident box edges stay visible.
const BVH_LAYER_OFFSET: f32 = 0.002;
const LEVEL_PAD: f32 = 0.006;

pub(super) fn draw_bvh(
    show: Res<ShowBvh>,
    mut gizmos: Gizmos,
    sketches: Query<(&Sketch, &SketchBvh)>,
) {
    if !show.0 {
        return;
    }
    for (sketch, bvh) in &sketches {
        let nodes = &bvh.0.nodes;
        if nodes.is_empty() {
            continue;
        }
        let (tangent, bitangent) = sketch.plane_basis();
        let origin = sketch.plane.normal() * BVH_LAYER_OFFSET;
        let world = |p: Vec2| {
            origin + tangent * p.x + bitangent * p.y
        };

        // Median splits keep the tree balanced, so the deepest level is
        // ceil(log2(leaves)). Leaves draw their exact curve AABB; each
        // level above is inflated one step further outward.
        let leaves = (nodes.len() + 1) / 2;
        let max_depth =
            leaves.next_power_of_two().trailing_zeros();

        let mut stack = vec![(0u32, 0u32)];
        while let Some((index, depth)) = stack.pop() {
            let node = nodes[index as usize];
            let aabb = node
                .aabb
                .padded(max_depth.saturating_sub(depth) as f32 * LEVEL_PAD);
            let color =
                Color::hsl(depth as f32 * 137.5 % 360.0, 0.9, 0.6);
            gizmos.linestrip(
                [
                    world(aabb.min),
                    world(Vec2::new(aabb.max.x, aabb.min.y)),
                    world(aabb.max),
                    world(Vec2::new(aabb.min.x, aabb.max.y)),
                    world(aabb.min),
                ],
                color,
            );
            if let bezier::BvhKind::Interior { right } = node.kind {
                stack.push((index + 1, depth + 1));
                stack.push((right, depth + 1));
            }
        }
    }
}

/// Plane-local distance from an edge's midpoint to each of its two labels.
const LABEL_OFFSET: f32 = 0.07;

/// A debug label pinned to a world-space point.
#[derive(Component)]
pub(super) struct PolarityLabel {
    position: Vec3,
}

pub(super) fn spawn_labels(
    commands: &mut Commands,
    curves: &[BezierCurve],
    polarity: &[Polarity],
    frame: PlaneFrame,
) -> Vec<Entity> {
    let world = |local: Vec2| {
        frame.origin
            + frame.tangent * local.x
            + frame.bitangent * local.y
    };
    let text = |side: Option<usize>| match side {
        Some(face) => face.to_string(),
        None => "N".to_string(),
    };

    let mut labels = Vec::new();
    for (curve, sides) in curves.iter().zip(polarity) {
        let mid = curve.eval(0.5);
        let chord = curve.p2 - curve.p0;
        let dir = if chord.length_squared() > 1e-12 {
            chord
        } else {
            curve.p1 - curve.p0
        }
        .normalize_or_zero();
        // Left of travel.
        let normal = Vec2::new(-dir.y, dir.x);

        for (side, offset) in
            [(sides.left, normal), (sides.right, -normal)]
        {
            labels.push(
                commands
                    .spawn((
                        PolarityLabel {
                            position: world(
                                mid + offset * LABEL_OFFSET,
                            ),
                        },
                        Text::new(text(side)),
                        TextFont::from_font_size(12.0),
                        TextColor(Color::WHITE),
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                    ))
                    .id(),
            );
        }
    }
    labels
}

/// Keeps each label over its world-space point.
pub(super) fn position_labels(
    show: Res<ShowPolarityLabels>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut labels: Query<(&PolarityLabel, &mut Node, &mut Visibility)>,
) {
    if !show.0 {
        for (_, _, mut visibility) in &mut labels {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    for (label, mut node, mut visibility) in &mut labels {
        match camera
            .world_to_viewport(camera_transform, label.position)
        {
            Ok(p) => {
                node.left = Val::Px(p.x);
                node.top = Val::Px(p.y);
                *visibility = Visibility::Visible;
            }
            Err(_) => *visibility = Visibility::Hidden,
        }
    }
}
