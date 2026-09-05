use bevy_color::LinearRgba;
use bevy_math::Vec2;

use crate::BezierCurve;
use crate::shapes::line::line;

// axis aligned
pub fn rectangle(
    center: Vec2,
    size: Vec2,
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    let half = size * 0.5;
    let corners = [
        center + Vec2::new(-half.x, -half.y),
        center + Vec2::new(half.x, -half.y),
        center + Vec2::new(half.x, half.y),
        center + Vec2::new(-half.x, half.y),
    ];
    (0..corners.len())
        .map(|i| {
            line(
                corners[i],
                corners[(i + 1) % corners.len()],
                color,
                stroke_width,
            )
        })
        .collect()
}
