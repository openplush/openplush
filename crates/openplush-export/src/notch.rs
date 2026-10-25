//! Notches as printed ticks, in the piece's millimetres.

use openplush_geom::DVec2;
use openplush_pattern::PatternPiece;

/// Space between the ticks of one notch.
const TICK_GAP_MM: f64 = 1.5;
/// The shortest tick, so a notch shows where the allowance is narrow or zero.
const MIN_TICK_MM: f64 = 3.0;

/// Every tick of every notch on `piece`: per notch, `marks` ticks side by side `TICK_GAP_MM`
/// apart and centred on it, each from the sewing line out to the cut line, or `MIN_TICK_MM`
/// where that is nearer.
pub(crate) fn ticks(piece: &PatternPiece) -> Vec<[DVec2; 2]> {
    let mut out = Vec::new();
    for n in piece.notches() {
        let out_dir = n.outward();
        let along = DVec2::new(-out_dir.y, out_dir.x);
        let middle = f64::from(n.marks() - 1) / 2.0;
        for k in 0..n.marks() {
            let start = n.at() + along * ((f64::from(k) - middle) * TICK_GAP_MM);
            let reach = to_cut_line(piece, start, out_dir).max(MIN_TICK_MM);
            out.push([start, start + out_dir * reach]);
        }
    }
    out
}

/// How far `from` goes along `dir` before it meets the cut line; zero if it starts on the cut
/// line, where a zero allowance puts it, or never meets it.
fn to_cut_line(piece: &PatternPiece, from: DVec2, dir: DVec2) -> f64 {
    let points = piece.cut_line().points();
    let n = points.len();
    let edges = || (0..n).map(|i| (points[i], points[(i + 1) % n]));
    // Off by less than a micrometre is on the line: the cut line is the sewing line there.
    let on_line = edges().any(|(a, b)| {
        let t = ((from - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
        (a + (b - a) * t - from).length() <= 1e-3
    });
    if on_line {
        return 0.0;
    }
    (0..n)
        .filter_map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let edge = b - a;
            let denom = dir.perp_dot(edge);
            if denom == 0.0 {
                return None;
            }
            let s = (a - from).perp_dot(edge) / denom;
            let u = (a - from).perp_dot(dir) / denom;
            (s > 0.0 && (0.0..=1.0).contains(&u)).then_some(s)
        })
        .min_by(f64::total_cmp)
        .unwrap_or(0.0)
}
