//! Polygons to triangles, without moving or adding a vertex.

use openplush_geom::predicates::orient2d;
use openplush_geom::{DVec2, DVec3, Sign};

/// Splits a polygon into `points.len() - 2` triangles, as corner indices in the
/// polygon's winding. Quads split on the shorter diagonal that keeps both triangles
/// turning the polygon's way; larger polygons are ear-clipped on the plane their
/// Newell normal faces. `None` when no split keeps every triangle turning that way
/// (the polygon crosses itself), unless the polygon has no area at all: then it is
/// split anyway and validation reports the zero-area triangles. Fewer than three
/// points give no triangles.
pub(crate) fn triangulate(points: &[DVec3]) -> Option<Vec<[usize; 3]>> {
    match points.len() {
        0..=2 => Some(Vec::new()),
        3 => Some(vec![[0, 1, 2]]),
        4 => quad(points),
        _ => ear_clip(points, &project(points)),
    }
}

fn quad(points: &[DVec3]) -> Option<Vec<[usize; 3]>> {
    let ac = [[0, 1, 2], [0, 2, 3]];
    let bd = [[0, 1, 3], [1, 2, 3]];
    let length = |i: usize, j: usize| (points[i] - points[j]).length_squared();
    let order = if length(1, 3) < length(0, 2) {
        [bd, ac]
    } else {
        [ac, bd]
    };
    let flat = project(points);
    let turns_right = |tris: &[[usize; 3]; 2]| tris.iter().all(|&t| ccw(&flat, t));
    match order.iter().find(|t| turns_right(t)) {
        Some(chosen) => Some(chosen.to_vec()),
        None => collinear(points, &order[0]).then(|| order[0].to_vec()),
    }
}

/// The points in 2D, dropping the axis the Newell normal leans on most and ordering
/// the other two so the polygon turns counter-clockwise when it turns along the normal.
fn project(points: &[DVec3]) -> Vec<DVec2> {
    let n = newell(points);
    let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
    let map: fn(DVec3) -> DVec2 = if ax >= ay && ax >= az {
        if n.x >= 0.0 {
            |p| DVec2::new(p.y, p.z)
        } else {
            |p| DVec2::new(p.z, p.y)
        }
    } else if ay >= az {
        if n.y >= 0.0 {
            |p| DVec2::new(p.z, p.x)
        } else {
            |p| DVec2::new(p.x, p.z)
        }
    } else if n.z >= 0.0 {
        |p| DVec2::new(p.x, p.y)
    } else {
        |p| DVec2::new(p.y, p.x)
    };
    points.iter().map(|&p| map(p)).collect()
}

fn newell(points: &[DVec3]) -> DVec3 {
    let mut n = DVec3::default();
    for (i, &p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        n += DVec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    n
}

/// Strictly counter-clockwise. A predicate that cannot decide counts as not.
fn ccw(flat: &[DVec2], [a, b, c]: [usize; 3]) -> bool {
    orient2d(flat[a], flat[b], flat[c]) == Ok(Sign::Positive)
}

/// Every triangle exactly degenerate in 3D: collinear in each coordinate plane. A
/// self-crossing polygon can have a zero Newell normal, so its projection cannot tell.
fn collinear(points: &[DVec3], tris: &[[usize; 3]]) -> bool {
    let planes: [fn(DVec3) -> DVec2; 3] = [
        |p| DVec2::new(p.x, p.y),
        |p| DVec2::new(p.y, p.z),
        |p| DVec2::new(p.z, p.x),
    ];
    tris.iter().all(|&[a, b, c]| {
        planes.iter().all(|plane| {
            orient2d(plane(points[a]), plane(points[b]), plane(points[c])) == Ok(Sign::Zero)
        })
    })
}

/// After each cut the search steps back one corner, since the cut can turn the
/// previous corner into an ear; the order is fixed, so the result is deterministic.
fn ear_clip(points: &[DVec3], flat: &[DVec2]) -> Option<Vec<[usize; 3]>> {
    let mut ring: Vec<usize> = (0..flat.len()).collect();
    let mut out = Vec::with_capacity(flat.len() - 2);
    let mut i = 0;
    let mut misses = 0;
    while ring.len() > 3 && misses < ring.len() {
        let m = ring.len();
        let tri = [ring[(i + m - 1) % m], ring[i], ring[(i + 1) % m]];
        if is_ear(flat, &ring, tri) {
            out.push(tri);
            ring.remove(i);
            i = i.saturating_sub(1) % ring.len();
            misses = 0;
        } else {
            i = (i + 1) % m;
            misses += 1;
        }
    }
    let rest: Vec<[usize; 3]> = (1..ring.len() - 1)
        .map(|k| [ring[0], ring[k], ring[k + 1]])
        .collect();
    let last_turns_right = ring.len() == 3 && rest.iter().all(|&t| ccw(flat, t));
    (last_turns_right || collinear(points, &rest)).then(|| {
        out.extend(rest);
        out
    })
}

/// A convex corner whose triangle holds no other ring vertex, on its edges included.
/// A vertex at the same place as one of the ear's corners does not block it, so a
/// polygon that touches itself there (a hole joined by a doubled bridge edge) still
/// clips. A predicate that cannot decide counts as inside.
fn is_ear(flat: &[DVec2], ring: &[usize], tri @ [a, b, c]: [usize; 3]) -> bool {
    if !ccw(flat, tri) {
        return false;
    }
    let corners = [flat[a], flat[b], flat[c]];
    let outside =
        |p: usize, q: usize, r: usize| orient2d(flat[p], flat[q], flat[r]) == Ok(Sign::Negative);
    ring.iter()
        .filter(|&&r| !corners.contains(&flat[r]))
        .all(|&r| outside(a, b, r) || outside(b, c, r) || outside(c, a, r))
}
