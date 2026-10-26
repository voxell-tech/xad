use bevy_math::Vec2;

use crate::curve::{Aabb, BezierCurve};

/// What a [`BvhNode`] holds.
#[derive(Clone, Copy, Debug)]
pub enum BvhKind {
    /// Index of the curve this leaf bounds.
    Leaf(u32),
    /// Index of the right child; the left child is the next node.
    Interior { right: u32 },
}

/// Node of a [`Bvh`].
#[derive(Clone, Copy, Debug)]
pub struct BvhNode {
    pub aabb: Aabb,
    pub kind: BvhKind,
}

/// Binary BVH over the curves' tight bounding boxes, flattened in
/// depth-first order with one curve per leaf.
#[derive(Clone, Debug, Default)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
}

impl Bvh {
    pub fn build(curves: &[BezierCurve]) -> Self {
        let boxes =
            curves.iter().map(BezierCurve::aabb).collect::<Vec<_>>();
        let mut order = (0..curves.len() as u32).collect::<Vec<_>>();
        let mut nodes = Vec::with_capacity(
            (curves.len() * 2).saturating_sub(1),
        );
        if !curves.is_empty() {
            build_node(&boxes, &mut order, &mut nodes);
        }
        Self { nodes }
    }

    /// Visits every curve whose bounding box lies within `radius` of
    /// `rect`, pruning whole subtrees that cannot.
    pub fn curves_near(
        &self,
        rect: Aabb,
        radius: f32,
        mut visit: impl FnMut(u32),
    ) {
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = vec![0u32];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index as usize];
            if rect.distance_to(&node.aabb) > radius {
                continue;
            }
            match node.kind {
                BvhKind::Leaf(curve) => visit(curve),
                BvhKind::Interior { right } => {
                    stack.push(index + 1);
                    stack.push(right);
                }
            }
        }
    }
}

// Splits `order` (indices into `boxes`) at the median of the axis the
// centroids spread widest on, and appends the subtree's nodes.
fn build_node(
    boxes: &[Aabb],
    order: &mut [u32],
    nodes: &mut Vec<BvhNode>,
) -> u32 {
    let aabb = order
        .iter()
        .map(|&i| boxes[i as usize])
        .reduce(Aabb::union)
        .unwrap();
    let node = nodes.len() as u32;

    if let [curve] = *order {
        nodes.push(BvhNode {
            aabb,
            kind: BvhKind::Leaf(curve),
        });
        return node;
    }

    let centroids = order
        .iter()
        .map(|&i| boxes[i as usize].center())
        .fold(None::<(Vec2, Vec2)>, |acc, c| {
            Some(match acc {
                Some((lo, hi)) => (lo.min(c), hi.max(c)),
                None => (c, c),
            })
        })
        .unwrap();
    let spread = centroids.1 - centroids.0;
    let axis = (spread.y > spread.x) as usize;
    order.sort_unstable_by(|&a, &b| {
        boxes[a as usize].center()[axis]
            .total_cmp(&boxes[b as usize].center()[axis])
    });

    nodes.push(BvhNode {
        aabb,
        kind: BvhKind::Interior { right: 0 },
    });
    let (left, right) = order.split_at_mut(order.len() / 2);
    build_node(boxes, left, nodes);
    let right = build_node(boxes, right, nodes);
    nodes[node as usize].kind = BvhKind::Interior { right };
    node
}

#[cfg(test)]
mod tests {
    use bevy_math::Vec2;

    use super::*;

    fn fan(count: usize) -> Vec<BezierCurve> {
        (0..count)
            .map(|i| {
                let a = i as f32;
                BezierCurve::new(
                    Vec2::new(a, 0.0),
                    Vec2::new(a + 0.5, 1.0),
                    Vec2::new(a + 1.0, 0.0),
                )
            })
            .collect()
    }

    fn contains(outer: Aabb, inner: Aabb) -> bool {
        outer.min.cmple(inner.min).all()
            && outer.max.cmpge(inner.max).all()
    }

    #[test]
    fn tight_aabb_includes_extremum() {
        let curve = BezierCurve::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(0.5, 1.0),
            Vec2::new(1.0, 0.0),
        );
        let aabb = curve.aabb();
        // Apex of this arch is at t = 0.5, y = 0.5.
        assert_eq!(aabb.min, Vec2::new(0.0, 0.0));
        assert_eq!(aabb.max, Vec2::new(1.0, 0.5));
    }

    #[test]
    fn every_curve_in_one_leaf() {
        let curves = fan(7);
        let bvh = Bvh::build(&curves);
        let mut seen: Vec<u32> = bvh
            .nodes
            .iter()
            .filter_map(|n| match n.kind {
                BvhKind::Leaf(curve) => Some(curve),
                BvhKind::Interior { .. } => None,
            })
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..7).collect::<Vec<_>>());
        assert_eq!(bvh.nodes.len(), 2 * 7 - 1);
    }

    #[test]
    fn parents_contain_children() {
        let curves = fan(9);
        let bvh = Bvh::build(&curves);
        for (i, node) in bvh.nodes.iter().enumerate() {
            match node.kind {
                BvhKind::Leaf(curve) => {
                    let curve = curves[curve as usize].aabb();
                    assert!(contains(node.aabb, curve));
                }
                BvhKind::Interior { right } => {
                    let left = bvh.nodes[i + 1].aabb;
                    let right = bvh.nodes[right as usize].aabb;
                    assert!(contains(node.aabb, left));
                    assert!(contains(node.aabb, right));
                }
            }
        }
    }

    #[test]
    fn curves_near_matches_brute_force() {
        let curves = fan(11);
        let bvh = Bvh::build(&curves);
        let rects = [
            Aabb {
                min: Vec2::new(2.0, 0.0),
                max: Vec2::new(3.0, 1.0),
            },
            Aabb {
                min: Vec2::new(-5.0, -5.0),
                max: Vec2::new(-4.0, -4.0),
            },
            Aabb {
                min: Vec2::new(0.0, 0.0),
                max: Vec2::new(11.0, 1.0),
            },
        ];
        for rect in rects {
            for radius in [0.0, 0.5, 2.0, 100.0] {
                let mut found = Vec::new();
                bvh.curves_near(rect, radius, |i| found.push(i));
                found.sort_unstable();
                let expected: Vec<u32> = (0..curves.len() as u32)
                    .filter(|&i| {
                        rect.distance_to(&curves[i as usize].aabb())
                            <= radius
                    })
                    .collect();
                assert_eq!(found, expected);
            }
        }
    }
}
