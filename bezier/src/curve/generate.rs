use bevy_math::Vec2;

use super::BezierCurve;

impl BezierCurve {
    // constantly checking whether its cubic or quadratic GPU-side is expensive.
    // we compute cubic curves into pair quads on CPU, and pass that to GPU instead
    pub fn cubic(
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        p3: Vec2,
    ) -> [Self; 2] {
        // TODO: consider if we should return impl Iter instead?
        let m = (p0 + 3.0 * p1 + 3.0 * p2 + p3) * 0.125;
        let l1 = (p0 + p1) * 0.5;
        let l2 = (p0 + 2.0 * p1 + p2) * 0.25;
        let ql = (3.0 * (l1 + l2) - p0 - m) * 0.25;
        let r1 = (p1 + 2.0 * p2 + p3) * 0.25;
        let r2 = (p2 + p3) * 0.5;
        let qr = (3.0 * (r1 + r2) - m - p3) * 0.25;

        [Self::new(p0, ql, m), Self::new(m, qr, p3)]
    }
}
