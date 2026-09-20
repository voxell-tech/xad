use bevy::color::LinearRgba;
use bevy::math::Vec2;

use crate::bezier::BezierCurve;

pub fn polygon(
    vertices: &[Vec2],
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    (0..vertices.len())
        .map(|i| {
            crate::sketch::features::line::line(
                vertices[i],
                vertices[(i + 1) % vertices.len()],
                color,
                stroke_width,
            )
        })
        .collect()
}
