//! Face detection: finds the regions enclosed by a set of curves.
//!
//! Expects curves already split at their intersections (step 1,
//! [`split_intersections`](crate::split_intersections)), so curves meet only
//! at shared endpoints. From there:
//!
//! - The curves are flattened into a planar graph ([`graph`]).
//! - The graph's minimal cycles are the faces ([`cycles`]).
//! - Each curve is assigned a polarity: the face touching its left and its
//!   right side. An island's free side belongs to the face it floats in.
//!
//! A renderer then needs no face geometry at all: find the curve nearest to
//! a point, check which side the point is on, and look up that side's face.

mod cycles;
mod graph;

use std::collections::HashSet;

use bevy_math::Vec2;

use crate::BezierCurve;
use graph::Graph;

/// Cycles enclosing less area than this are noise from welding.
const MIN_FACE_AREA: f32 = 1e-6;

pub type FaceId = usize;

/// The face touching each side of a curve, looking along p0 -> p2.
/// `None` is the unbounded outside.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Polarity {
    pub left: Option<FaceId>,
    pub right: Option<FaceId>,
}

/// A region enclosed by curves.
#[derive(Clone, Debug)]
pub struct Face {
    /// Boundary curves in order, counter-clockwise: the interior is on the
    /// left of each curve.
    pub boundary: Vec<BezierCurve>,
}

/// Every face formed by a set of curves, and each curve's side assignments.
pub struct Faces {
    pub faces: Vec<Face>,
    /// One entry per input curve.
    pub polarity: Vec<Polarity>,
}

pub fn detect_faces(curves: &[BezierCurve]) -> Faces {
    let graph = Graph::new(curves);
    let positions: Vec<[f64; 2]> = graph
        .positions
        .iter()
        .map(|p| [p.x as f64, p.y as f64])
        .collect();

    // Orient every cycle counter-clockwise so "interior on the left" holds.
    let loops: Vec<Vec<usize>> =
        cycles::minimal_cycles(&positions, &graph.edges)
            .into_iter()
            .filter_map(|mut cycle| {
                let area = signed_area(&graph.polygon(&cycle));
                if cycle.len() < 3 || area.abs() < MIN_FACE_AREA {
                    return None;
                }
                if area < 0.0 {
                    cycle.reverse();
                }
                Some(cycle)
            })
            .collect();

    // A face claims the left side of every curve on its boundary.
    let mut polarity = vec![Polarity::default(); curves.len()];
    for (face, cycle) in loops.iter().enumerate() {
        for (curve, forward) in graph.curves_along(cycle) {
            let sides = &mut polarity[curve];
            if forward {
                sides.left = Some(face);
            } else {
                sides.right = Some(face);
            }
        }
    }

    // An island's boundary has one side facing the face it floats in: give
    // any side still free to the enclosing face. Sides shared with another
    // face (adjacent cycles) are already taken and stay.
    let polygons: Vec<Vec<Vec2>> =
        loops.iter().map(|l| graph.polygon(l)).collect();
    let areas: Vec<f32> =
        polygons.iter().map(|p| signed_area(p).abs()).collect();
    for (face, parent) in enclosing_faces(&loops, &polygons, &areas, &graph) {
        for (curve, forward) in graph.curves_along(&loops[face]) {
            let sides = &mut polarity[curve];
            let free = if forward {
                &mut sides.right
            } else {
                &mut sides.left
            };
            if free.is_none() {
                *free = Some(parent);
            }
        }
    }

    // A curve bounding no face still lies in one: both of its sides face it,
    // so it draws nothing of its own. One outside every face keeps None,
    // which a renderer needs to stop faces from reaching past their ends.
    for (curve, sides) in curves.iter().zip(&mut polarity) {
        if sides.left.is_some() || sides.right.is_some() {
            continue;
        }
        let midpoint = curve.eval(0.5);
        let Some(face) = (0..polygons.len())
            .filter(|&f| contains(&polygons[f], midpoint))
            .min_by(|&a, &b| areas[a].total_cmp(&areas[b]))
        else {
            continue;
        };
        sides.left = Some(face);
        sides.right = Some(face);
    }

    let faces = loops
        .iter()
        .map(|cycle| Face {
            boundary: graph.boundary(cycle, curves),
        })
        .collect();

    Faces { faces, polarity }
}

