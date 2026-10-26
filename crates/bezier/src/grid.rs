use bevy_math::Vec2;

use crate::bvh::Bvh;
use crate::curve::{Aabb, BezierCurve};

// Points evaluated on each curve as distance upper-bound anchors.
const ANCHOR_TS: [f32; 3] = [0.0, 0.5, 1.0];

// Slack added to the bound: keeps borderline candidates that float
// rounding (or a consumer's epsilon, like the fill shader's vertex
// tolerance) could otherwise push just past it.
const MARGIN: f32 = 1e-3;

/// Per-tile shortlist into a [`TileGrid`]'s flat index array.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tile {
    pub offset: u32,
    pub count: u32,
}

/// Uniform grid over `bounds` where each tile lists every curve that
/// could be the nearest one somewhere in that tile.
///
/// Lists are conservative: built from bounding boxes and on-curve anchor
/// points only, they may hold extra curves but never miss the true
/// nearest. A nearest-curve query over a tile's list therefore matches a
/// query over all curves exactly.
#[derive(Clone, Debug)]
pub struct TileGrid {
    pub bounds: Aabb,
    pub cols: u32,
    pub rows: u32,
    /// Row-major from `bounds.min`.
    pub tiles: Vec<Tile>,
    pub indices: Vec<u32>,
}

impl TileGrid {
    /// Bins `curves` over `bounds`, splitting the longer axis into
    /// `resolution` tiles and the other proportionally.
    pub fn build(
        curves: &[BezierCurve],
        bounds: Aabb,
        resolution: u32,
    ) -> Self {
        let (cols, rows) = dimensions(bounds, resolution);
        let tile_size =
            bounds.size() / Vec2::new(cols as f32, rows as f32);

        let bvh = Bvh::build(curves);
        let anchors: Vec<[Vec2; 3]> = curves
            .iter()
            .map(|c| ANCHOR_TS.map(|t| c.eval(t)))
            .collect();

        let mut tiles = Vec::with_capacity((cols * rows) as usize);
        let mut indices = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let min = bounds.min
                    + tile_size * Vec2::new(col as f32, row as f32);
                let tile = Aabb {
                    min,
                    max: min + tile_size,
                };

                let offset = indices.len() as u32;
                bvh.curves_near(tile, reach(tile, &anchors), |i| {
                    indices.push(i);
                });
                indices[offset as usize..].sort_unstable();
                tiles.push(Tile {
                    offset,
                    count: indices.len() as u32 - offset,
                });
            }
        }

        Self {
            bounds,
            cols,
            rows,
            tiles,
            indices,
        }
    }

    /// Like [`build`](Self::build), but halves the resolution until the
    /// flat index list fits `max_indices` (for fixed-size GPU arrays).
    /// 1x1 always fits: it lists each curve once.
    pub fn build_within(
        curves: &[BezierCurve],
        bounds: Aabb,
        mut resolution: u32,
        max_indices: usize,
    ) -> Self {
        loop {
            let grid = Self::build(curves, bounds, resolution);
            if grid.indices.len() <= max_indices || resolution == 1 {
                return grid;
            }
            resolution /= 2;
        }
    }
}

// Tile counts per axis: `resolution` along the longer side, the other
// proportional to stay square-ish.
fn dimensions(bounds: Aabb, resolution: u32) -> (u32, u32) {
    let size = bounds.size().max(Vec2::splat(f32::EPSILON));
    let scaled = size / size.max_element() * resolution as f32;
    (
        (scaled.x.round() as u32).max(1),
        (scaled.y.round() as u32).max(1),
    )
}

// Upper bound on the nearest-curve distance from anywhere in `tile`:
// for one anchor point `a` on some curve, dist(p, a) is convex in p, so
// its maximum over the tile is at a corner, and the nearest curve is
// never farther than that. The smallest such bound over all anchors
// caps every curve the tile can possibly see.
fn reach(tile: Aabb, anchors: &[[Vec2; 3]]) -> f32 {
    let corners = tile.corners();
    anchors
        .iter()
        .flatten()
        .map(|&anchor| {
            corners
                .iter()
                .map(|&c| c.distance(anchor))
                .fold(0.0, f32::max)
        })
        .fold(f32::MAX, f32::min)
        + MARGIN
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::bounding_box;

    // Scene with crossings, shared endpoints, and a dangling curve.
    fn scene() -> Vec<BezierCurve> {
        let line =
            |a: Vec2, b: Vec2| BezierCurve::new(a, a.lerp(b, 0.5), b);
        vec![
            // Triangle.
            line(Vec2::new(0.0, 0.6), Vec2::new(-0.55, -0.35)),
            line(Vec2::new(-0.55, -0.35), Vec2::new(0.55, -0.35)),
            line(Vec2::new(0.55, -0.35), Vec2::new(0.0, 0.6)),
            // Arcs crossing twice (a lens).
            BezierCurve::new(
                Vec2::new(0.8, 0.25),
                Vec2::new(1.25, -0.45),
                Vec2::new(1.7, 0.25),
            ),
            BezierCurve::new(
                Vec2::new(0.8, -0.25),
                Vec2::new(1.25, 0.45),
                Vec2::new(1.7, -0.25),
            ),
            // Dangling line far off to one side.
            line(Vec2::new(-2.0, 1.5), Vec2::new(-1.2, 1.0)),
        ]
    }

    // Reference distance via a dense polyline.
    fn distance(p: Vec2, curve: &BezierCurve) -> f32 {
        const STEPS: usize = 256;
        (0..STEPS)
            .map(|k| {
                let a = curve.eval(k as f32 / STEPS as f32);
                let b = curve.eval((k + 1) as f32 / STEPS as f32);
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared())
                    .clamp(0.0, 1.0);
                p.distance(a + ab * t)
            })
            .fold(f32::MAX, f32::min)
    }

    #[test]
    fn every_tile_has_a_candidate() {
        let curves = scene();
        let bounds = bounding_box(&curves).unwrap().padded(0.3);
        let grid = TileGrid::build(&curves, bounds, 8);
        assert_eq!(
            grid.tiles.len(),
            (grid.cols * grid.rows) as usize
        );
        for tile in &grid.tiles {
            assert!(tile.count > 0);
        }
    }

    #[test]
    fn tile_lists_contain_the_true_nearest() {
        let curves = scene();
        let bounds = bounding_box(&curves).unwrap().padded(0.3);
        let grid = TileGrid::build(&curves, bounds, 8);

        let tile_size = bounds.size()
            / Vec2::new(grid.cols as f32, grid.rows as f32);
        // Dense sample sweep, including points outside every curve.
        let samples = 60;
        for sy in 0..=samples {
            for sx in 0..=samples {
                let p = bounds.min
                    + bounds.size()
                        * Vec2::new(sx as f32, sy as f32)
                        / samples as f32;

                let nearest = (0..curves.len())
                    .min_by(|&a, &b| {
                        distance(p, &curves[a])
                            .total_cmp(&distance(p, &curves[b]))
                    })
                    .unwrap() as u32;

                let cell = ((p - bounds.min) / tile_size)
                    .floor()
                    .min(Vec2::new(
                        grid.cols as f32 - 1.0,
                        grid.rows as f32 - 1.0,
                    ));
                let tile = grid.tiles[(cell.y as u32 * grid.cols
                    + cell.x as u32)
                    as usize];
                let list = &grid.indices[tile.offset as usize..]
                    [..tile.count as usize];
                assert!(
                    list.contains(&nearest),
                    "nearest curve {nearest} missing from tile list \
                     {list:?} at {p}",
                );
            }
        }
    }
}
