use bevy::prelude::*;
use bezier::{BezierCurve, Polarity};

use super::frame::PlaneFrame;

/// Label each side of every edge with the face id it borders (N = none),
/// as screen-space text pinned over the sketch.
pub(super) const POLARITY_LABELS: bool = true;

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
    camera: Query<(&Camera, &GlobalTransform)>,
    mut labels: Query<(&PolarityLabel, &mut Node, &mut Visibility)>,
) {
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
