/// Union-find over `0..n`. Each set's root is its smallest member, so the result
/// does not depend on the order of unions.
pub(super) struct DisjointSet {
    parent: Vec<u32>,
}

impl DisjointSet {
    /// `n` singletons. `n` fits in `u32`: it counts soup vertices or faces.
    pub(super) fn new(n: usize) -> Self {
        // Lossless: soup vertex and face counts fit in u32 (`TriangleSoup::new`).
        Self {
            parent: (0..n as u32).collect(),
        }
    }

    // INVARIANT: every `x` passed in is below the `n` given to `new`.
    pub(super) fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let up = self.parent[self.parent[x as usize] as usize];
            self.parent[x as usize] = up;
            x = up;
        }
        x
    }

    pub(super) fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        let (low, high) = if ra < rb { (ra, rb) } else { (rb, ra) };
        self.parent[high as usize] = low;
    }
}
