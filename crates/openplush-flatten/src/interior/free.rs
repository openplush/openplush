//! Which vertices the solve may move, and the number each gets.

use std::collections::BTreeSet;

use openplush_mesh::{FaceId, VertexId};

/// The interior vertices of a chart, numbered in vertex order; the rest stay where they are.
pub(super) struct Free {
    /// Per vertex: its number among the free ones, or `None` for a fixed or unused vertex.
    pub(super) slot: Vec<Option<usize>>,
    pub(super) count: usize,
}

impl Free {
    /// Every vertex of `faces` not in `fixed` is free. Vertices are indices below `vertices`.
    pub(super) fn new(
        vertices: usize,
        faces: &[(FaceId, [VertexId; 3])],
        fixed: &[VertexId],
    ) -> Self {
        let pinned: BTreeSet<u32> = fixed.iter().map(|v| v.index()).collect();
        let used: BTreeSet<u32> = faces
            .iter()
            .flat_map(|(_, t)| t.map(VertexId::index))
            .filter(|v| !pinned.contains(v))
            .collect();
        let mut slot = vec![None; vertices];
        for (n, v) in used.iter().enumerate() {
            // INVARIANT: the caller checked every face vertex has a position, so `v < vertices`.
            slot[*v as usize] = Some(n);
        }
        Self {
            slot,
            count: used.len(),
        }
    }

    /// The number of the free vertex `v`.
    pub(super) fn of(&self, v: VertexId) -> Option<usize> {
        self.slot.get(v.index() as usize).copied().flatten()
    }
}
