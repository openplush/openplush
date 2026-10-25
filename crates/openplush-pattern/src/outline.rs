use openplush_geom::predicates::orient2d;
use openplush_geom::{DVec2, GeomError, Sign};

use crate::PatternError;

/// A closed, simple, counter-clockwise polygon in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    points: Vec<DVec2>,
}

impl Outline {
    /// Refuses an outline that is too short, non-finite, self-touching, folded or clockwise. Cost is quadratic in the point count.
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

    /// The unit direction out of the outline at corner `i`, halfway between its two edges' outward
    /// normals, so it points out at reflex corners too; `None` past the last corner.
    pub fn outward_at(&self, i: usize) -> Option<DVec2> {
        let n = self.points.len();
        let at = *self.points.get(i)?;
        let (before, after) = (self.points[(i + n - 1) % n], self.points[(i + 1) % n]);
        let right = |d: DVec2| DVec2::new(d.y, -d.x).normalize();
        // A simple outline never folds back, so the two normals never cancel.
        (right(at - before)? + right(after - at)?).normalize()
    }

    /// A point inside the outline as far from its edges as a 32 × 32 grid over its bounding box
    /// finds, the first such in row order, so a label there stays clear of narrow parts. A
    /// piece too thin for the grid gets a point just inside its lowest, leftmost corner.
    pub fn label_point(&self) -> DVec2 {
        const CELLS: u32 = 32;
        let (min, max) =
            self.points
                .iter()
                .fold((self.points[0], self.points[0]), |(lo, hi), p| {
                    (
                        DVec2::new(lo.x.min(p.x), lo.y.min(p.y)),
                        DVec2::new(hi.x.max(p.x), hi.y.max(p.y)),
                    )
                });
        let cell = (max - min) / f64::from(CELLS);
        let mut best: Option<(f64, DVec2)> = None;
        for row in 0..CELLS {
            for col in 0..CELLS {
                let p = min
                    + DVec2::new(
                        cell.x * (f64::from(col) + 0.5),
                        cell.y * (f64::from(row) + 0.5),
                    );
                if !self.contains(p) {
                    continue;
                }
                let clear = self.clearance(p);
                if best.is_none_or(|(c, _)| clear > c) {
                    best = Some((clear, p));
                }
            }
        }
        best.map_or_else(|| self.inside_lowest_corner(), |(_, p)| p)
    }

    /// Even-odd: whether `p` is inside, for points off the outline.
    fn contains(&self, p: DVec2) -> bool {
        self.edges()
            .filter(|&(a, b)| {
                (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x)
            })
            .count()
            % 2
            == 1
    }

    /// The distance from `p` to the nearest edge.
    fn clearance(&self, p: DVec2) -> f64 {
        self.edges()
            .map(|(a, b)| {
                let t = ((p - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
                (a + (b - a) * t - p).length()
            })
            .fold(f64::INFINITY, f64::min)
    }

    fn edges(&self) -> impl Iterator<Item = (DVec2, DVec2)> + '_ {
        let n = self.points.len();
        (0..n).map(move |i| (self.points[i], self.points[(i + 1) % n]))
    }

    /// The lowest, leftmost corner, always convex, moved inward along its bisector by a quarter
    /// of its shorter edge.
    fn inside_lowest_corner(&self) -> DVec2 {
        let n = self.points.len();
        let i = (0..n)
            .min_by(|&a, &b| {
                let (p, q) = (self.points[a], self.points[b]);
                p.y.total_cmp(&q.y).then(p.x.total_cmp(&q.x))
            })
            .unwrap_or(0);
        let at = self.points[i];
        let shorter = (at - self.points[(i + n - 1) % n])
            .length()
            .min((self.points[(i + 1) % n] - at).length());
        let out = self.outward_at(i).unwrap_or(DVec2::new(0.0, 0.0));
        at - out * (shorter / 4.0)
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
        // Plain comparison, not `total_cmp`, which orders -0.0 below 0.0; every coordinate is finite here.
        pi.y < pj.y || (pi.y == pj.y && pi.x < pj.x)
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
    // Exact, and skips four exact predicate calls for the vast majority of edge pairs.
    if p.x.min(q.x) > r.x.max(s.x)
        || r.x.min(s.x) > p.x.max(q.x)
        || p.y.min(q.y) > r.y.max(s.y)
        || r.y.min(s.y) > p.y.max(q.y)
    {
        return Ok(false);
    }
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
