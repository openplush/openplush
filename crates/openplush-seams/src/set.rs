//! The seam set: typed flags on mesh edges. The mesh never changes when seams do.

use std::collections::BTreeMap;

use openplush_mesh::{EdgeId, ManifoldMesh};

use crate::{SeamKind, SeamsError};

/// The seams of one mesh, each edge with its kind, in ascending `EdgeId` order.
///
/// **Invariant**: a [`SeamKind::Mandatory`] seam is never removed and never takes
/// another kind, and it is only placed on an edge with no seam. Repair makes them
/// on the edges it splits, so a maker's seam can never become one that undo cannot
/// remove.
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
    /// [`SeamsError::MandatorySeam`] if `edge` is mandatory and `kind` is not, or
    /// `kind` is mandatory and `edge` has another seam.
    pub fn insert(&mut self, edge: EdgeId, kind: SeamKind) -> Result<Option<SeamKind>, SeamsError> {
        let current = self.get(edge);
        let mandatory = SeamKind::Mandatory;
        // An existing seam may be replaced only by a kind on the same side of `Mandatory`.
        if (current == Some(mandatory)) != (kind == mandatory) && current.is_some() {
            return Err(SeamsError::MandatorySeam { edge });
        }
        Ok(self.kind.insert(edge, kind))
    }

    /// Errors with [`SeamsError::UnknownEdge`], naming the lowest, when a seam is on an edge
    /// `mesh` does not have.
    pub(crate) fn check_edges(&self, mesh: &ManifoldMesh) -> Result<(), SeamsError> {
        match self
            .kind
            .keys()
            .find(|e| e.index() as usize >= mesh.edge_count())
        {
            Some(&edge) => Err(SeamsError::UnknownEdge { edge }),
            None => Ok(()),
        }
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
