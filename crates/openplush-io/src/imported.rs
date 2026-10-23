use openplush_mesh::TriangleSoup;

/// A read file: its soup, and what was in it that the soup does not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    /// The drawn triangles.
    pub soup: TriangleSoup,
    /// Everything not applied, in walk order, animations last.
    pub ignored: Vec<Ignored>,
}

/// Something in the file that the soup does not carry. glTF parts are named by the file's
/// own indices, FBX parts by the names of the file's objects.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    /// An FBX deformer (a skin, a blend shape) on a drawn mesh, read in the rest pose.
    Deformer {
        /// The model drawing the mesh.
        model: String,
        /// The deformer.
        deformer: String,
    },
    /// An FBX animation stack, not played.
    AnimationStack {
        /// The stack.
        stack: String,
    },
    /// FBX geometry on a drawn model that is not a polygon mesh (a curve, a NURBS
    /// surface), not drawn.
    Geometry {
        /// The model.
        model: String,
        /// The geometry.
        geometry: String,
    },
}
