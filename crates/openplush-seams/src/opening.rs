//! The stuffing opening: a seam chain left unsewn.

use crate::{SeamChain, SeamKind, SeamSet, SeamsError};

/// Marks every edge of `chain` an [`SeamKind::Opening`] in `seams`, or none of them.
///
/// # Errors
/// Checked in `chain`'s walking order, before anything changes: `MandatorySeam` for a mandatory
/// edge, and `NotASeam` for an edge with no seam in `seams` (a chain from an older seam graph).
pub fn mark_opening(seams: &mut SeamSet, chain: &SeamChain) -> Result<(), SeamsError> {
    for &edge in chain.edges() {
        match seams.get(edge) {
            None => return Err(SeamsError::NotASeam { edge }),
            Some(SeamKind::Mandatory) => return Err(SeamsError::MandatorySeam { edge }),
            Some(_) => {}
        }
    }
    for &edge in chain.edges() {
        seams.insert(edge, SeamKind::Opening)?;
    }
    Ok(())
}
