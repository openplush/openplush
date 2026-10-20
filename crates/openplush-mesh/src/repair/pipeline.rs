//! `into_manifold`: the repair steps in order, and the report they fill.

use openplush_geom::epsilon::ModelTolerances;

use super::nonmanifold_edge::pair_edges;
use super::orientation::Conflict;
use super::{bowtie, cap, components, degenerate, fill, holes, orientation, weld};
use crate::{
    EdgeId, FaceId, HalfEdgeId, HoleReport, ManifoldMesh, MeshError, PairedEdge, RepairPlan,
    RepairReport, TriangleSoup, VertexId,
};

/// What the steps found that the build and the report still need.
struct Pending {
    pairings: Vec<PairedEdge>,
    sheets: Vec<[FaceId; 2]>,
    /// Edges cut loose by the pairings, as the vertices the soup now gives them.
    seams: Vec<[VertexId; 2]>,
    /// Every edge the pairings or the orientation cuts split.
    splits: Vec<SplitEdge>,
}

/// An edge repair splits: one face per copy, with the edge's ends as that face has them in
/// the soup now, and whether the split is an orientation cut (every copy a boundary and a seam).
struct SplitEdge {
    copies: Vec<(FaceId, [VertexId; 2])>,
    cut: bool,
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
    let mut conflicts = Vec::new();
    if plan.fix_orientation {
        let (oriented, done) = orientation::orient(&stage.soup, &tol)?;
        stage.soup = oriented;
        report.flipped_faces = stage.input_faces(&done.flipped);
        conflicts = done.conflicts;
    }
    let (seams, splits) = split_vertices(&mut stage, plan, &pairings, &conflicts, &mut report)?;
    // The pairings name the soup before the split, so they are stale after it.
    let pairings = pair(plan.split_non_manifold_edges, &stage.soup, &sheets)?;
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
        splits,
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
        .filter_map(|p| after_split(p.vertices, p.unpaired?, before, after))
        .collect()
}

/// Splits the vertices between fans, if the plan says so, and returns the unpaired edges and
/// every split edge as the soup now numbers them.
fn split_vertices(
    stage: &mut Stage,
    plan: &RepairPlan,
    pairings: &[PairedEdge],
    conflicts: &[Conflict],
    report: &mut RepairReport,
) -> Result<(Vec<[VertexId; 2]>, Vec<SplitEdge>), MeshError> {
    let mut seams = unpaired_edges(pairings, &stage.soup, &stage.soup);
    let mut splits = split_edges(pairings, conflicts);
    if plan.split_bowties {
        let (split, copies) = bowtie::split(&stage.soup, pairings, conflicts)?;
        seams = unpaired_edges(pairings, &stage.soup, &split);
        for s in &mut splits {
            s.copies = s
                .copies
                .iter()
                .filter_map(|&(face, edge)| {
                    Some((face, after_split(edge, face, &stage.soup, &split)?))
                })
                .collect();
        }
        stage.soup = split;
        report.vertex_copies = copies.copies;
    }
    Ok((seams, splits))
}

/// Every edge the pairings or the conflicts split, one face per copy: the first face of each
/// pair and the unpaired face, or a conflict's two faces.
fn split_edges(pairings: &[PairedEdge], conflicts: &[Conflict]) -> Vec<SplitEdge> {
    let paired = pairings.iter().map(|p| SplitEdge {
        copies: p
            .pairs
            .iter()
            .map(|pair| pair[0])
            .chain(p.unpaired)
            .map(|face| (face, p.vertices))
            .collect(),
        cut: false,
    });
    let cut = conflicts.iter().map(|c| SplitEdge {
        copies: c.faces.iter().map(|&face| (face, c.vertices)).collect(),
        cut: true,
    });
    paired.chain(cut).collect()
}

