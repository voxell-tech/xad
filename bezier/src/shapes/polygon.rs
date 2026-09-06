use bevy_color::LinearRgba;
use bevy_math::Vec2;

use crate::BezierCurve;
use crate::shapes::line::line;

pub fn polygon(
    vertices: &[Vec2],
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    (0..vertices.len())
        .map(|i| {
            line(
                vertices[i],
                vertices[(i + 1) % vertices.len()],
                color,
                stroke_width,
            )
        })
        .collect()
}
