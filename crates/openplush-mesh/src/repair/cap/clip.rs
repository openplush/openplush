//! Triangulating a face whose edges carry extra vertices.

use super::Corner;

/// Ear-clips `polygon`, a face's corners and the vertices on its edges in its
/// winding, into triangles wound the same way. No triangle has all three corners
/// on one face edge, and that is read from `Corner::on`, not recomputed.
#[allow(
    clippy::expect_used,
    reason = "an ear always exists; see the INVARIANT below"
)]
pub(super) fn triangles(mut polygon: Vec<Corner>) -> Vec<[Corner; 3]> {
    let mut out = Vec::with_capacity(polygon.len().saturating_sub(2));
    while polygon.len() > 3 {
        let n = polygon.len();
        // INVARIANT: the polygon has more than three corners on the three face
        // edges, not all on one. If clipping a corner would leave only one edge's
        // run, the polygon is that run plus the corner, and clipping an end of
        // the run instead leaves the corner and two of the run.
        let ear = (0..n)
            .find(|&i| is_ear(&polygon, i))
            .expect("a polygon on three lines, not all on one, has an ear");
        let (prev, next) = ((ear + n - 1) % n, (ear + 1) % n);
        out.push([polygon[prev], polygon[ear], polygon[next]]);
        polygon.remove(ear);
    }
    if let [a, b, c] = polygon[..] {
        out.push([a, b, c]);
    }
    out
}

/// True if corners `i - 1`, `i`, `i + 1` are not on one edge and neither is the
/// polygon left without `i`.
fn is_ear(polygon: &[Corner], i: usize) -> bool {
    let n = polygon.len();
    let ear = polygon[(i + n - 1) % n].on & polygon[i].on & polygon[(i + 1) % n].on;
    let rest = polygon
        .iter()
        .enumerate()
        .filter(|&(k, _)| k != i)
        .fold(0b111, |on, (_, c)| on & c.on);
    ear == 0 && rest == 0
}