/// Pairs each face with the smallest face enclosing it, where one exists.
///
/// Minimal cycles never cross, so one strictly contains another exactly when
/// any non-shared vertex of the inner lies inside the outer.
fn enclosing_faces(
    loops: &[Vec<usize>],
    polygons: &[Vec<Vec2>],
    areas: &[f32],
    graph: &Graph,
) -> Vec<(FaceId, FaceId)> {
    let vertex_sets: Vec<HashSet<usize>> =
        loops.iter().map(|l| l.iter().copied().collect()).collect();

    (0..loops.len())
        .filter_map(|inner| {
            let parent = (0..loops.len())
                .filter(|&outer| outer != inner)
                .filter(|&outer| {
                    loops[inner]
                        .iter()
                        .find(|v| !vertex_sets[outer].contains(v))
                        .is_some_and(|&v| {
                            contains(
                                &polygons[outer],
                                graph.positions[v],
                            )
                        })
                })
                .min_by(|&a, &b| areas[a].total_cmp(&areas[b]))?;
            Some((inner, parent))
        })
        .collect()
}

fn signed_area(polygon: &[Vec2]) -> f32 {
    let n = polygon.len();
    (0..n)
        .map(|i| polygon[i].perp_dot(polygon[(i + 1) % n]))
        .sum::<f32>()
        * 0.5
}

