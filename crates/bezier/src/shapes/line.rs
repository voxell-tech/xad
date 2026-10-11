use bevy_color::LinearRgba;
use bevy_math::Vec2;

use crate::BezierCurve;

pub fn line(
    a: Vec2,
    b: Vec2,
    color: LinearRgba,
    stroke_width: f32,
) -> BezierCurve {
    BezierCurve::new(a, a.lerp(b, 0.5), b)
        .with_color(color)
        .with_width(stroke_width)
}
