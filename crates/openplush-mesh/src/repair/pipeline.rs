//! `into_manifold`: the repair steps in order, and the report they fill.

use openplush_geom::epsilon::ModelTolerances;

use super::nonmanifold_edge::pair_edges;
use super::{bowtie, cap, components, degenerate, orientation, weld};
use crate::{
    FaceId, HalfEdgeId, HoleReport, LoopRef, ManifoldMesh, MeshError, PairedEdge, RepairPlan,
    RepairReport, TriangleSoup, VertexId,
};

/// What the steps found that the build and the report still need.
struct Pending {
    pairings: Vec<PairedEdge>,
    sheets: Vec<[FaceId; 2]>,
    /// Edges cut loose by the pairings, as the vertices the soup now gives them.
    seams: Vec<[VertexId; 2]>,
}

/// The soup as the steps have left it, and the input face each of its faces came from.
struct Stage {
    soup: TriangleSoup,
    origin: Vec<FaceId>,
}

impl Stage {
    /// `faces` of this stage's soup, as the input faces they came from, sorted.
    fn input_faces(&self, faces: &[FaceId]) -> Vec<FaceId> {
        let mut out: Vec<FaceId> = faces
            .iter()
            // INVARIANT: `origin` has one entry per face of the soup, and `faces` are its faces.
            .map(|f| self.origin[f.index() as usize])
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn input_pairing(&self, p: &PairedEdge) -> PairedEdge {
        let one = |f: FaceId| self.input_faces(&[f])[0];
        PairedEdge {
            vertices: p.vertices,
            // Two halves of one stitched face are not a pair the input had.
            pairs: p
                .pairs
                .iter()
                .map(|&[a, b]| [one(a), one(b)])
                .filter(|[a, b]| a != b)
                .collect(),
            unpaired: p.unpaired.map(one),
        }
    }
}

/// Repairs `soup` as `plan` says and builds the mesh, listing every change in the report.
///
/// This is the only way to obtain a [`ManifoldMesh`]. Steps the plan turns off are
/// skipped, and whatever they would have fixed reaches the build, which refuses it.
/// A loop in `plan.fill_holes` is refused: the mesh is built only to find the loop.
pub fn into_manifold(
    soup: &TriangleSoup,
    plan: &RepairPlan,
) -> Result<(ManifoldMesh, RepairReport), MeshError> {
    let diagonal = soup.bbox_diagonal().ok_or(MeshError::NoTriangles)?;
    let tol = ModelTolerances::for_diagonal(diagonal).map_err(|_| MeshError::UnusableExtent)?;
    let mut report = RepairReport::default();
    // Lossless: the face count fits in u32 (`TriangleSoup::new`).
    let origin = (0..soup.triangle_count() as u32).map(FaceId::new).collect();
    let mut stage = Stage {
        soup: soup.clone(),
        origin,
    };
    if plan.weld {
        let (welded, groups) = weld::merge(&stage.soup, &tol);
        stage.soup = welded;
        report.welded = groups;
    }
    if plan.drop_degenerates {
        drop_degenerates(&mut stage, &tol, &mut report);
    }
    let sheets = if plan.drop_degenerates {
        degenerate::classify(stage.soup.triangle_indices()).sheets
    } else {
        Vec::new()
    };
    let pairings = pair(plan.split_non_manifold_edges, &stage.soup, &sheets)?;
    report.paired_edges = pairings
        .iter()
        .map(|p| stage.input_pairing(p))
        .filter(|p| !p.pairs.is_empty() || p.unpaired.is_some())
        .collect();
    let mut seams = unpaired_edges(&pairings, &stage.soup, &stage.soup);
    if plan.split_bowties {
        let (split, copies) = bowtie::split(&stage.soup, &pairings)?;
        seams = unpaired_edges(&pairings, &stage.soup, &split);
        stage.soup = split;
        report.vertex_copies = copies.copies;
    }
    // The pairings name the soup before the split, so they are stale after it.
    let pairings = pair(plan.split_non_manifold_edges, &stage.soup, &sheets)?;
    if plan.fix_orientation {
        let (oriented, done) = orientation::orient(&stage.soup, &tol)?;
        stage.soup = oriented;
        // A conflict edge is left in the soup, so the build refuses it.
        report.flipped_faces = stage.input_faces(&done.flipped);
    }
    report.components = components::split(&stage.soup)
        .into_iter()
        .map(|mut c| {
            c.faces = stage.input_faces(&c.faces);
            c
        })
        .collect();
    let pending = Pending {
        pairings,
        sheets,
        seams,
    };
    build(&stage, &tol, plan, pending, report)
}

fn pair(
    on: bool,
    soup: &TriangleSoup,
    sheets: &[[FaceId; 2]],
) -> Result<Vec<PairedEdge>, MeshError> {
    if on {
        pair_edges(soup, sheets)
    } else {
        Ok(Vec::new())
    }
}

/// The edge each unpaired face is left alone on, as the vertices that face has in `after`
/// (the soup `before` split, same faces): once cut loose, that edge is a boundary.
fn unpaired_edges(
    pairings: &[PairedEdge],
    before: &TriangleSoup,
    after: &TriangleSoup,
) -> Vec<[VertexId; 2]> {
    pairings
        .iter()
        .filter_map(|p| {
            let face = p.unpaired?;
            let was = before.triangle(face)?;
            let now = after.triangle(face)?;
            let at = |end: VertexId| was.iter().position(|&v| v == end).map(|i| now[i]);
            Some([at(p.vertices[0])?, at(p.vertices[1])?])
        })
        .collect()
}

/// Drops repeated-index faces and copies, then stitches caps, keeping `stage.origin` and
/// the report in the input's face IDs.
fn drop_degenerates(stage: &mut Stage, tol: &ModelTolerances, report: &mut RepairReport) {
    let (kept, removed) = degenerate::remove(&stage.soup);
    report.repeated_index_faces = removed.repeated_index;
    report.duplicate_faces = removed.duplicates;
    report.sheets = removed.sheets;
    stage.soup = kept;
    stage.origin = removed.origin;
    let (stitched, st) = cap::stitch(&stage.soup, tol);
    report.stitched_caps = stage.input_faces(&st.caps);
    report.retriangulated_faces = stage.input_faces(&st.retriangulated);
    // INVARIANT: `st.origin` has one entry per stitched face, naming a face of the soup before.
    stage.origin = st
        .origin
        .iter()
        .map(|f| stage.origin[f.index() as usize])
        .collect();
    stage.soup = stitched;
}

/// Compacts the soup, builds the mesh, and fills the rest of the report in the mesh's IDs.
fn build(
    stage: &Stage,
    tol: &ModelTolerances,
    plan: &RepairPlan,
    pending: Pending,
    mut report: RepairReport,
) -> Result<(ManifoldMesh, RepairReport), MeshError> {
    let Pending {
        pairings,
        sheets,
        seams,
    } = pending;
    let (compact, map) = stage.soup.compacted();
    // INVARIANT: every vertex the pairings, copies and seams name is used by a face.
    let renumber = |v: VertexId| VertexId::new(map[v.index() as usize].unwrap_or(v.index()));
    let edge = |[a, b]: [VertexId; 2]| [renumber(a), renumber(b)];
    let pairings: Vec<PairedEdge> = pairings
        .iter()
        .map(|p| PairedEdge {
            vertices: edge(p.vertices),
            ..p.clone()
        })
        .collect();
    let mesh = ManifoldMesh::from_paired_triangles(&compact, tol, &pairings, &sheets)?;
    for copy in &mut report.vertex_copies {
        copy.of = renumber(copy.of);
        copy.copy = renumber(copy.copy);
    }
    let mut seams: Vec<[VertexId; 2]> = seams.into_iter().map(edge).collect();
    seams.sort();
    seams.dedup();
    report.mandatory_seams = seams;
    report.holes = holes(&mesh)
        .into_iter()
        .map(|loop_ref| HoleReport {
            loop_ref,
            filled: false,
            added_faces: Vec::new(),
        })
        .collect();
    let has = |l: &LoopRef| report.holes.iter().any(|h| h.loop_ref == *l);
    if let Some(&loop_ref) = plan.fill_holes.iter().find(|l| !has(l)) {
        return Err(MeshError::UnknownHoleLoop { loop_ref });
    }
    if let Some(&loop_ref) = plan.fill_holes.iter().next() {
        return Err(MeshError::HoleFillUnavailable { loop_ref });
    }
    Ok((mesh, report))
}

/// Each boundary loop of the mesh, named by the lowest source corner among its vertices.
fn holes(mesh: &ManifoldMesh) -> Vec<LoopRef> {
    mesh.boundary_loops()
        .iter()
        .filter_map(|boundary| {
            boundary
                .iter()
                // The twin of a boundary half-edge has a face, whose corner at its
                // origin is the boundary half-edge's destination.
                .filter_map(|&h: &HalfEdgeId| mesh.corner_source(h.twin()))
                .min()
                .map(LoopRef::new)
        })
        .collect()
}
