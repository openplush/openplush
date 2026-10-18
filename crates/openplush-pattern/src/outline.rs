use openplush_geom::predicates::orient2d;
use openplush_geom::{DVec2, GeomError, Sign};

use crate::PatternError;

/// A closed, simple, counter-clockwise polygon in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    points: Vec<DVec2>,
}

impl Outline {
    /// Refuses an outline that is too short, non-finite, self-touching, folded or clockwise.
    pub fn new(points: Vec<DVec2>) -> Result<Self, PatternError> {
        if points.len() < 3 {
            return Err(PatternError::OutlineTooShort {
                points: points.len(),
            });
        }
        if let Some(point) = points.iter().position(|p| !p.is_finite()) {
            return Err(PatternError::OutlineNotFinite { point });
        }
        check_simple(&points).map_err(|e| match e {
            Refusal::Edges(edge, other) => PatternError::OutlineNotSimple { edge, other },
            Refusal::Predicate(source) => PatternError::OutlineUnjudgeable { source },
        })?;
        check_counter_clockwise(&points)?;
        Ok(Self { points })
    }

    /// The corners in order; edge `i` runs from point `i` to point `i + 1`, the last to the first.
    pub fn points(&self) -> &[DVec2] {
        &self.points
    }
}

enum Refusal {
    Edges(usize, usize),
    Predicate(GeomError),
}

impl From<GeomError> for Refusal {
    fn from(source: GeomError) -> Self {
        Self::Predicate(source)
    }
}

/// No repeated point, no fold at a corner, and no two non-adjacent edges that meet.
fn check_simple(points: &[DVec2]) -> Result<(), Refusal> {
    let n = points.len();
    // INVARIANT: the caller checked n >= 3 and every index is reduced mod n.
    let edge = |i: usize| (points[i % n], points[(i + 1) % n]);
    if let Some(a) = (0..n).find(|&a| edge(a).0 == edge(a).1) {
        return Err(Refusal::Edges(a, a));
    }
    for a in 0..n {
        let (p, q) = edge(a);
        if !turns_cleanly(p, q, edge(a + 1).1)? {
            return Err(Refusal::Edges(a, (a + 1) % n));
        }
        // Edge n - 1 is adjacent to edge 0, so it is skipped for edge 0 only.
        let end = if a == 0 { n - 1 } else { n };
        for b in a + 2..end {
            let (r, s) = edge(b);
            if segments_meet(p, q, r, s)? {
                return Err(Refusal::Edges(a, b));
            }
        }
    }
    Ok(())
}

/// The lowest, then leftmost, vertex of a simple polygon is convex, so its turn is the orientation.
fn check_counter_clockwise(points: &[DVec2]) -> Result<(), PatternError> {
    let n = points.len();
    let lowest = |i: usize, j: usize| {
        let (pi, pj) = (points[i], points[j]);
        pi.y.total_cmp(&pj.y).then(pi.x.total_cmp(&pj.x)).is_lt()
    };
    let v = (1..n).fold(0, |best, i| if lowest(i, best) { i } else { best });
    let prev = (v + n - 1) % n;
    match orient2d(points[prev], points[v], points[(v + 1) % n]) {
        Ok(Sign::Positive) => Ok(()),
        Ok(Sign::Negative) => Err(PatternError::OutlineClockwise),
        // A zero turn at this corner would have been a fold, which `check_simple` refused.
        Ok(Sign::Zero) => Err(PatternError::OutlineNotSimple {
            edge: prev,
            other: v,
        }),
        Err(source) => Err(PatternError::OutlineUnjudgeable { source }),
    }
}

/// The corner `p -> q -> next` does not fold `next` back over `pq`. Precondition: `p != q` and `q != next`.
fn turns_cleanly(p: DVec2, q: DVec2, next: DVec2) -> Result<bool, GeomError> {
    Ok(match orient2d(p, q, next)? {
        Sign::Zero => !(within_box(q, p, next) || within_box(q, next, p)),
        _ => true,
    })
}

/// Closed segments `pq` and `rs` share at least one point.
fn segments_meet(p: DVec2, q: DVec2, r: DVec2, s: DVec2) -> Result<bool, GeomError> {
    let (o1, o2) = (orient2d(p, q, r)?, orient2d(p, q, s)?);
    let (o3, o4) = (orient2d(r, s, p)?, orient2d(r, s, q)?);
    let opposite = |x: Sign, y: Sign| x != Sign::Zero && y != Sign::Zero && x != y;
    if opposite(o1, o2) && opposite(o3, o4) {
        return Ok(true);
    }
    Ok((o1 == Sign::Zero && within_box(p, q, r))
        || (o2 == Sign::Zero && within_box(p, q, s))
        || (o3 == Sign::Zero && within_box(r, s, p))
        || (o4 == Sign::Zero && within_box(r, s, q)))
}

/// `c`, known collinear with `ab`, lies on the closed segment `ab`.
fn within_box(a: DVec2, b: DVec2, c: DVec2) -> bool {
    a.x.min(b.x) <= c.x && c.x <= a.x.max(b.x) && a.y.min(b.y) <= c.y && c.y <= a.y.max(b.y)
}
