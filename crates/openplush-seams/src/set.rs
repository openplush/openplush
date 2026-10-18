//! The seam set: typed flags on mesh edges. The mesh never changes when seams do.

use std::collections::BTreeMap;

use openplush_mesh::EdgeId;

use crate::{SeamKind, SeamsError};

/// The seams of one mesh, each edge with its kind, in ascending `EdgeId` order.
///
/// **Invariant**: a [`SeamKind::Mandatory`] seam is never removed and never takes
/// another kind, because repair needs it to hold the surface together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeamSet {
    kind: BTreeMap<EdgeId, SeamKind>,
}

impl SeamSet {
    /// A set with no seams.
    pub fn new() -> Self {
        Self::default()
    }

    /// The kind of seam on `edge`, if there is one.
    pub fn get(&self, edge: EdgeId) -> Option<SeamKind> {
        self.kind.get(&edge).copied()
    }

    /// Number of seam edges.
    pub fn len(&self) -> usize {
        self.kind.len()
    }

    /// True if there are no seams.
    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    /// Every seam, in ascending edge order.
    pub fn iter(&self) -> impl Iterator<Item = (EdgeId, SeamKind)> + '_ {
        self.kind.iter().map(|(&edge, &kind)| (edge, kind))
    }

    /// Sets `edge`'s kind and returns the kind it replaced. Errors with
    /// [`SeamsError::MandatorySeam`] if `edge` is mandatory and `kind` is not.
    pub fn insert(&mut self, edge: EdgeId, kind: SeamKind) -> Result<Option<SeamKind>, SeamsError> {
        if self.get(edge) == Some(SeamKind::Mandatory) && kind != SeamKind::Mandatory {
            return Err(SeamsError::MandatorySeam { edge });
        }
        Ok(self.kind.insert(edge, kind))
    }

    /// Removes `edge`'s seam and returns its kind, or `None` if it had none. Errors
    /// with [`SeamsError::MandatorySeam`] if the seam is mandatory.
    pub fn remove(&mut self, edge: EdgeId) -> Result<Option<SeamKind>, SeamsError> {
        if self.get(edge) == Some(SeamKind::Mandatory) {
            return Err(SeamsError::MandatorySeam { edge });
        }
        Ok(self.kind.remove(&edge))
    }
}
