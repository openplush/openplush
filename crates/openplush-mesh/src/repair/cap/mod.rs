//! Caps: zero-area faces whose middle vertex lies on their longest edge.

mod clip;

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::epsilon::ModelTolerances;
use openplush_geom::{DVec2, DVec3};

use crate::halfedge::is_sliver;
use crate::{FaceId, SourceCorner, TriangleSoup};

/// What stitching changed, by the IDs of the soup it was given; each list sorted.
#[derive(Debug, Default)]
pub(crate) struct Stitched {
    pub(crate) caps: Vec<FaceId>,
    /// Faces re-triangulated with the caps' vertices on their edges.
    pub(crate) retriangulated: Vec<FaceId>,
    /// The input face each face of the returned soup came from, in its order.
    pub(crate) origin: Vec<FaceId>,
}

/// A polygon corner: its vertex and corner records, and the face edges it lies
/// on as bits (edge i runs from corner i to the next).
#[derive(Debug, Clone, Copy)]
struct Corner {
    v: u32,
    src: SourceCorner,
    uv: Option<DVec2>,
    on: u8,
}

/// Each cap's middle vertex, with the cap's record there, by the cap's longest
/// edge, ends sorted.
type OnEdge = BTreeMap<[u32; 2], Vec<(u32, SourceCorner)>>;

/// Drops every cap (three distinct vertices, area within the area tolerance).
/// Each other face is re-triangulated with every vertex the caps put on its
/// edges, so no crack opens, no vertex moves and no vertex is added. Faces that
/// repeat a vertex pass through; `degenerate::remove` drops them first. Within a
/// few weld distances of a face's corner, a vertex can land on two of its edges
/// or leave a zero-area triangle; the half-edge build refuses either, naming it.
pub(crate) fn stitch(soup: &TriangleSoup, tol: &ModelTolerances) -> (TriangleSoup, Stitched) {
    let tris = soup.triangle_indices();
    let caps: Vec<bool> = tris
        .iter()
        .map(|&t| is_cap(t, soup.positions(), tol))
        .collect();
    let on_edge = on_edge(soup, &caps);
    let mut stitched = Stitched::default();
    let mut out: Vec<[Corner; 3]> = Vec::with_capacity(tris.len());
    for (f, &cap) in caps.iter().enumerate() {
        // Lossless: the face count fits in u32 (`TriangleSoup::new`).
        let face = FaceId::new(f as u32);
        if cap {
            stitched.caps.push(face);
            continue;
        }
        let polygon = polygon(soup, f, &on_edge);
        if polygon.len() > 3 {
            stitched.retriangulated.push(face);
        }
        for t in clip::triangles(polygon) {
            out.push(t);
            stitched.origin.push(face);
        }
    }
    let corners = || out.iter().flatten();
    let rebuilt = soup.with_faces(
        out.iter().map(|t| t.map(|c| c.v)).collect(),
        corners().map(|c| c.src).collect(),
        soup.corner_uvs().map(|_| corners().map(|c| c.uv).collect()),
    );
    (rebuilt, stitched)
}

// INVARIANT: soup indices are in range (`TriangleSoup::new`).
fn is_cap([a, b, c]: [u32; 3], positions: &[DVec3], tol: &ModelTolerances) -> bool {
    let p = |v: u32| positions[v as usize];
    a != b && b != c && c != a && is_sliver(p(a), p(b), p(c), tol)
}

fn key(a: u32, b: u32) -> [u32; 2] {
    if a < b { [a, b] } else { [b, a] }
}

/// The corner of each cap opposite its first longest edge, by that edge.
// INVARIANT: soup indices are in range and corner records come three per face
// (`TriangleSoup::new`).
fn on_edge(soup: &TriangleSoup, caps: &[bool]) -> OnEdge {
    let p = |v: u32| soup.positions()[v as usize];
    let mut on_edge = OnEdge::new();
    for (f, _) in caps.iter().enumerate().filter(|&(_, &cap)| cap) {
        let t = soup.triangle_indices()[f];
        let length = |i: usize| (p(t[(i + 1) % 3]) - p(t[i])).length();
        let long = (1..3).fold(0, |best, i| if length(i) > length(best) { i } else { best });
        let mid = (long + 2) % 3;
        on_edge
            .entry(key(t[long], t[(long + 1) % 3]))
            .or_default()
            .push((t[mid], soup.corner_sources()[3 * f + mid]));
    }
    on_edge
}

/// Face `f`'s corners in its winding, with the vertices the caps put on each edge
/// between them, in order along it. A new corner takes the record of the first
/// cap found to put its vertex there, and the UV the face gives that point.
// INVARIANT: `f` is a face, so its indices and corner records exist
// (`TriangleSoup::new`).
fn polygon(soup: &TriangleSoup, f: usize, on_edge: &OnEdge) -> Vec<Corner> {
    let t = soup.triangle_indices()[f];
    let uv = |c: usize| soup.corner_uvs().and_then(|uv| uv[3 * f + c]);
    let mut polygon = Vec::with_capacity(3);
    for i in 0..3 {
        let j = (i + 1) % 3;
        polygon.push(Corner {
            v: t[i],
            src: soup.corner_sources()[3 * f + i],
            uv: uv(i),
            on: 1 << i | 1 << ((i + 2) % 3),
        });
        for (v, src, s) in points_on(t[i], t[j], on_edge, soup.positions()) {
            let uv = match (uv(i), uv(j)) {
                (Some(a), Some(b)) => Some(a + (b - a) * s),
                _ => None,
            };
            polygon.push(Corner {
                v,
                src,
                uv,
                on: 1 << i,
            });
        }
    }
    polygon
}

/// Every vertex the caps put on edge u-v, following caps on its parts, with its
/// record and its place along the edge from u (0 to 1), in that order.
// INVARIANT: soup indices are in range (`TriangleSoup::new`).
fn points_on(
    u: u32,
    v: u32,
    on_edge: &OnEdge,
    positions: &[DVec3],
) -> Vec<(u32, SourceCorner, f64)> {
    let mut found: BTreeMap<u32, SourceCorner> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![key(u, v)];
    while let Some(edge) = stack.pop() {
        if !seen.insert(edge) {
            continue;
        }
        for &(m, src) in on_edge.get(&edge).into_iter().flatten() {
            if m != u && m != v {
                found.entry(m).or_insert(src);
            }
            stack.push(key(edge[0], m));
            stack.push(key(m, edge[1]));
        }
    }
    // Ordered from the edge's smaller vertex, then reversed for the face walking it
    // the other way, so both faces on an edge get the same order even where two
    // vertices tie.
    let p = |w: u32| positions[w as usize];
    let [lo, hi] = key(u, v);
    let edge = p(hi) - p(lo);
    let mut points: Vec<_> = found
        .into_iter()
        .map(|(w, src)| (w, src, (p(w) - p(lo)).dot(edge) / edge.length_squared()))
        .collect();
    points.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)));
    if u != lo {
        points.reverse();
        points.iter_mut().for_each(|point| point.2 = 1.0 - point.2);
    }
    points
}

#[cfg(test)]
mod tests;
