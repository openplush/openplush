//! Dropping the vertices no triangle uses.

use super::TriangleSoup;

impl TriangleSoup {
    /// This soup without the vertices no triangle uses, the rest keeping their order, and
    /// each old vertex's new index (`None` if dropped).
    pub(crate) fn compacted(&self) -> (Self, Vec<Option<u32>>) {
        let mut used = vec![false; self.positions.len()];
        for &v in self.triangles.iter().flatten() {
            // INVARIANT: triangle indices are below the vertex count (`TriangleSoup::new`).
            used[v as usize] = true;
        }
        let mut map: Vec<Option<u32>> = vec![None; used.len()];
        let mut positions = Vec::new();
        for (i, &u) in used.iter().enumerate().filter(|&(_, &u)| u) {
            let _ = u;
            // Lossless: the vertex count fits in u32 (`TriangleSoup::new`).
            map[i] = Some(positions.len() as u32);
            positions.push(self.positions[i]);
        }
        let triangles = self
            .triangles
            .iter()
            // INVARIANT: every triangle vertex is used, so it has an entry.
            .map(|t| t.map(|v| map[v as usize].unwrap_or(v)))
            .collect();
        let compact = Self {
            positions,
            triangles,
            ..self.clone()
        };
        (compact, map)
    }
}
