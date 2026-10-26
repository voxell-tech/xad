use bevy::prelude::*;
use sketch::Sketch;

use crate::bounds::Bounds;
use crate::solid::{Extent, Solid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    Union,
    Subtract,
    Intersect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Node {
    /// Index into [`Volume::solids`].
    Solid(u32),
    Op(BoolOp),
}

/// A body, as a postfix expression over extruded faces.
///
/// A [`Node::Solid`] pushes its distance, a [`Node::Op`] combines the
/// top two. Postfix makes combining two bodies a concatenation, so
/// [`combine`](Self::combine) mutates the body in place and arbitrary
/// nesting stays correct without a boxed tree.
#[derive(Component, Clone, Debug)]
pub struct Volume {
    pub nodes: Vec<Node>,
    pub solids: Vec<Solid>,
}

impl Volume {
    pub fn new(solid: Solid) -> Self {
        Self {
            nodes: vec![Node::Solid(0)],
            solids: vec![solid],
        }
    }

    pub fn from_face(
        sketch: &Sketch,
        face: usize,
        extent: Extent,
    ) -> Option<Self> {
        Solid::from_face(sketch, face, extent).map(Self::new)
    }

    pub fn union(&mut self, other: Self) {
        self.combine(other, BoolOp::Union);
    }

    pub fn subtract(&mut self, other: Self) {
        self.combine(other, BoolOp::Subtract);
    }

    pub fn intersect(&mut self, other: Self) {
        self.combine(other, BoolOp::Intersect);
    }

    /// Appends `other`'s expression and the op that joins them. Taking
    /// `other` by value spends it: a tool body cannot be reused.
    pub fn combine(&mut self, other: Self, op: BoolOp) {
        let base = self.solids.len() as u32;
        self.solids.extend(other.solids);
        self.nodes.extend(other.nodes.into_iter().map(|node| {
            match node {
                Node::Solid(index) => Node::Solid(index + base),
                Node::Op(op) => Node::Op(op),
            }
        }));
        self.nodes.push(Node::Op(op));
    }

    pub fn into_bundle(self) -> impl Bundle {
        // The render proxy spawns as a child, which needs both to
        // inherit a world position.
        (self, Transform::default(), Visibility::default())
    }

    /// Operand stack the postfix form needs to evaluate.
    pub fn stack_depth(&self) -> usize {
        let mut depth = 0usize;
        let mut peak = 0usize;
        for node in &self.nodes {
            match node {
                Node::Solid(_) => {
                    depth += 1;
                    peak = peak.max(depth);
                }
                Node::Op(_) => depth = depth.saturating_sub(1),
            }
        }
        peak
    }

    /// World bounds of the result, or `None` if it is empty.
    ///
    /// Subtracting cannot grow the target and intersecting cannot leave
    /// the overlap, so folding the leaves' boxes bounds the body.
    pub fn bounds(&self) -> Option<Bounds> {
        let mut stack = Vec::with_capacity(self.stack_depth());
        for node in &self.nodes {
            match node {
                Node::Solid(index) => {
                    stack.push(self.solids[*index as usize].bounds())
                }
                Node::Op(op) => {
                    let tool = stack.pop()?;
                    let target = stack.pop()?;
                    stack.push(match op {
                        BoolOp::Union => target.merge(tool),
                        BoolOp::Subtract => target,
                        BoolOp::Intersect => target.intersect(tool)?,
                    });
                }
            }
        }
        stack.pop()
    }
}

#[cfg(test)]
mod tests {
    use bezier::BezierCurve;
    use sketch::Plane;

    use super::*;

    // Square profile of `size` centred on `center`, as a sketch.
    fn square(plane: Plane, center: Vec2, size: f32) -> Sketch {
        let h = size * 0.5;
        let corners = [
            center + Vec2::new(-h, -h),
            center + Vec2::new(h, -h),
            center + Vec2::new(h, h),
            center + Vec2::new(-h, h),
        ];
        Sketch {
            plane,
            curves: (0..4)
                .map(|i| {
                    let (a, b) = (corners[i], corners[(i + 1) % 4]);
                    BezierCurve::new(a, a.lerp(b, 0.5), b)
                })
                .collect(),
        }
    }

    fn volume(center: Vec2, size: f32, extent: Extent) -> Volume {
        Volume::from_face(
            &square(Plane::right(), center, size),
            0,
            extent,
        )
        .expect("square encloses a face")
    }

    fn leaf() -> Volume {
        volume(Vec2::ZERO, 1.0, Extent::symmetric(1.0))
    }

    #[test]
    fn combine_rebases_the_tool_indices() {
        let mut a = leaf();
        a.union(leaf());
        let mut b = leaf();
        b.subtract(leaf());

        a.combine(b, BoolOp::Subtract);

        assert_eq!(a.solids.len(), 4);
        assert_eq!(
            a.nodes,
            vec![
                Node::Solid(0),
                Node::Solid(1),
                Node::Op(BoolOp::Union),
                Node::Solid(2),
                Node::Solid(3),
                Node::Op(BoolOp::Subtract),
                Node::Op(BoolOp::Subtract),
            ],
        );
    }

    #[test]
    fn nesting_differs_from_chaining() {
        // a - (b - c)
        let mut nested = leaf();
        let mut inner = leaf();
        inner.subtract(leaf());
        nested.subtract(inner);

        // (a - b) - c
        let mut chained = leaf();
        chained.subtract(leaf());
        chained.subtract(leaf());

        assert_ne!(nested.nodes, chained.nodes);
        assert_eq!(nested.stack_depth(), 3);
        assert_eq!(chained.stack_depth(), 2);
    }

    #[test]
    fn union_bounds_cover_both() {
        let mut body =
            volume(Vec2::ZERO, 1.0, Extent::symmetric(1.0));
        body.union(volume(
            Vec2::new(4.0, 0.0),
            1.0,
            Extent::symmetric(1.0),
        ));

        let bounds = body.bounds().unwrap();
        assert!(bounds.min.x < -0.5);
        assert!(bounds.max.x > 4.0);
    }

    #[test]
    fn subtract_keeps_the_target_bounds() {
        let target =
            volume(Vec2::ZERO, 1.0, Extent::symmetric(1.0));
        let expected = target.bounds().unwrap();

        let mut body = target;
        body.subtract(volume(
            Vec2::new(4.0, 0.0),
            1.0,
            Extent::symmetric(1.0),
        ));

        assert_eq!(body.bounds().unwrap(), expected);
    }

    #[test]
    fn intersect_bounds_shrink_to_the_overlap() {
        let mut body =
            volume(Vec2::ZERO, 2.0, Extent::symmetric(1.0));
        body.intersect(volume(
            Vec2::new(1.0, 0.0),
            2.0,
            Extent::symmetric(1.0),
        ));

        let bounds = body.bounds().unwrap();
        assert!(bounds.min.x > -1.0);
        assert!(bounds.max.x < 2.0);
    }

    #[test]
    fn disjoint_intersection_is_empty() {
        let mut body =
            volume(Vec2::ZERO, 1.0, Extent::symmetric(1.0));
        body.intersect(volume(
            Vec2::new(40.0, 0.0),
            1.0,
            Extent::symmetric(1.0),
        ));

        assert!(body.bounds().is_none());
    }
}
