//! The two constructions under test.
//!
//! `strips`: the per-edge construction the engine would use. One rectangle per edge on its
//! outer side, one miter (or bevel) wedge per convex corner, one guard over each
//! zero-allowance edge, then (outline ∪ pieces) − guards in one i_overlay call.
//! `uniform_outline`: i_overlay's own `outline` offset, which only takes one distance.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::integer::OverlayInt;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::FloatOverlay;
use i_overlay::i_shape::base::data::Shapes;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::style::{LineJoin, OutlineStyle};

use crate::geom::{P, dist, orient, signed_area};

/// A wedge whose miter point is farther than this × its larger allowance becomes a bevel.
/// 4 is the SVG/PostScript default; it bevels corners sharper than 2·asin(1/4) ≈ 29°.
pub const MITER_LIMIT: f64 = 4.0;

fn unit_dir(a: P, b: P) -> P {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    [dx / len, dy / len]
}

/// Outward normal of a CCW edge with direction `d`.
fn outward(d: P) -> P {
    [d[1], -d[0]]
}

fn add(p: P, v: P, s: f64) -> P {
    [p[0] + v[0] * s, p[1] + v[1] * s]
}

/// Convex, CCW pieces whose union with the outline is the cut region; empty pieces are dropped.
pub fn pieces(outline: &[P], allowance: &[f64]) -> Vec<Vec<P>> {
    let n = outline.len();
    let mut out = Vec::with_capacity(2 * n);
    for i in 0..n {
        let (v0, v1, v2) = (outline[i], outline[(i + 1) % n], outline[(i + 2) % n]);
        let (ai, aj) = (allowance[i], allowance[(i + 1) % n]);
        let (di, dj) = (unit_dir(v0, v1), unit_dir(v1, v2));
        let (ni, nj) = (outward(di), outward(dj));
        if ai > 0.0 {
            out.push(vec![v0, add(v0, ni, ai), add(v1, ni, ai), v1]);
        }
        if orient(v0, v1, v2) > 0.0 && ai.max(aj) > 0.0 {
            out.push(corner(v1, (di, ni, ai), (dj, nj, aj)));
        }
    }
    // A zero allowance makes a wedge corner coincide with the vertex.
    out.iter_mut().for_each(|p| p.dedup());
    out.retain(|p| p.len() >= 3 && p.first() != p.last() && signed_area(p) > 0.0);
    out
}

/// One rectangle over the outer side of every zero-allowance edge (fold or edge-stitched),
/// as deep as any piece reaches: no allowance may put fabric there.
pub fn fold_guards(outline: &[P], allowance: &[f64]) -> Vec<Vec<P>> {
    let n = outline.len();
    let depth = MITER_LIMIT * allowance.iter().copied().fold(0.0, f64::max);
    (0..n)
        .filter(|&i| allowance[i] == 0.0 && depth > 0.0)
        .map(|i| {
            let (v0, v1) = (outline[i], outline[(i + 1) % n]);
            let nrm = outward(unit_dir(v0, v1));
            vec![v0, add(v0, nrm, depth), add(v1, nrm, depth), v1]
        })
        .collect()
}

/// The wedge at a convex vertex `v` between the incoming and outgoing edges' offsets.
fn corner(v: P, (di, ni, ai): (P, P, f64), (dj, nj, aj): (P, P, f64)) -> Vec<P> {
    let (a, b) = (add(v, ni, ai), add(v, nj, aj));
    let cross = di[0] * dj[1] - di[1] * dj[0];
    let t = ((b[0] - a[0]) * dj[1] - (b[1] - a[1]) * dj[0]) / cross;
    let s = ((b[0] - a[0]) * di[1] - (b[1] - a[1]) * di[0]) / cross;
    let x = add(a, di, t);
    let reach = dist(x, v);
    // The miter point must lie ahead of `a` on the incoming offset and behind `b` on the
    // outgoing one; unequal allowances can violate either, and the quad would then be a bowtie.
    if t.is_finite() && t >= 0.0 && s <= 0.0 && reach <= MITER_LIMIT * ai.max(aj) {
        vec![v, a, x, b]
    } else {
        vec![v, a, b]
    }
}

/// Drops every edge shorter than `eps` (its end vertex goes, the next edge keeps its allowance).
/// The caller must re-validate: merging can make a simple outline touch itself.
pub fn merge_short_edges(outline: &[P], allowance: &[f64], eps: f64) -> (Vec<P>, Vec<f64>) {
    let (mut pts, mut alw) = (outline.to_vec(), allowance.to_vec());
    let mut i = 0;
    while i < pts.len() && pts.len() > 3 {
        let j = (i + 1) % pts.len();
        let (p, q) = (pts[i], pts[j]);
        if dist(p, q) < eps {
            pts.remove(j);
            alw.remove(i);
            if j == 0 {
                // The merged edge now ends the list; old edge 0 was its second half.
                alw.rotate_left(1);
                i -= 1;
            }
        } else {
            i += 1;
        }
    }
    (pts, alw)
}

/// (outline ∪ pieces) − fold guards in one overlay, NonZero fill, on integer engine `I`.
/// One overlay means one float-to-integer grid, so a fold edge and its guard's inner edge
/// stay identical; two passes on two grids leave sub-grid slivers along every fold.
/// Precondition: `guards_clear`, or the guards would cut the outline itself.
pub fn strips<I: OverlayInt>(outline: &[P], allowance: &[f64]) -> Shapes<P> {
    let mut subj = vec![outline.to_vec()];
    subj.extend(pieces(outline, allowance));
    let guards = fold_guards(outline, allowance);
    FloatOverlay::<P, I>::from_subj_and_clip(&subj, &guards)
        .overlay(OverlayRule::Difference, FillRule::NonZero)
}

/// No guard reaches into the outline: no outline vertex inside one, no edge crossing its far sides.
pub fn guards_clear(outline: &[P], guards: &[Vec<P>]) -> bool {
    let n = outline.len();
    guards.iter().all(|g| {
        let strictly_inside = |p: P| (0..4).all(|k| orient(g[k], g[(k + 1) % 4], p) > 0.0);
        let far_sides = [(g[0], g[1]), (g[1], g[2]), (g[2], g[3])];
        !outline.iter().any(|&p| strictly_inside(p))
            && !(0..n).any(|i| {
                let (p, q) = (outline[i], outline[(i + 1) % n]);
                far_sides.iter().any(|&(a, b)| proper_cross(p, q, a, b))
            })
    })
}

fn proper_cross(a: P, b: P, c: P, d: P) -> bool {
    orient(a, b, c) * orient(a, b, d) < 0.0 && orient(c, d, a) * orient(c, d, b) < 0.0
}

/// i_overlay's single-distance outline offset with the same miter limit.
pub fn uniform_outline<I: OverlayInt + 'static>(outline: &[P], offset: f64) -> Shapes<P> {
    let min_angle = 2.0 * libm::asin(1.0 / MITER_LIMIT);
    let style = OutlineStyle::new(offset).line_join(LineJoin::Miter(min_angle));
    outline.outline_as::<I>(&style)
}
