//! Stable keys for edges and faces, from source provenance. A key written against one repair
//! of a model resolves against another, where `EdgeId`s and `FaceId`s move.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use openplush_mesh::{EdgeId, FaceId, ManifoldMesh, SourceCorner, VertexId};

use crate::ProjectError;

/// An edge by its two ends' source vertices, in `SourceCorner` order; written `0.9:0.12`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeKey([SourceCorner; 2]);

impl EdgeKey {
    /// The key of the edge between `a` and `b`, in either order.
    #[must_use]
    pub fn new(a: SourceCorner, b: SourceCorner) -> Self {
        Self(if b < a { [b, a] } else { [a, b] })
    }

    /// The two source vertices, lower first.
    #[must_use]
    pub fn ends(self) -> [SourceCorner; 2] {
        self.0
    }
}

/// A face by its three corners' source vertices, in `SourceCorner` order, so an orientation
/// flip does not change it; written `0.7:0.40:1.2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FaceKey([SourceCorner; 3]);

impl FaceKey {
    /// The key of the face with these corners, in any order.
    #[must_use]
    pub fn new(mut corners: [SourceCorner; 3]) -> Self {
        corners.sort();
        Self(corners)
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

fn write_joined(f: &mut fmt::Formatter<'_>, corners: &[SourceCorner]) -> fmt::Result {
    for (i, c) in corners.iter().enumerate() {
        let sep = if i == 0 { "" } else { ":" };
        write!(f, "{sep}{}.{}", c.primitive, c.vertex)?;
    }
    Ok(())
}

fn parse_joined<const N: usize>(text: &str) -> Result<[SourceCorner; N], ProjectError> {
    let bad = || ProjectError::BadKey { text: text.into() };
    let corners: Vec<SourceCorner> = text
        .split(':')
        .map(|part| {
            let (primitive, vertex) = part.split_once('.').ok_or_else(bad)?;
            Ok(SourceCorner {
                primitive: primitive.parse().map_err(|_| bad())?,
                vertex: vertex.parse().map_err(|_| bad())?,
            })
        })
        .collect::<Result<_, ProjectError>>()?;
    corners.try_into().map_err(|_| bad())
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
    /// Per vertex, the lowest source vertex among its corners.
    canonical: Vec<Option<SourceCorner>>,
    /// Per source vertex, every vertex with a corner from it, ascending.
    holders: BTreeMap<SourceCorner, Vec<VertexId>>,
    /// Per face key, every face with exactly those corners, ascending.
    faces: BTreeMap<FaceKey, Vec<FaceId>>,
}

impl<'m> Provenance<'m> {
    /// Indexes `mesh`'s corners by source vertex.
    #[must_use]
    pub fn new(mesh: &'m ManifoldMesh) -> Self {
        let mut canonical = Vec::with_capacity(mesh.vertex_count());
        let mut holders: BTreeMap<SourceCorner, Vec<VertexId>> = BTreeMap::new();
        for v in ids(mesh.vertex_count()).map(VertexId::new) {
            let mut lowest = None;
            for c in mesh.outgoing(v).filter_map(|h| mesh.corner_source(h)) {
                lowest = Some(lowest.map_or(c, |l: SourceCorner| l.min(c)));
                let at = holders.entry(c).or_default();
                if at.last() != Some(&v) {
                    at.push(v);
                }
            }
            canonical.push(lowest);
        }
        let mut faces: BTreeMap<FaceKey, Vec<FaceId>> = BTreeMap::new();
        for f in ids(mesh.face_count()).map(FaceId::new) {
            if let Some(key) = face_key(mesh, f) {
                faces.entry(key).or_default().push(f);
            }
        }
        Self {
            mesh,
            canonical,
            holders,
            faces,
        }
    }

    /// `edge`'s key; `None` if an end has no source vertex.
    #[must_use]
    pub fn edge_key(&self, edge: EdgeId) -> Option<EdgeKey> {
        let h = edge.half_edge();
        // INVARIANT: `canonical` has one entry per vertex of `self.mesh`, which `h` is from.
        let end = |v: VertexId| self.canonical[ix(v.index())];
        Some(EdgeKey::new(
            end(self.mesh.origin(h))?,
            end(self.mesh.dest(h))?,
        ))
    }

    /// Every edge between a vertex holding one of `key`'s source vertices and a vertex holding
    /// the other, ascending; empty when the mesh has none.
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

    /// `face`'s key; `None` if a corner has no source vertex.
    #[must_use]
    pub fn face_key(&self, face: FaceId) -> Option<FaceKey> {
        face_key(self.mesh, face)
    }

    /// Every face with exactly `key`'s corners, ascending: one, or both layers of a
    /// double-sided sheet; empty when the mesh has none.
    #[must_use]
    pub fn resolve_face(&self, key: FaceKey) -> &[FaceId] {
        self.faces.get(&key).map_or(&[], Vec::as_slice)
    }
}

fn face_key(mesh: &ManifoldMesh, face: FaceId) -> Option<FaceKey> {
    let h = mesh.face_half_edge(face);
    let next = mesh.next(h);
    Some(FaceKey::new([
        mesh.corner_source(h)?,
        mesh.corner_source(next)?,
        mesh.corner_source(mesh.next(next))?,
    ]))
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
