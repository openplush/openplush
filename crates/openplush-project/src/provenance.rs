//! Stable keys for edges and faces, from source provenance. A key written against one repair
//! of a model resolves against another, where `EdgeId`s and `FaceId`s move.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use openplush_mesh::{
    EdgeId, FaceId, HalfEdgeId, HoleReport, LoopRef, ManifoldMesh, SourceCorner, VertexId,
};

use crate::ProjectError;

/// One vertex in a key. Every source vertex sorts before every patch vertex, and patch
/// vertices sort by loop, then number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VertexKey {
    /// A source vertex; written `0.12`.
    Source(SourceCorner),
    /// Vertex `n` of the patch filling `rim`, numbered in `VertexId` order; written `0.12+3`.
    Patch {
        /// The filled loop.
        rim: LoopRef,
        /// The vertex's place among the patch's vertices.
        n: u32,
    },
}

impl From<SourceCorner> for VertexKey {
    fn from(corner: SourceCorner) -> Self {
        Self::Source(corner)
    }
}

/// An edge by its two ends, in `VertexKey` order; written `0.9:0.12`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeKey([VertexKey; 2]);

impl EdgeKey {
    /// The key of the edge between `a` and `b`, in either order.
    #[must_use]
    pub fn new(a: impl Into<VertexKey>, b: impl Into<VertexKey>) -> Self {
        let (a, b) = (a.into(), b.into());
        Self(if b < a { [b, a] } else { [a, b] })
    }

    /// The two ends, lower first.
    #[must_use]
    pub fn ends(self) -> [VertexKey; 2] {
        self.0
    }
}

/// A face by its three corners, in `VertexKey` order, so an orientation flip does not change
/// it; written `0.7:0.40:1.2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FaceKey([VertexKey; 3]);

impl FaceKey {
    /// The key of the face with these corners, in any order.
    #[must_use]
    pub fn new<C: Into<VertexKey>>(corners: [C; 3]) -> Self {
        let mut corners = corners.map(Into::into);
        corners.sort();
        Self(corners)
    }
}

impl fmt::Display for VertexKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source =
            |f: &mut fmt::Formatter<'_>, c: SourceCorner| write!(f, "{}.{}", c.primitive, c.vertex);
        match *self {
            Self::Source(c) => source(f, c),
            Self::Patch { rim, n } => {
                source(f, rim.corner())?;
                write!(f, "+{n}")
            }
        }
    }
}

impl fmt::Display for EdgeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_joined(f, &self.0)
    }
}

impl fmt::Display for FaceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_joined(f, &self.0)
    }
}

impl FromStr for EdgeKey {
    type Err = ProjectError;

    /// Only the text [`Display`](fmt::Display) writes parses.
    fn from_str(text: &str) -> Result<Self, ProjectError> {
        let [a, b] = parse_joined(text)?;
        canonical(text, Self::new(a, b))
    }
}

impl FromStr for FaceKey {
    type Err = ProjectError;

    /// Only the text [`Display`](fmt::Display) writes parses.
    fn from_str(text: &str) -> Result<Self, ProjectError> {
        canonical(text, Self::new(parse_joined(text)?))
    }
}

fn write_joined(f: &mut fmt::Formatter<'_>, ends: &[VertexKey]) -> fmt::Result {
    for (i, end) in ends.iter().enumerate() {
        let sep = if i == 0 { "" } else { ":" };
        write!(f, "{sep}{end}")?;
    }
    Ok(())
}

fn parse_joined<const N: usize>(text: &str) -> Result<[VertexKey; N], ProjectError> {
    let bad = || ProjectError::BadKey { text: text.into() };
    let ends: Vec<VertexKey> = text
        .split(':')
        .map(|part| parse_vertex(part).ok_or_else(bad))
        .collect::<Result<_, ProjectError>>()?;
    ends.try_into().map_err(|_| bad())
}

/// `0.12` or `0.12+3`, not yet checked for canonical form.
fn parse_vertex(part: &str) -> Option<VertexKey> {
    let (source, n) = match part.split_once('+') {
        Some((source, n)) => (source, Some(n.parse().ok()?)),
        None => (part, None),
    };
    let (primitive, vertex) = source.split_once('.')?;
    let corner = SourceCorner {
        primitive: primitive.parse().ok()?,
        vertex: vertex.parse().ok()?,
    };
    Some(match n {
        Some(n) => VertexKey::Patch {
            rim: LoopRef::new(corner),
            n,
        },
        None => VertexKey::Source(corner),
    })
}

/// `key`, if `text` is exactly how it prints: sorted, no sign, no leading zero, no padding.
fn canonical<K: fmt::Display>(text: &str, key: K) -> Result<K, ProjectError> {
    if key.to_string() == text {
        Ok(key)
    } else {
        Err(ProjectError::BadKey { text: text.into() })
    }
}

/// Writes and resolves keys against one repaired mesh.
#[derive(Debug)]
pub struct Provenance<'m> {
    mesh: &'m ManifoldMesh,
    /// Per vertex, its key: the lowest source vertex among its corners, else its patch place.
    names: Vec<Option<VertexKey>>,
    /// Per vertex key, every vertex it names or with a corner from that source, ascending.
    holders: BTreeMap<VertexKey, Vec<VertexId>>,
    /// Per face key, every face with exactly those corners, ascending.
    faces: BTreeMap<FaceKey, Vec<FaceId>>,
}

