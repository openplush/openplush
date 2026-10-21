use openplush_mesh::TriangleSoup;

/// A read file: its soup, and what was in it that the soup does not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    /// The drawn triangles.
    pub soup: TriangleSoup,
    /// Everything not applied, in walk order, animations last.
    pub ignored: Vec<Ignored>,
}

/// Something in the file that the soup does not carry. Indices are the file's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ignored {
    /// A skinned node, read in its bind pose.
    Skin {
        /// The node.
        node: u32,
    },
    /// A mesh's morph targets, read as the base mesh.
    MorphTargets {
        /// The node drawing it.
        node: u32,
        /// The mesh.
        mesh: u32,
    },
    /// An animation, not played.
    Animation {
        /// The animation.
        animation: u32,
    },
    /// A point or line primitive, not drawn.
    Primitive {
        /// The node drawing it.
        node: u32,
        /// Its mesh.
        mesh: u32,
        /// Its index in the mesh.
        primitive: u32,
    },
}
