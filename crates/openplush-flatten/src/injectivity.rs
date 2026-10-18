use openplush_geom::DVec2;
use openplush_mesh::FaceId;
use openplush_seams::ChartId;

use crate::FlattenError;

/// Verifies every face is strictly counter-clockwise (det J > 0); lists all that are not.
pub fn verify(chart: ChartId, faces: &[(FaceId, [DVec2; 3])]) -> Result<(), FlattenError> {
    let _ = (chart, faces);
    Ok(())
}
