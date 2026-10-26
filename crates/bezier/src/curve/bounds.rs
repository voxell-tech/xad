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

    /// Smallest box containing both.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn corners(&self) -> [Vec2; 4] {
        [
            self.min,
            Vec2::new(self.max.x, self.min.y),
            self.max,
            Vec2::new(self.min.x, self.max.y),
        ]
    }

    /// Distance between the two boxes' nearest points; 0 if they touch
    /// or overlap.
    pub fn distance_to(&self, other: &Self) -> f32 {
        let gap = (self.min - other.max)
            .max(other.min - self.max)
            .max(Vec2::ZERO);
        gap.length()
    }
}

impl BezierCurve {
    /// Tight bounding box of the curve itself, not its control hull.
    pub fn aabb(&self) -> Aabb {
        let mut min = self.p0.min(self.p2);
        let mut max = self.p0.max(self.p2);
        // Per axis, the derivative crosses zero at t = (p0-p1)/(p0-2p1+p2).
        let denom = self.p0 - 2.0 * self.p1 + self.p2;
        let t = (self.p0 - self.p1) / denom;
        for axis in 0..2 {
            if denom[axis].abs() > 1e-12
                && t[axis] > 0.0
                && t[axis] < 1.0
            {
                let v = self.eval(t[axis])[axis];
                min[axis] = min[axis].min(v);
                max[axis] = max[axis].max(v);
            }
        }
        Aabb { min, max }
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
