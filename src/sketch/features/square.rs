use bevy::prelude::*;

use crate::bezier::BezierCurve;
use crate::sketch::features::rectangle::rectangle;

// axis aligned
pub fn square(
    center: Vec2,
    side: f32,
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    rectangle(center, Vec2::splat(side), color, stroke_width)
}