/// `edge` of `face`, as the vertices that face has in `after` (the soup `before` split, same
/// faces).
fn after_split(
    edge: [VertexId; 2],
    face: FaceId,
    before: &TriangleSoup,
    after: &TriangleSoup,
) -> Option<[VertexId; 2]> {
    let was = before.triangle(face)?;
    let now = after.triangle(face)?;
    let at = |end: VertexId| was.iter().position(|&v| v == end).map(|i| now[i]);
    Some([at(edge[0])?, at(edge[1])?])
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
        splits,
    } = pending;
    let (compact, map) = stage.soup.compacted();
    // INVARIANT: every vertex the pairings, copies and seams name is used by a face.
    let renumber = |v: VertexId| VertexId::new(map[v.index() as usize]);
    let edge = |[a, b]: [VertexId; 2]| [renumber(a), renumber(b)];
    let pairings: Vec<PairedEdge> = pairings
        .iter()
        .map(|p| PairedEdge {
            vertices: edge(p.vertices),
            ..p.clone()
        })
        .collect();
    let splits: Vec<SplitEdge> = splits
        .into_iter()
        .map(|s| SplitEdge {
            copies: s.copies.iter().map(|&(f, e)| (f, edge(e))).collect(),
            cut: s.cut,
        })
        .collect();
    // A cut whose faces still share both ends after the vertex split is cut by the build.
    let conflicts: Vec<Conflict> = splits
        .iter()
        .filter(|s| s.cut && s.copies[0].1 == s.copies[1].1)
        .map(|s| Conflict {
            vertices: s.copies[0].1,
            faces: [s.copies[0].0, s.copies[1].0],
        })
        .collect();
    let mut mesh =
        ManifoldMesh::from_paired_triangles(&compact, tol, &pairings, &sheets, &conflicts)?;
    for copy in &mut report.vertex_copies {
        copy.of = renumber(copy.of);
        copy.copy = renumber(copy.copy);
    }
    let mut seams: Vec<[VertexId; 2]> = seams.into_iter().map(edge).collect();
    resolve_splits(&mesh, &splits, &mut report, &mut seams);
    mesh.set_edge_copies(report.edge_copies.clone());
    seams.sort();
    seams.dedup();
    report.mandatory_seams = seams;
    let mut holes = Vec::new();
    for hole in holes::classify(&mesh, plan)? {
        let added_faces = if hole.accidental {
            fill::fill(&mut mesh, hole.loop_ref, &hole.half_edges, tol)?
        } else {
            Vec::new()
        };
        holes.push(HoleReport {
            loop_ref: hole.loop_ref,
            filled: hole.accidental,
            added_faces,
        });
    }
    report.holes = holes;
    Ok((mesh, report))
}

/// Fills the report's edge copies and orientation conflicts from the split edges, as the
/// mesh's IDs, and adds every copy of a cut to `seams`.
fn resolve_splits(
    mesh: &ManifoldMesh,
    splits: &[SplitEdge],
    report: &mut RepairReport,
    seams: &mut Vec<[VertexId; 2]>,
) {
    let mut groups = Vec::new();
    for s in splits {
        let mut copies: Vec<EdgeId> = s
            .copies
            .iter()
            .filter_map(|&(face, ends)| edge_of(mesh, face, ends))
            .collect();
        if s.cut {
            report.orientation_conflicts.push(s.copies[0].1);
            seams.extend(copies.iter().map(|&e| sorted_ends(mesh, e)));
        }
        copies.sort();
        groups.push(copies);
    }
    groups.sort();
    report.edge_copies = groups;
    report.orientation_conflicts.sort();
}

/// The edge of `face` between `ends`, if it has one.
fn edge_of(mesh: &ManifoldMesh, face: FaceId, ends: [VertexId; 2]) -> Option<EdgeId> {
    let h = mesh.face_half_edge(face);
    [h, mesh.next(h), mesh.prev(h)]
        .into_iter()
        .find(|&h| {
            let (a, b) = (mesh.origin(h), mesh.dest(h));
            [a, b] == ends || [b, a] == ends
        })
        .map(HalfEdgeId::edge)
}

/// An edge's ends, lower first.
fn sorted_ends(mesh: &ManifoldMesh, e: EdgeId) -> [VertexId; 2] {
    let h = e.half_edge();
    let mut ends = [mesh.origin(h), mesh.dest(h)];
    ends.sort();
    ends
}
