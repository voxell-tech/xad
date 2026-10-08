use bevy_math::Vec2;

use super::BezierCurve;

/// Axis-aligned bounding box in a sketch's plane-local coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    // corner to corner repr of a bounding box
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    // Offset the min and max by `margin`
    pub fn padded(&self, margin: f32) -> Self {
        let m = Vec2::splat(margin);
        Self {
            min: self.min - m,
            max: self.max + m,
        }
    }
}

/// Bounding box of the curves' control-point hulls
pub fn bounding_box(curves: &[BezierCurve]) -> Option<Aabb> {
    curves.iter().fold(None, |acc, c| {
        let lo = c.p0.min(c.p1).min(c.p2);
        let hi = c.p0.max(c.p1).max(c.p2);
        Some(match acc {
            Some(a) => Aabb {
                min: a.min.min(lo),
                max: a.max.max(hi),
            },
            None => Aabb { min: lo, max: hi },
        })
    })
}
