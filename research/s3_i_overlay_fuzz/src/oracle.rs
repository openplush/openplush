//! Checks i_overlay's result against facts computed without i_overlay.
//!
//! The reference region is (outline ∪ pieces) − guards, where every piece and guard is
//! convex, so membership is a winding test plus half-plane tests. Points closer than `band`
//! to any reference edge are skipped: there the answer depends on rounding.

use std::collections::BTreeSet;

use crate::allowance::{self, MITER_LIMIT};
use crate::generate::Rng;
use crate::geom::{
    P, bbox, bbox_diag, dist, dist_to_boundary, dist_to_segment, first_self_contact, orient,
    segments_meet, winding,
};

/// What is wrong with one result. A case can carry several.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    /// i_overlay panicked; the payload is the panic location and message.
    Panic(String),
    Empty,
    /// More than one outer contour; the union of pieces that each touch the outline is connected.
    Disconnected,
    /// A hole with a vertex on the outline: a pocket the allowances close over an outline edge,
    /// typically a short zero-allowance edge between allowed neighbours.
    HoleAtOutline,
    /// A hole clear of the outline: allowances that close over a concave notch.
    HoleDetached,
    /// A result contour passes twice through one vertex: two regions joined at a point.
    /// i_overlay's default (non-OGC) output allows it; the cut line must not.
    Pinch,
    /// A result contour crosses or overlaps itself other than at a shared vertex.
    NonSimple,
    /// A zero-allowance edge (fold, edge-stitched) is not on the cut line: another edge's
    /// allowance covers it. Consistent with the reference, but a conflict the engine must report.
    FoldCovered,
    /// A fold guard reaches into the outline: the fold faces another part of the piece closer
    /// than the allowance reach, typically a reflex corner sharper than 90° beside it.
    /// A conflict the engine must report; the boolean still runs and is still checked.
    GuardHitsOutline,
    /// Merging short edges left an outline that is no longer simple; not run.
    InvalidAfterMerge,
    /// A point the reference covers is outside the result: lost outline or allowance.
    Missing,
    /// A point the reference leaves out is inside the result: allowance where none belongs.
    Extra,
}

/// Relative size of the ambiguity band and of sample offsets, as fractions of the bbox diagonal.
const BAND: f64 = 1e-7;
const STEP: f64 = 1e-6;
const ALONG: [f64; 5] = [0.01, 0.25, 0.5, 0.75, 0.99];
const RANDOM_SAMPLES: usize = 64;

fn normal(a: P, b: P) -> P {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    [dy / len, -dx / len]
}

fn inside_convex(piece: &[P], p: P) -> bool {
    (0..piece.len()).all(|k| orient(piece[k], piece[(k + 1) % piece.len()], p) > 0.0)
}

/// Segment `s`–`t` reaches the interior or boundary of convex `g`.
fn ray_enters(g: &[P], s: P, t: P) -> bool {
    inside_convex(g, s)
        || inside_convex(g, t)
        || (0..g.len()).any(|k| segments_meet(s, t, g[k], g[(k + 1) % g.len()]))
}

fn topology(outline: &[P], band: f64, result: &[Vec<Vec<P>>], faults: &mut BTreeSet<Fault>) {
    match result.len() {
        0 => {
            faults.insert(Fault::Empty);
        }
        1 => {}
        _ => {
            faults.insert(Fault::Disconnected);
        }
    }
    for hole in result.iter().flat_map(|shape| shape.iter().skip(1)) {
        let touches = hole.iter().any(|&v| dist_to_boundary(outline, v) < band);
        faults.insert(if touches {
            Fault::HoleAtOutline
        } else {
            Fault::HoleDetached
        });
    }
    for c in result.iter().flatten() {
        let n = c.len();
        match first_self_contact(c) {
            _ if n < 3 => faults.insert(Fault::NonSimple),
            None => false,
            Some((i, j)) => {
                let shared = [c[i], c[(i + 1) % n]]
                    .iter()
                    .any(|p| *p == c[j] || *p == c[(j + 1) % n]);
                faults.insert(if shared {
                    Fault::Pinch
                } else {
                    Fault::NonSimple
                })
            }
        };
    }
}