impl<'m> Provenance<'m> {
    /// Indexes `mesh`'s corners by source vertex, and the vertices of each filled loop's
    /// patch by their place in it. `holes` is the report of the repair that made `mesh`.
    #[must_use]
    pub fn new(mesh: &'m ManifoldMesh, holes: &[HoleReport]) -> Self {
        let (mut names, mut holders) = source_names(mesh);
        for hole in holes.iter().filter(|h| h.filled()) {
            for (v, n) in patch_vertices(mesh, hole, &names).into_iter().zip(0u32..) {
                let key = VertexKey::Patch {
                    rim: hole.loop_ref(),
                    n,
                };
                names[ix(v.index())] = Some(key);
                holders.entry(key).or_default().push(v);
            }
        }
        let mut faces: BTreeMap<FaceKey, Vec<FaceId>> = BTreeMap::new();
        for f in ids(mesh.face_count()).map(FaceId::new) {
            if let Some(key) = face_key(mesh, &names, f) {
                faces.entry(key).or_default().push(f);
            }
        }
        Self {
            mesh,
            names,
            holders,
            faces,
        }
    }

    /// `edge`'s key; `None` if an end has neither a source vertex nor a patch place.
    #[must_use]
    pub fn edge_key(&self, edge: EdgeId) -> Option<EdgeKey> {
        let h = edge.half_edge();
        // INVARIANT: `names` has one entry per vertex of `self.mesh`, which `h` is from.
        let end = |v: VertexId| self.names[ix(v.index())];
        Some(EdgeKey::new(
            end(self.mesh.origin(h))?,
            end(self.mesh.dest(h))?,
        ))
    }

    /// Every edge between a vertex held by one of `key`'s ends and a vertex held by the
    /// other, ascending; empty when the mesh has none.
    #[must_use]
    pub fn resolve_edge(&self, key: EdgeKey) -> Vec<EdgeId> {
        let [a, b] = key.0;
        let (Some(from), Some(to)) = (self.holders.get(&a), self.holders.get(&b)) else {
            return Vec::new();
        };
        let mut edges: Vec<EdgeId> = from
            .iter()
            .flat_map(|&u| self.mesh.outgoing(u))
            .filter(|&h| to.binary_search(&self.mesh.dest(h)).is_ok())
            .map(|h| h.edge())
            .collect();
        edges.sort();
        edges.dedup();
        edges
    }

    /// `face`'s key; `None` if a corner has no key.
    #[must_use]
    pub fn face_key(&self, face: FaceId) -> Option<FaceKey> {
        face_key(self.mesh, &self.names, face)
    }

    /// Every face with exactly `key`'s corners, ascending: one, or both layers of a
    /// double-sided sheet; empty when the mesh has none.
    #[must_use]
    pub fn resolve_face(&self, key: FaceKey) -> &[FaceId] {
        self.faces.get(&key).map_or(&[], Vec::as_slice)
    }
}

type Names = (Vec<Option<VertexKey>>, BTreeMap<VertexKey, Vec<VertexId>>);

/// Per vertex, the lowest source vertex among its corners; per source vertex, its holders.
fn source_names(mesh: &ManifoldMesh) -> Names {
    let mut names = Vec::with_capacity(mesh.vertex_count());
    let mut holders: BTreeMap<VertexKey, Vec<VertexId>> = BTreeMap::new();
    for v in ids(mesh.vertex_count()).map(VertexId::new) {
        let mut lowest = None;
        for c in mesh.outgoing(v).filter_map(|h| mesh.corner_source(h)) {
            lowest = Some(lowest.map_or(c, |l: SourceCorner| l.min(c)));
            let at = holders.entry(VertexKey::Source(c)).or_default();
            if at.last() != Some(&v) {
                at.push(v);
            }
        }
        names.push(lowest.map(VertexKey::Source));
    }
    (names, holders)
}

/// The vertices of `hole`'s added faces that have no source corner, which are the patch's
/// own vertices off the rim, in `VertexId` order: the order the fill appended them in.
fn patch_vertices(
    mesh: &ManifoldMesh,
    hole: &HoleReport,
    names: &[Option<VertexKey>],
) -> Vec<VertexId> {
    let mut patch: Vec<VertexId> = hole
        .added_faces()
        .iter()
        .flat_map(|&f| corners(mesh, f))
        .map(|h| mesh.origin(h))
        // INVARIANT: `names` has one entry per vertex of `mesh`, which the report is from.
        .filter(|v| names[ix(v.index())].is_none())
        .collect();
    patch.sort();
    patch.dedup();
    patch
}

/// A face's three half-edges, from its first.
fn corners(mesh: &ManifoldMesh, face: FaceId) -> [HalfEdgeId; 3] {
    let h = mesh.face_half_edge(face);
    let next = mesh.next(h);
    [h, next, mesh.next(next)]
}

/// A corner's key is its source vertex, or for a patch corner, which has none, its vertex's.
fn face_key(mesh: &ManifoldMesh, names: &[Option<VertexKey>], face: FaceId) -> Option<FaceKey> {
    let key = |h| {
        mesh.corner_source(h)
            .map(VertexKey::Source)
            // INVARIANT: `names` has one entry per vertex of `mesh`.
            .or(names[ix(mesh.origin(h).index())])
    };
    let [a, b, c] = corners(mesh, face);
    Some(FaceKey::new([key(a)?, key(b)?, key(c)?]))
}

/// The IDs `0..count`.
fn ids(count: usize) -> impl Iterator<Item = u32> {
    // Lossless: a mesh's element counts fit in u32 (checked at construction).
    (0..count).map(|i| i as u32)
}

#[inline]
fn ix(i: u32) -> usize {
    // Lossless: usize is at least 32 bits on every supported target.
    i as usize
}
