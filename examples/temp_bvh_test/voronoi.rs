//! Temp: one bounding box per nearest-edge (voronoi-ish) cell.
//!
//! The fill shader's R view tints each pixel by its nearest edge, carving
//! the plane into voronoi-ish cells. This example reproduces that partition
//! on the CPU by sampling a grid, spawns one entity per cell, and draws each
//! cell's bounding box in the cell's tint colour. A cell's box encompasses
//! every sample its edge wins.

use bevy::prelude::*;
use bezier::{
    BezierCurve, bounding_box, gen_color, split_intersections,
};
use sketch::{Plane, Sketch};
use xad::material::{ShowEdgeTint, SketchRenderPlugin};

const STROKE: f32 = 0.012;
// Matches the fill quad's padding in sync.rs, so cells cover the whole quad.
const BBOX_PAD: f32 = 0.05;
// Samples along the wider axis of the sketch bounds.
const GRID: usize = 220;
// Straight segments approximating each curve for distance queries.
const CURVE_SEGMENTS: usize = 32;
// In front of the fill layer.
const CELL_LAYER_OFFSET: f32 = 0.003;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SketchRenderPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, draw_cells)
        .run();
}

/// Bounding box of one nearest-edge cell, in world space.
#[derive(Component)]
struct VoronoiCell {
    corners: [Vec3; 4],
    color: Color,
}

fn line(a: Vec2, b: Vec2) -> BezierCurve {
    BezierCurve::new(a, a.lerp(b, 0.5), b)
        .with_width(STROKE)
        .with_color(gen_color())
}

fn arc(p0: Vec2, p1: Vec2, p2: Vec2) -> BezierCurve {
    BezierCurve::new(p0, p1, p2)
        .with_width(STROKE)
        .with_color(gen_color())
}

fn scene() -> Vec<BezierCurve> {
    let mut curves = Vec::new();

    // Triangle.
    let t = Vec2::new(-1.3, 0.1);
    let (a, b, c) = (
        t + Vec2::new(0.0, 0.6),
        t + Vec2::new(-0.55, -0.35),
        t + Vec2::new(0.55, -0.35),
    );
    curves.extend([line(a, b), line(b, c), line(c, a)]);

    // Lens: two arcs crossing twice.
    let lens = Vec2::new(0.1, 0.0);
    curves.push(arc(
        lens + Vec2::new(-0.45, 0.25),
        lens + Vec2::new(0.0, -0.45),
        lens + Vec2::new(0.45, 0.25),
    ));
    curves.push(arc(
        lens + Vec2::new(-0.45, -0.25),
        lens + Vec2::new(0.0, 0.45),
        lens + Vec2::new(0.45, -0.25),
    ));

    // Circle cut by a chord.
    curves.extend(bezier::shapes::circle::circle(
        Vec2::new(1.5, 0.0),
        0.5,
        gen_color(),
        STROKE,
    ));
    curves.push(line(Vec2::new(0.9, -0.65), Vec2::new(2.1, 0.55)));

    curves
}

/// The shader's per-edge tint (PCG hash), replicated so each box matches
/// the region it bounds.
fn edge_tint(i: u32) -> Color {
    let mut s = i.wrapping_mul(747796405).wrapping_add(2891336453);
    s = ((s >> ((s >> 28) + 4)) ^ s).wrapping_mul(277803737);
    s = (s >> 22) ^ s;
    let rgb = Vec3::new(
        (s & 255) as f32,
        ((s >> 8) & 255) as f32,
        ((s >> 16) & 255) as f32,
    ) / 255.0;
    let c = Vec3::splat(0.35) + 0.65 * rgb;
    Color::linear_rgb(c.x, c.y, c.z)
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn setup(mut commands: Commands, mut tint: ResMut<ShowEdgeTint>) {
    // Match the drawn boxes to the shader's R view.
    tint.0 = true;

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 3.5))
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let curves = scene();

    // Same split the fill material uses, so cell indices and tints line up.
    let split = split_intersections(curves.clone());
    let polylines: Vec<Vec<Vec2>> = split
        .iter()
        .map(|curve| {
            (0..=CURVE_SEGMENTS)
                .map(|k| curve.eval(k as f32 / CURVE_SEGMENTS as f32))
                .collect()
        })
        .collect();

    let bounds = bounding_box(&split).unwrap().padded(BBOX_PAD);
    let size = bounds.size();
    let step = size.max_element() / GRID as f32;
    let (cols, rows) = (
        (size.x / step).ceil() as usize,
        (size.y / step).ceil() as usize,
    );

    // Nearest edge per grid sample; a cell's box grows over its samples.
    let mut cells: Vec<Option<(Vec2, Vec2)>> =
        vec![None; split.len()];
    for row in 0..=rows {
        for col in 0..=cols {
            let p = bounds.min
                + Vec2::new(col as f32, row as f32) * step;
            let (index, _) = polylines
                .iter()
                .enumerate()
                .map(|(index, points)| {
                    let d = points
                        .windows(2)
                        .map(|s| segment_distance(p, s[0], s[1]))
                        .fold(f32::MAX, f32::min);
                    (index, d)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            let cell = &mut cells[index];
            *cell = Some(match *cell {
                Some((lo, hi)) => (lo.min(p), hi.max(p)),
                None => (p, p),
            });
        }
    }

    // One entity per cell, its box padded by the sampling error.
    let sketch = Sketch {
        plane: Plane::right(),
        curves,
    };
    let normal = sketch.plane.normal();
    let world =
        |p: Vec2| sketch.plane_offset(p) + normal * CELL_LAYER_OFFSET;
    for (index, cell) in cells.iter().enumerate() {
        let Some((lo, hi)) = *cell else {
            continue;
        };
        let (lo, hi) =
            (lo - Vec2::splat(step), hi + Vec2::splat(step));
        commands.spawn(VoronoiCell {
            corners: [
                world(lo),
                world(Vec2::new(hi.x, lo.y)),
                world(hi),
                world(Vec2::new(lo.x, hi.y)),
            ],
            color: edge_tint(index as u32),
        });
    }

    commands.spawn(sketch.into_bundle());
}

fn draw_cells(mut gizmos: Gizmos, cells: Query<&VoronoiCell>) {
    for cell in &cells {
        gizmos.linestrip(
            cell.corners.iter().copied().chain([cell.corners[0]]),
            cell.color,
        );
    }
}
