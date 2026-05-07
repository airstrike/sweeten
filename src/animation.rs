//! Animation utilities shared across sweetened widgets.
//!
//! Currently just [`cubic_bezier`] — the math kernel several widgets
//! call into when wiring [`Easing::Custom`](crate::core::animation::Easing::Custom)
//! to a curve borrowed from a popular design system (Tailwind, shadcn,
//! framer-motion).
//!
//! Kept `pub(crate)` deliberately — this is an implementation detail,
//! not an API surface we want to commit to.

/// Evaluates a cubic-bezier curve at parameter `x`.
///
/// The curve has control points `P0 = (0, 0)`, `P1 = (x1, y1)`,
/// `P2 = (x2, y2)`, `P3 = (1, 1)`. Given an `x` in `[0, 1]`, we solve
/// for the parameter `t` such that `B_x(t) = x` using a few iterations
/// of Newton's method, then evaluate `B_y(t)` at that `t`.
pub(crate) fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let mut t = x;
    for _ in 0..8 {
        let t2 = t * t;
        let t3 = t2 * t;
        let bx = 3.0 * (1.0 - t) * (1.0 - t) * t * x1
            + 3.0 * (1.0 - t) * t2 * x2
            + t3;
        let dbx = 3.0 * (1.0 - t) * (1.0 - t) * x1
            + 6.0 * (1.0 - t) * t * (x2 - x1)
            + 3.0 * t2 * (1.0 - x2);
        if dbx.abs() < 1e-6 {
            break;
        }
        t -= (bx - x) / dbx;
        t = t.clamp(0.0, 1.0);
    }

    let t2 = t * t;
    let t3 = t2 * t;
    3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t2 * y2 + t3
}