/// Even-odd ray cast.
fn contains(polygon: &[Vec2], p: Vec2) -> bool {
    let n = polygon.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[j]);
        if (a.y > p.y) != (b.y > p.y)
            && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::split_intersections;

    fn line(a: Vec2, b: Vec2) -> BezierCurve {
        BezierCurve::new(a, a.lerp(b, 0.5), b)
    }

    /// Closed counter-clockwise loop of lines through the given corners.
    fn ring(corners: &[(f32, f32)]) -> Vec<BezierCurve> {
        let n = corners.len();
        (0..n)
            .map(|i| {
                let (a, b) = (corners[i], corners[(i + 1) % n]);
                line(Vec2::new(a.0, a.1), Vec2::new(b.0, b.1))
            })
            .collect()
    }

    fn square(x: f32, y: f32, size: f32) -> Vec<BezierCurve> {
        ring(&[
            (x, y),
            (x + size, y),
            (x + size, y + size),
            (x, y + size),
        ])
    }

    fn boundary_area(face: &Face) -> f32 {
        let polygon: Vec<Vec2> = face
            .boundary
            .iter()
            .flat_map(|c| {
                (0..16).map(move |i| c.eval(i as f32 / 16.0))
            })
            .collect();
        signed_area(&polygon)
    }

    /// Face id with the boundary area closest to `area`.
    fn face_with_area(detected: &Faces, area: f32) -> FaceId {
        let distance = |f: &Face| (boundary_area(f) - area).abs();
        let (id, face) = detected
            .faces
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                distance(a).total_cmp(&distance(b))
            })
            .unwrap();
        assert!(distance(face) < 1e-2, "no face with area {area}");
        id
    }

    #[test]
    fn square_encloses_one_face() {
        let detected = detect_faces(&square(0.0, 0.0, 1.0));

        assert_eq!(detected.faces.len(), 1);
        assert_eq!(detected.faces[0].boundary.len(), 4);
        assert!(
            (boundary_area(&detected.faces[0]) - 1.0).abs() < 1e-3
        );

        // The input winds counter-clockwise, so the face is on every
        // curve's left and the outside on its right.
        for sides in &detected.polarity {
            assert_eq!(sides.left, Some(0));
            assert_eq!(sides.right, None);
        }
    }

    #[test]
    fn open_chain_encloses_nothing() {
        let curves = vec![
            line(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)),
            line(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)),
        ];
        let detected = detect_faces(&curves);

        assert!(detected.faces.is_empty());
        assert!(
            detected
                .polarity
                .iter()
                .all(|p| *p == Polarity::default())
        );
    }

    #[test]
    fn separate_shapes_are_separate_faces() {
        let mut curves = ring(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        curves.extend(ring(&[(5.0, 0.0), (6.0, 0.0), (5.5, 1.0)]));
        assert_eq!(detect_faces(&curves).faces.len(), 2);
    }

    #[test]
    fn dangling_curves_take_the_face_they_lie_in() {
        let mut curves = square(0.0, 0.0, 4.0);
        curves.push(line(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0)));
        curves.push(line(Vec2::new(9.0, 9.0), Vec2::new(8.0, 8.0)));
        let detected = detect_faces(&curves);

        // Inside the square: both sides face it. Outside: no face at all.
        let inside = detected.polarity[4];
        assert_eq!(inside.left, Some(0));
        assert_eq!(inside.right, Some(0));
        assert_eq!(detected.polarity[5], Polarity::default());
    }

    #[test]
    fn concave_outline_is_one_face() {
        let detected = detect_faces(&ring(&[
            (0.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (1.0, 1.0),
            (1.0, 2.0),
            (0.0, 2.0),
        ]));

        assert_eq!(detected.faces.len(), 1);
        assert_eq!(detected.faces[0].boundary.len(), 6);
        assert!(
            (boundary_area(&detected.faces[0]) - 3.0).abs() < 1e-3
        );
    }

    #[test]
    fn diagonal_splits_square_in_two() {
        let mut curves = square(0.0, 0.0, 1.0);
        curves.push(line(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)));
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 2);

        // The diagonal borders both halves; the outer edges border one.
        let diagonal = detected.polarity.last().unwrap();
        assert!(diagonal.left.is_some() && diagonal.right.is_some());
        assert_ne!(diagonal.left, diagonal.right);
        for sides in &detected.polarity[..4] {
            assert!(sides.left.is_some() != sides.right.is_some());
        }
    }

    #[test]
    fn crossing_lines_enclose_their_center() {
        // Three lines crossing pairwise; the tails hang past the triangle.
        let curves = split_intersections(vec![
            line(Vec2::new(-0.30, 0.0), Vec2::new(1.30, 0.0)),
            line(Vec2::new(1.15, -0.30), Vec2::new(0.35, 1.30)),
            line(Vec2::new(0.65, 1.30), Vec2::new(-0.15, -0.30)),
        ]);
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 1);
        assert!(
            (boundary_area(&detected.faces[0]) - 0.5).abs() < 1e-2
        );

        // The tails touch no face on either side.
        let dangling = detected
            .polarity
            .iter()
            .filter(|p| **p == Polarity::default())
            .count();
        assert_eq!(dangling, 6);
    }

    #[test]
    fn island_floats_in_a_face() {
        let mut curves = square(0.0, 0.0, 4.0); // curves 0..4
        curves.extend(square(1.0, 1.0, 2.0)); // curves 4..8
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 2);
        let outer = face_with_area(&detected, 16.0);
        let island = face_with_area(&detected, 4.0);

        // The island's inside is its own; its outside belongs to the face
        // it floats in, so no region renders as background.
        for sides in &detected.polarity[4..] {
            assert_eq!(sides.left, Some(island));
            assert_eq!(sides.right, Some(outer));
        }
        for sides in &detected.polarity[..4] {
            assert_eq!(sides.left, Some(outer));
            assert_eq!(sides.right, None);
        }
    }

    #[test]
    fn nested_islands_chain_their_parents() {
        let mut curves = square(0.0, 0.0, 6.0);
        curves.extend(square(1.0, 1.0, 4.0));
        curves.extend(square(2.0, 2.0, 2.0));
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 3);
        let outer = face_with_area(&detected, 36.0);
        let middle = face_with_area(&detected, 16.0);
        let inner = face_with_area(&detected, 4.0);

        // Each island's outside belongs to its immediate parent.
        assert_eq!(detected.polarity[4].right, Some(outer));
        assert_eq!(detected.polarity[4].left, Some(middle));
        assert_eq!(detected.polarity[8].right, Some(middle));
        assert_eq!(detected.polarity[8].left, Some(inner));
    }

    #[test]
    fn bridged_island_keeps_its_parent() {
        let mut curves = square(0.0, 0.0, 4.0);
        curves.extend(ring(&[(1.0, 1.0), (2.0, 1.0), (1.5, 2.0)]));
        // A bridge from the outer square's corner to the island.
        curves.push(line(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)));
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 2);
        let outer = face_with_area(&detected, 16.0);
        let island = face_with_area(&detected, 0.5);
        for sides in &detected.polarity[4..7] {
            assert_eq!(sides.left, Some(island));
            assert_eq!(sides.right, Some(outer));
        }
        // The bridge bounds nothing; it floats in the outer face.
        assert_eq!(
            *detected.polarity.last().unwrap(),
            Polarity {
                left: Some(outer),
                right: Some(outer)
            }
        );
    }

    #[test]
    fn touching_islands_split_their_shared_edge() {
        let mut curves = square(0.0, 0.0, 4.0); // curves 0..4
        // Two 1x2 islands sharing the edge x = 2.
        curves.extend(ring(&[
            (1.0, 1.0),
            (2.0, 1.0),
            (3.0, 1.0),
            (3.0, 3.0),
            (2.0, 3.0),
            (1.0, 3.0),
        ])); // curves 4..10
        curves.push(line(Vec2::new(2.0, 1.0), Vec2::new(2.0, 3.0)));
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 3);
        let outer = face_with_area(&detected, 16.0);

        // The shared edge belongs to the two islands, not the outer face.
        let shared = detected.polarity.last().unwrap();
        assert!(shared.left.is_some() && shared.right.is_some());
        assert_ne!(shared.left, Some(outer));
        assert_ne!(shared.right, Some(outer));

        // Every island rim edge faces the outer face on its free side.
        for sides in &detected.polarity[4..10] {
            assert!(
                sides.left == Some(outer)
                    || sides.right == Some(outer)
            );
        }
    }

    #[test]
    fn arcs_crossing_twice_enclose_a_lens() {
        let up = BezierCurve::new(
            Vec2::new(0.2, 0.7),
            Vec2::new(0.5, 0.15),
            Vec2::new(0.8, 0.7),
        );
        let down = BezierCurve::new(
            Vec2::new(0.2, 0.3),
            Vec2::new(0.5, 0.85),
            Vec2::new(0.8, 0.3),
        );
        let detected =
            detect_faces(&split_intersections(vec![up, down]));

        assert_eq!(detected.faces.len(), 1);
        assert_eq!(detected.faces[0].boundary.len(), 2);
    }

    #[test]
    fn circle_floats_in_a_circle() {
        use crate::shapes::circle::circle;
        use bevy_color::LinearRgba;

        let mut curves =
            circle(Vec2::ZERO, 2.0, LinearRgba::WHITE, 0.01);
        let rim = curves.len();
        curves.extend(circle(
            Vec2::new(0.3, 0.2),
            1.0,
            LinearRgba::WHITE,
            0.01,
        ));
        let detected = detect_faces(&curves);

        assert_eq!(detected.faces.len(), 2);
        // Inner circle curves face both regions; outer ones face one.
        for sides in &detected.polarity[rim..] {
            assert!(sides.left.is_some() && sides.right.is_some());
        }
        for sides in &detected.polarity[..rim] {
            assert!(sides.left.is_some() != sides.right.is_some());
        }
    }
}
