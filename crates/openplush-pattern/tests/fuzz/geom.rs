//! Plain f64 helpers shared by the generator, the construction and the oracle.
//! Simplicity tests use exact orientation (`robust`), so they never depend on i_overlay.

pub(crate) type P = [f64; 2];

pub(crate) fn orient(a: P, b: P, c: P) -> f64 {
    let c2 = |p: P| robust::Coord { x: p[0], y: p[1] };
    robust::orient2d(c2(a), c2(b), c2(c))
}

pub(crate) fn signed_area(poly: &[P]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f64>()
        / 2.0
}

/// Closed segments `ab` and `cd` share at least one point (exact).
pub(crate) fn segments_meet(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2, o3, o4) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        return true;
    }
    let on = |p: P, q: P, r: P, o: f64| {
        o == 0.0
            && r[0] >= p[0].min(q[0])
            && r[0] <= p[0].max(q[0])
            && r[1] >= p[1].min(q[1])
            && r[1] <= p[1].max(q[1])
    };
    on(a, b, c, o1) || on(a, b, d, o2) || on(c, d, a, o3) || on(c, d, b, o4)
}

/// Index pair of the first two edges that meet other than at their shared vertex.
pub(crate) fn first_self_contact(poly: &[P]) -> Option<(usize, usize)> {
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        for j in i + 1..n {
            let (c, d) = (poly[j], poly[(j + 1) % n]);
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            if adjacent {
                // Neighbours share one vertex; they must not overlap beyond it.
                let (p, q, r) = if j == i + 1 { (a, b, d) } else { (c, d, b) };
                if orient(p, q, r) == 0.0
                    && (r[0] - q[0]) * (p[0] - q[0]) + (r[1] - q[1]) * (p[1] - q[1]) > 0.0
                {
                    return Some((i, j));
                }
            } else if segments_meet(a, b, c, d) {
                return Some((i, j));
            }
        }
    }
    None
}

pub(crate) fn is_simple_ccw(poly: &[P]) -> bool {
    poly.len() >= 3
        && poly.iter().all(|p| p[0].is_finite() && p[1].is_finite())
        && (0..poly.len()).all(|i| poly[i] != poly[(i + 1) % poly.len()])
        && signed_area(poly) > 0.0
        && first_self_contact(poly).is_none()
}

/// Winding number of `p` with respect to a set of closed contours.
pub(crate) fn winding(contours: &[&[P]], p: P) -> i32 {
    let mut w = 0;
    for c in contours {
        let n = c.len();
        for i in 0..n {
            let (a, b) = (c[i], c[(i + 1) % n]);
            if a[1] <= p[1] {
                if b[1] > p[1] && orient(a, b, p) > 0.0 {
                    w += 1;
                }
            } else if b[1] <= p[1] && orient(a, b, p) < 0.0 {
                w -= 1;
            }
        }
    }
    w
}

pub(crate) fn dist_to_segment(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (x, y) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (x * x + y * y).sqrt()
}

pub(crate) fn dist_to_boundary(contour: &[P], p: P) -> f64 {
    let n = contour.len();
    (0..n)
        .map(|i| dist_to_segment(p, contour[i], contour[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min)
}

pub(crate) fn bbox(points: impl Iterator<Item = P>) -> (P, P) {
    points.fold(
        ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
        |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1])],
                [hi[0].max(p[0]), hi[1].max(p[1])],
            )
        },
    )
}

pub(crate) fn bbox_diag(points: impl Iterator<Item = P>) -> f64 {
    let (lo, hi) = bbox(points);
    dist(lo, hi)
}

/// Euclidean distance with only IEEE-exact operations (`powi` and `hypot` are not, across targets).
pub(crate) fn dist(a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    (dx * dx + dy * dy).sqrt()
}
