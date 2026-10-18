//! Dropping the vertices no triangle uses.

use super::TriangleSoup;

impl TriangleSoup {
    /// This soup without the vertices no triangle uses, the rest keeping their order, and
    /// each old vertex's new index. For a vertex that was dropped the entry is only the
    /// count of kept vertices below it, so look up used vertices only.
    pub(crate) fn compacted(&self) -> (Self, Vec<u32>) {
        let mut used = vec![false; self.positions.len()];
        for &v in self.triangles.iter().flatten() {
            // INVARIANT: triangle indices are below the vertex count (`TriangleSoup::new`).
            used[v as usize] = true;
        }
        let mut rank = Vec::with_capacity(used.len());
        let mut positions = Vec::new();
        for (position, &is_used) in self.positions.iter().zip(&used) {
            // Lossless: the vertex count fits in u32 (`TriangleSoup::new`).
            rank.push(positions.len() as u32);
            if is_used {
                positions.push(*position);
            }
        }
        let triangles = self
            .triangles
            .iter()
            // INVARIANT: triangle indices are below the vertex count, so they have a rank.
            .map(|t| t.map(|v| rank[v as usize]))
            .collect();
        let compact = Self {
            positions,
            triangles,
            ..self.clone()
        };
        (compact, rank)
    }
}
