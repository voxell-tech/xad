use std::collections::HashMap;

use bevy_math::Vec2;

use crate::BezierCurve;

/// Endpoints closer than this weld into one vertex.
const WELD_DISTANCE: f32 = 2e-3;
/// Straight segments used to approximate each curve.
const SEGMENTS_PER_CURVE: usize = 8;

/// The curves flattened into a planar graph. Curve endpoints weld into shared
/// vertices; the sample vertices along a curve's interior belong to it alone.
pub(super) struct Graph {
    pub(super) positions: Vec<Vec2>,
    pub(super) edges: Vec<[usize; 2]>,
    /// Vertices below this index are curve endpoints.
    endpoints: usize,
    /// Directed edge -> (curve, whether the edge follows p0 -> p2).
    owner: HashMap<(usize, usize), (usize, bool)>,
}

impl Graph {
    pub(super) fn new(curves: &[BezierCurve]) -> Self {
        let mut welder = Welder::default();
        let ends: Vec<(usize, usize)> = curves
            .iter()
            .map(|c| (welder.vertex(c.p0), welder.vertex(c.p2)))
            .collect();

        let mut positions = welder.positions;
        let endpoints = positions.len();
        let mut edges = Vec::new();
        let mut owner = HashMap::new();

        for (index, (curve, &(start, end))) in
            curves.iter().zip(&ends).enumerate()
        {
            // A curve that starts and ends at one vertex encloses no area.
            if start == end {
                continue;
            }
            let mut prev = start;
            for k in 1..=SEGMENTS_PER_CURVE {
                let next = if k == SEGMENTS_PER_CURVE {
                    end
                } else {
                    let t = k as f32 / SEGMENTS_PER_CURVE as f32;
                    positions.push(curve.eval(t));
                    positions.len() - 1
                };
                edges.push([prev, next]);
                owner.insert((prev, next), (index, true));
                owner.insert((next, prev), (index, false));
                prev = next;
            }
        }

        Self {
            positions,
            edges,
            endpoints,
            owner,
        }
    }

    /// Vertex positions of a loop, as a polygon.
    pub(super) fn polygon(&self, vertices: &[usize]) -> Vec<Vec2> {
        vertices.iter().map(|&v| self.positions[v]).collect()
    }

    /// The curves a closed loop runs along, with whether the loop follows
    /// each curve's p0 -> p2 direction.
    pub(super) fn curves_along(
        &self,
        vertices: &[usize],
    ) -> Vec<(usize, bool)> {
        let n = vertices.len();
        // Each curve's run of segments starts at an endpoint vertex.
        (0..n)
            .filter(|&i| vertices[i] < self.endpoints)
            .filter_map(|i| {
                let edge = (vertices[i], vertices[(i + 1) % n]);
                self.owner.get(&edge).copied()
            })
            .collect()
    }

    /// The curves of a loop, oriented to the loop's direction.
    pub(super) fn boundary(
        &self,
        vertices: &[usize],
        curves: &[BezierCurve],
    ) -> Vec<BezierCurve> {
        self.curves_along(vertices)
            .into_iter()
            .map(|(index, forward)| {
                let mut curve = curves[index];
                if !forward {
                    std::mem::swap(&mut curve.p0, &mut curve.p2);
                }
                curve
            })
            .collect()
    }
}

/// Welds nearby points into shared vertices using a spatial grid.
#[derive(Default)]
struct Welder {
    positions: Vec<Vec2>,
    grid: HashMap<(i32, i32), Vec<usize>>,
}

impl Welder {
    fn vertex(&mut self, p: Vec2) -> usize {
        let cell_x = (p.x / WELD_DISTANCE).floor() as i32;
        let cell_y = (p.y / WELD_DISTANCE).floor() as i32;

        // A match can sit in any neighbouring cell; scan the 3x3 block.
        for dx in -1..=1 {
            for dy in -1..=1 {
                let cell = (cell_x + dx, cell_y + dy);
                let Some(ids) = self.grid.get(&cell) else {
                    continue;
                };
                for &id in ids {
                    if self.positions[id].distance_squared(p)
                        <= WELD_DISTANCE * WELD_DISTANCE
                    {
                        return id;
                    }
                }
            }
        }

        let id = self.positions.len();
        self.positions.push(p);
        self.grid.entry((cell_x, cell_y)).or_default().push(id);
        id
    }
}
