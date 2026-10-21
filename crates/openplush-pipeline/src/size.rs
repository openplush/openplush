use openplush_geom::DVec3;
use openplush_geom::units::Scale;
use openplush_mesh::{ManifoldMesh, VertexId};
use openplush_pattern::PatternError;
use openplush_pattern::scale::{self, MAX_MM, MIN_MM, SizeRange, SizeTarget};

use crate::{
    EngineError, NonBlank, Remedy, StageResult, Staged, Warning, WarningCode, WarningSet,
    WarningTarget,
};

/// The scale that makes `mesh` the size `target` asks for, judging its longest dimension as the
/// longest side of its axis-aligned bounding box in the file's axes.
///
/// Raises `size-out-of-range` when that dimension finishes under 75 mm or over 1000 mm.
///
/// # Errors
/// `Pattern`, locating nothing, for a reference length that is not finite and positive, or a
/// scale or finished size that is zero or not finite.
pub fn size(mesh: &ManifoldMesh, target: SizeTarget) -> StageResult<Scale> {
    let longest = longest_side(mesh);
    let refused = |source| EngineError::Pattern {
        piece: None,
        source,
    };
    let (value, range) = scale::size(longest, target).map_err(refused)?;
    let mut warnings = WarningSet::default();
    if range != SizeRange::Within {
        let finished = value
            .to_mm(longest)
            .map_err(|source| refused(PatternError::BadScale { source }))?;
        warnings.insert(out_of_range(finished.get()));
    }
    Ok(Staged { value, warnings })
}

fn longest_side(mesh: &ManifoldMesh) -> f64 {
    // Lossless: a mesh's vertex count fits in u32, as its IDs do.
    let mut points = (0..mesh.vertex_count() as u32).map(|v| mesh.position(VertexId::new(v)));
    let Some(first) = points.next() else {
        return 0.0;
    };
    let (lo, hi) = points.fold((first, first), |(lo, hi), p| {
        (
            DVec3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z)),
            DVec3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z)),
        )
    });
    let side = hi - lo;
    side.x.max(side.y).max(side.z)
}

fn out_of_range(finished_mm: f64) -> Warning {
    // INVARIANT: both texts are non-blank literals or built around them.
    let text = |t: String| NonBlank::new(t).unwrap_or_else(|| unreachable!());
    Warning::new(
        WarningCode::SizeOutOfRange,
        WarningTarget::Project,
        text(format!(
            "The pattern's longest dimension finishes at {} mm, outside the supported {MIN_MM} \
             mm to {MAX_MM} mm.",
            outside(finished_mm)
        )),
        Remedy::Advisory {
            what_to_change: text(format!(
                "Resize the model so its longest dimension is between {MIN_MM} mm and {MAX_MM} \
                 mm, or keep this size: the pattern is still made, but untested at this size."
            )),
        },
    )
}

/// `mm` to one decimal, or to as many more as it takes to print a number outside the range, so
/// the text never reads `1000.0 mm, outside ... 1000 mm`.
fn outside(mm: f64) -> String {
    (1..=6)
        .map(|digits| format!("{mm:.digits$}"))
        .find(|s| {
            s.parse::<f64>()
                .is_ok_and(|v| !(MIN_MM..=MAX_MM).contains(&v))
        })
        .unwrap_or_else(|| mm.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_texts_print_in_the_pdf_s_fonts() {
        let w = out_of_range(1234.5);
        let Remedy::Advisory { what_to_change } = w.remedy() else {
            panic!("the remedy is not advisory");
        };
        for text in [w.explanation(), what_to_change.as_str()] {
            assert!(text.chars().all(|c| (' '..='~').contains(&c)), "{text}");
        }
    }

    #[test]
    fn a_size_just_outside_the_range_prints_enough_digits_to_show_it() {
        for (mm, shown) in [
            (1000.04, "1000.04 mm"),
            (74.96, "74.96 mm"),
            (74.9, "74.9 mm"),
        ] {
            let w = out_of_range(mm);
            assert!(
                w.explanation().contains(&format!("at {shown},")),
                "{}",
                w.explanation()
            );
        }
    }
}
