use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::bezier::BezierCurve;
use crate::sketch::features::arc::arc;

pub fn circle(
    center: Vec2,
    radius: f32,
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    arc(center, radius, 0.0, TAU, color, stroke_width)
}
