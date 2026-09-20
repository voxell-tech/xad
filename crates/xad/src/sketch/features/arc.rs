use std::f32::consts::PI;

use bevy::prelude::*;

use crate::bezier::BezierCurve;

const MAX_SEGMENT_SWEEP: f32 = PI / 2.0;

pub fn arc(
    center: Vec2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: LinearRgba,
    stroke_width: f32,
) -> Vec<BezierCurve> {
    let sweep = end_angle - start_angle;
    let segment_count =
        (sweep.abs() / MAX_SEGMENT_SWEEP).ceil().max(1.0) as usize;
    let segment_sweep = sweep / segment_count as f32;

    (0..segment_count)
        .map(|i| {
            let a0 = start_angle + segment_sweep * i as f32;
            let a1 = a0 + segment_sweep;
            arc_segment(center, radius, a0, a1, color, stroke_width)
        })
        .collect()
}

fn arc_segment(
    center: Vec2,
    radius: f32,
    a0: f32,
    a1: f32,
    color: LinearRgba,
    stroke_width: f32,
) -> BezierCurve {
    let p0 = center + radius * Vec2::new(a0.cos(), a0.sin());
    let p3 = center + radius * Vec2::new(a1.cos(), a1.sin());
    let handle = (4.0 / 3.0) * ((a1 - a0) / 4.0).tan() * radius;
    let p1 = p0 + handle * Vec2::new(-a0.sin(), a0.cos());
    let p2 = p3 - handle * Vec2::new(-a1.sin(), a1.cos());

    BezierCurve::new(p0, p1, p2, p3)
        .with_color(color)
        .with_width(stroke_width)
}
