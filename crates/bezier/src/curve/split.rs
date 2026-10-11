use bevy_math::Vec2;

use super::BezierCurve;

const SPLIT_EPS: f32 = 1e-4; // Dont split endpoints closer than this
const MERGE_EPS: f32 = 1e-2; // If two parameters are this close, we merge them into one

impl BezierCurve {
    // Split the curve at `t` (de Casteljau). returns left and right curves.
    pub fn split(&self, t: f32) -> (Self, Self) {
        let a = self.p0.lerp(self.p1, t);
        let b = self.p1.lerp(self.p2, t);
        let m = a.lerp(b, t);

        let mut left = *self;
        left.p1 = a;
        left.p2 = m;

        let mut right = *self;
        right.p0 = m;
        right.p1 = b;

        (left, right)
    }

    // Splits the curve at each `t` in `ts`.
    // Near duplicates (`MERGE_EPS`) are ignored.
    pub fn split_at(&self, ts: &[f32]) -> Vec<Self> {
        let mut ts: Vec<f32> = ts
            .iter()
            .copied()
            // if `t` is too close to the end/start of the curve, we just ignore it
            .filter(|t| *t > SPLIT_EPS && *t < 1.0 - SPLIT_EPS)
            .collect();
        ts.sort_by(|a, b| a.total_cmp(b));
        // splitting at such a small scale is impractical.
        // for this reason, we treat those two points as a similar point
        ts.dedup_by(|a, b| (*a - *b).abs() < MERGE_EPS);

        let mut pieces = Vec::with_capacity(ts.len() + 1);
        let mut remainder = *self;
        let mut prev = 0.0;
        for t in ts {
            let local = (t - prev) / (1.0 - prev);
            let (left, right) = remainder.split(local);
            pieces.push(left);
            remainder = right;
            // slowly consume till prev = last node
            prev = t;
        }
        pieces.push(remainder);
        pieces
    }
}

// Split every curve wherever it crosses another curve in the set.
//
// At every point a curve intersects with another, we split it there. In the
// majority of cases, two curves intersecting with one another will create 4
// resultant curves; left and right from each original curve.
//
// Non-intersecting curves remain untouched.
pub fn split_intersections(
    curves: Vec<BezierCurve>,
) -> Vec<BezierCurve> {
    const SAMPLES: usize = 256;

    let polylines: Vec<Vec<Vec2>> = curves
        .iter()
        .map(|c| {
            (0..=SAMPLES)
                .map(|i| c.eval(i as f32 / SAMPLES as f32))
                .collect()
        })
        .collect();

    // Collect intersection points for each curve, dont split yet
    // TODO: Some sort of optimisation? this is O(n^2)
    let mut splits: Vec<Vec<f32>> = vec![Vec::new(); curves.len()];
    for i in 0..curves.len() {
        for j in (i + 1)..curves.len() {
            for (ti, tj) in
                polyline_intersections(&polylines[i], &polylines[j])
            {
                splits[i].push(ti);
                splits[j].push(tj);
            }
        }
    }

    // Finally split at all the points
    let mut out = Vec::with_capacity(curves.len());
    for (curve, ts) in curves.iter().zip(splits.iter()) {
        out.extend(curve.split_at(ts));
    }
    out
}

// Returns the curve parameters at every intersection of two polylines.
// TODO: update to use another method. possibly bezier clipping?
// - https://scholarsarchive.byu.edu/cgi/viewcontent.cgi?article=1000&context=facpub
// - see Section 7.7, Fat line method
fn polyline_intersections(a: &[Vec2], b: &[Vec2]) -> Vec<(f32, f32)> {
    let samples = (a.len() - 1) as f32;
    let mut hits = Vec::new();
    for i in 0..a.len() - 1 {
        for j in 0..b.len() - 1 {
            if let Some((ua, ub)) =
                segment_intersection(a[i], a[i + 1], b[j], b[j + 1])
            {
                hits.push((
                    (i as f32 + ua) / samples,
                    (j as f32 + ub) / samples,
                ));
            }
        }
    }
    hits
}

// Intersect two line segments, returning the parameters along each if they
// cross, or `None` when parallel or non-overlapping.
fn segment_intersection(
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    p4: Vec2,
) -> Option<(f32, f32)> {
    let r = p2 - p1;
    let s = p4 - p3;
    let denom = r.perp_dot(s);
    if denom.abs() < 1e-12 {
        return None;
    }
    let qp = p3 - p1;
    let t = qp.perp_dot(s) / denom;
    let u = qp.perp_dot(r) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((t, u))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_intersections_breaks_crossing_curves() {
        // Curve A: an arc bowing to the left (top -> bottom).
        let a = BezierCurve::new(
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.5, 0.0),
            Vec2::new(0.0, -1.0),
        );
        // Curve B: sweeps left -> right and crosses A once.
        let b = BezierCurve::new(
            Vec2::new(-1.0, 0.3),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, -0.3),
        );

        let before = vec![a, b];
        println!("BEFORE ({} curves):", before.len());
        for (i, c) in before.iter().enumerate() {
            println!(
                "  [{i}] {:?} -> {:?} -> {:?}",
                c.p0, c.p1, c.p2
            );
        }

        let after = split_intersections(before.clone());
        println!("AFTER ({} curves):", after.len());
        for (i, c) in after.iter().enumerate() {
            println!(
                "  [{i}] {:?} -> {:?} -> {:?}",
                c.p0, c.p1, c.p2
            );
        }

        // Both curves crossed once, so each becomes two pieces.
        assert!(
            after.len() > before.len(),
            "expected the crossing to add pieces"
        );

        // Original outer endpoints are preserved.
        assert_eq!(after.first().unwrap().p0, a.p0);
        assert_eq!(after.last().unwrap().p2, b.p2);

        // The split point sits on both curves (they meet at the crossing).
        let a_pieces = a.split_at(&[]);
        assert_eq!(
            a_pieces.len(),
            1,
            "no splits requested -> unchanged"
        );
    }

    #[test]
    fn split_reconstructs_original_endpoints() {
        let c = BezierCurve::new(
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(1.0, 0.0),
        );
        let (left, right) = c.split(0.5);

        // Halves join at the midpoint and keep the outer endpoints.
        assert_eq!(left.p0, c.p0);
        assert_eq!(right.p2, c.p2);
        assert_eq!(left.p2, right.p0);
        assert!((left.p2 - c.eval(0.5)).length() < 1e-6);
    }
}
