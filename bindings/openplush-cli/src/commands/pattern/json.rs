//! `pattern.json`: the printed pieces in millimetres with their cuts and distortion, and every
//! warning in the words the print uses.

use openplush_pipeline::{FaceId, Outline, Parameterization, PatternPiece, Summary, WarningSet};
use serde_json::{Value, json};

/// Lengths to the micrometre, as the SVG prints them.
const MM: f64 = 1e3;
/// Ratios to one part in a million.
const RATIO: f64 = 1e6;

/// The file's contents. `layouts` are the flatten stage's, one per chart.
pub(super) fn pattern_json(
    pieces: &[PatternPiece],
    layouts: &[Parameterization],
    warnings: &WarningSet,
) -> Value {
    let points = |o: &Outline| -> Vec<Value> {
        o.points()
            .iter()
            .map(|p| json!([round(p.x, MM), round(p.y, MM)]))
            .collect()
    };
    let pieces: Vec<Value> = pieces
        .iter()
        .map(|p| {
            // INVARIANT: every piece is numbered as a chart the flatten stage laid out.
            let layout = layouts
                .iter()
                .find(|l| l.chart().index() == p.id().index())
                .unwrap_or_else(|| unreachable!());
            json!({
                "piece": p.id().index(),
                "cuts": { "as_printed": p.cuts().as_printed(), "flipped": p.cuts().flipped() },
                "sewing_line_mm": points(p.sewing_line()),
                "cut_line_mm": points(p.cut_line()),
                "distortion": distortion(layout.summary()),
            })
        })
        .collect();
    let warnings: Vec<Value> = warnings
        .iter()
        .map(|w| {
            json!({
                "code": w.code().as_str(),
                "target": w.target().to_string(),
                "explanation": w.explanation(),
                "remedy": w.remedy().text(),
            })
        })
        .collect();
    json!({ "pieces": pieces, "warnings": warnings })
}

/// `v` to the nearest `1 / per`, so last-bit float differences between platforms do not show;
/// adding 0 turns `-0` into `0`.
fn round(v: f64, per: f64) -> f64 {
    (v * per).round() / per + 0.0
}

fn distortion(s: &Summary) -> Value {
    let face =
        |(id, ratio): (FaceId, f64)| json!({ "face": id.index(), "ratio": round(ratio, RATIO) });
    json!({
        "most_stretched": face(s.most_stretched()),
        "most_compressed": face(s.most_compressed()),
        "tail_stretch": round(s.tail_stretch(), RATIO),
        "tail_compression": round(s.tail_compression(), RATIO),
        "stretch_excess": round(s.stretch_excess(), RATIO),
        "compression_deficit": round(s.compression_deficit(), RATIO),
        "area_ratio": round(s.area_ratio(), RATIO),
        "stopped_by": s.stopped_by().as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::{MM, round};

    #[test]
    fn rounding_keeps_a_micrometre_and_never_gives_minus_zero() {
        assert_eq!(round(-48.426_406_610_106_184, MM), -48.426);
        assert_eq!(round(1.000_499_9, MM), 1.0);
        assert!(round(-0.000_2, MM).is_sign_positive());
    }
}