/// Faults of the per-edge strip construction.
pub fn check_strips(outline: &[P], allowance: &[f64], result: &[Vec<Vec<P>>]) -> Verdict {
    let (pieces, guards) = (
        allowance::pieces(outline, allowance),
        allowance::fold_guards(outline, allowance),
    );
    let mut faults = BTreeSet::new();
    let diag = bbox_diag(outline.iter().chain(pieces.iter().flatten()).copied());
    let (band, step) = (BAND * diag, STEP * diag);
    topology(outline, band, result, &mut faults);
    let res: Vec<&[P]> = result.iter().flatten().map(Vec::as_slice).collect();
    let near_ref = |p: P| {
        dist_to_boundary(outline, p) < band
            || pieces
                .iter()
                .chain(&guards)
                .any(|q| dist_to_boundary(q, p) < band)
    };
    let in_ref = |p: P| {
        (winding(&[outline], p) != 0 || pieces.iter().any(|q| inside_convex(q, p)))
            && !guards.iter().any(|g| inside_convex(g, p))
    };
    let mut judge = |p: P| {
        if near_ref(p) {
            return;
        }
        match (in_ref(p), winding(&res, p) != 0) {
            (true, false) => faults.insert(Fault::Missing),
            (false, true) => faults.insert(Fault::Extra),
            _ => false,
        };
    };
    let n = outline.len();
    let mut covered_fold = false;
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let nrm = normal(a, b);
        for t in ALONG {
            let s = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            for d in [-step, step, allowance[i] - step, allowance[i] + step] {
                judge([s[0] + d * nrm[0], s[1] + d * nrm[1]]);
            }
            let out = [s[0] + step * nrm[0], s[1] + step * nrm[1]];
            if allowance[i] == 0.0 && !near_ref(out) && winding(&res, out) != 0 {
                covered_fold = true;
            }
        }
    }
    let mut rng = Rng::new(n as u64);
    for q in &pieces {
        let w: Vec<f64> = q.iter().map(|_| rng.unit() + 1e-3).collect();
        let total: f64 = w.iter().sum();
        let c = q.iter().zip(&w).fold([0.0, 0.0], |acc, (p, w)| {
            [acc[0] + p[0] * w / total, acc[1] + p[1] * w / total]
        });
        judge(c);
    }
    // Probe every region i_overlay emitted, and uniformly over both sides' bbox, so stray
    // area cannot hide.
    for c in &res {
        for k in 0..c.len() {
            let (p, q, r) = (c[k], c[(k + 1) % c.len()], c[(k + 2) % c.len()]);
            judge([(p[0] + q[0] + r[0]) / 3.0, (p[1] + q[1] + r[1]) / 3.0]);
        }
    }
    let (lo, hi) = bbox(
        outline
            .iter()
            .chain(pieces.iter().flatten())
            .chain(res.iter().copied().flatten())
            .copied(),
    );
    for _ in 0..RANDOM_SAMPLES {
        judge([rng.range(lo[0], hi[0]), rng.range(lo[1], hi[1])]);
    }
    if covered_fold {
        faults.insert(Fault::FoldCovered);
    }
    if !allowance::guards_clear(outline, &guards) {
        faults.insert(Fault::GuardHitsOutline);
    }
    let [shortfall, shortfall_long] = shortfall(outline, |i| allowance[i], &guards, &res);
    Verdict {
        faults,
        shortfall,
        shortfall_long,
    }
}

/// Faults of i_overlay's uniform `outline` offset by `a`: the band [0, a) outside every edge
/// must be covered, and no output vertex may sit inside the outline or beyond the miter reach.
pub fn check_uniform(outline: &[P], a: f64, result: &[Vec<Vec<P>>]) -> Verdict {
    let mut faults = BTreeSet::new();
    let diag = bbox_diag(outline.iter().copied()) + 2.0 * MITER_LIMIT * a;
    let (band, step) = (BAND * diag, STEP * diag);
    topology(outline, band, result, &mut faults);
    let res: Vec<&[P]> = result.iter().flatten().map(Vec::as_slice).collect();
    let n = outline.len();
    for i in 0..n {
        let (p, q) = (outline[i], outline[(i + 1) % n]);
        let nrm = normal(p, q);
        for t in ALONG {
            let s = [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])];
            for d in [-step, step, a - step] {
                let x = [s[0] + d * nrm[0], s[1] + d * nrm[1]];
                if dist_to_boundary(outline, x) > band && winding(&res, x) == 0 {
                    faults.insert(Fault::Missing);
                }
            }
        }
    }
    for &v in res.iter().copied().flatten() {
        let d = (0..n)
            .map(|i| dist_to_segment(v, outline[i], outline[(i + 1) % n]))
            .fold(f64::INFINITY, f64::min);
        let inside = winding(&[outline], v) != 0;
        if d > MITER_LIMIT * a + band || (inside && d > band) {
            faults.insert(Fault::Extra);
        }
    }
    let [shortfall, shortfall_long] = shortfall(outline, |_| a, &[], &res);
    Verdict {
        faults,
        shortfall,
        shortfall_long,
    }
}

/// The faults found, and the worst allowance shortfall in mm.
pub struct Verdict {
    pub faults: BTreeSet<Fault>,
    /// Largest `a − depth` over edge midpoints, where `depth` is how far along the outward
    /// normal the result covers, found by bisection on [0, a].
    pub shortfall: f64,
    /// The same over edges at least `LONG_EDGE` × the result's bbox diagonal long.
    pub shortfall_long: f64,
}

/// Edges shorter than this fraction of the diagonal are a few integer grid cells at most.
pub const LONG_EDGE: f64 = 1e-6;

/// Edges whose normal ray at the midpoint enters a fold guard are skipped: there the fold wins.
fn shortfall(
    outline: &[P],
    allowance: impl Fn(usize) -> f64,
    guards: &[Vec<P>],
    res: &[&[P]],
) -> [f64; 2] {
    let n = outline.len();
    let diag = bbox_diag(res.iter().copied().flatten().copied());
    let mut worst = [0.0_f64; 2];
    for i in 0..n {
        let (p, q, a) = (outline[i], outline[(i + 1) % n], allowance(i));
        let nrm = normal(p, q);
        let s = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
        let tip = [s[0] + a * nrm[0], s[1] + a * nrm[1]];
        let covered = |d: f64| winding(res, [s[0] + d * nrm[0], s[1] + d * nrm[1]]) != 0;
        if a <= 0.0 || covered(a) || guards.iter().any(|g| ray_enters(g, s, tip)) {
            continue;
        }
        let (mut lo, mut hi) = (0.0, a);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if covered(mid) { lo = mid } else { hi = mid }
        }
        worst[0] = worst[0].max(a - lo);
        if dist(p, q) >= LONG_EDGE * diag {
            worst[1] = worst[1].max(a - lo);
        }
    }
    worst
}
