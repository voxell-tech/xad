use bevy::prelude::*;
use bezier::{BezierCurve, Polarity};
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
