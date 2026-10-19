use openplush_geom::predicates::orient2d;
use openplush_geom::{DVec2, Sign};
use openplush_mesh::FaceId;
use openplush_seams::ChartId;

use crate::FlattenError;

/// Verifies every face is strictly counter-clockwise (det J > 0); lists all that are not.
///
/// Faces carry the winding they have on the 3D mesh. A face the exact predicate cannot judge
/// (non-finite or extreme coordinates) is an error of its own, never counted as a pass.
pub fn verify(chart: ChartId, faces: &[(FaceId, [DVec2; 3])]) -> Result<(), FlattenError> {
    if faces.is_empty() {
        return Err(FlattenError::EmptyChart { chart });
    }
    let mut inverted = Vec::new();
    for &(face, [a, b, c]) in faces {
        match orient2d(a, b, c) {
            Ok(Sign::Positive) => {}
            Ok(_) => inverted.push(face),
            Err(source) => {
                return Err(FlattenError::Unjudgeable {
                    chart,
                    face,
                    source,
                });
            }
        }
    }
    if inverted.is_empty() {
        Ok(())
    } else {
        Err(FlattenError::Flip {
            chart,
            faces: inverted,
        })
    }
}
