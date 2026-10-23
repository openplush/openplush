//! The chart stage: seams in, charts and their sewing map out, with the first violated invariant
//! as the only other outcome. Meshes are written as OBJ and repaired, as a shell would.

use std::fmt::Write as _;

use openplush_io::obj;
use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    ChartFailure, ChartId, EdgeId, EngineError, ManifoldMesh, SeamChainId, SeamKind, SeamSet,
    SeamsError, VertexId, chart,
};

type Setup = Box<dyn std::error::Error>;

/// Repairs the OBJ text `obj` with the default plan.
fn mesh_of(obj: &str) -> Result<(ManifoldMesh, openplush_mesh::RepairReport), Setup> {
    let soup = obj::read(obj.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?)
}

/// A `cols` by `rows` grid of quads in the plane, each split along its rising diagonal.
fn sheet(cols: u32, rows: u32) -> String {
    let mut s = String::new();
    for r in 0..=rows {
        for c in 0..=cols {
            let _ = writeln!(s, "v {} {} 0", 10 * c, 10 * r);
        }
    }
    let id = |c: u32, r: u32| r * (cols + 1) + c + 1;
    for r in 0..rows {
        for c in 0..cols {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            let _ = writeln!(s, "f {a} {b} {d}\nf {a} {d} {e}");
        }
    }
    s
}

/// A tube of `cols` quads round and `rows` along, open at both ends.
fn tube(cols: u32, rows: u32) -> String {
    let mut s = String::new();
    for r in 0..=rows {
        for c in 0..cols {
            let a = std::f64::consts::TAU * f64::from(c) / f64::from(cols);
            let _ = writeln!(
                s,
                "v {} {} {}",
                10.0 * libm::cos(a),
                10.0 * libm::sin(a),
                10 * r
            );
        }
    }
    let id = |c: u32, r: u32| r * cols + c % cols + 1;
    for r in 0..rows {
        for c in 0..cols {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            let _ = writeln!(s, "f {a} {b} {d}\nf {a} {d} {e}");
        }
    }
    s
}

const OCTAHEDRON: &str = "v 10 0 0\nv 0 10 0\nv -10 0 0\nv 0 -10 0\nv 0 0 10\nv 0 0 -10\n\
f 1 2 5\nf 2 1 6\nf 2 3 5\nf 3 2 6\nf 3 4 5\nf 4 3 6\nf 4 1 5\nf 1 4 6\n";

/// The vertex of `mesh` at `(x, y, z)`, found by position.
fn vertex_at(mesh: &ManifoldMesh, x: f64, y: f64, z: f64) -> Result<VertexId, Setup> {
    (0..u32::try_from(mesh.vertex_count())?)
        .map(VertexId::new)
        .find(|&v| {
            let p = mesh.position(v);
            (p.x, p.y, p.z) == (x, y, z)
        })
        .ok_or_else(|| format!("no vertex at ({x}, {y}, {z})").into())
}

/// The edge of `mesh` between two vertices.
fn edge_between(mesh: &ManifoldMesh, a: VertexId, b: VertexId) -> Result<EdgeId, Setup> {
    mesh.outgoing(a)
        .find(|&h| mesh.dest(h) == b)
        .map(|h| h.edge())
        .ok_or_else(|| "no such edge".into())
}

/// Seams of `kind` along a walk through the grid points `(column, row)` of a sheet.
fn sheet_seam(
    mesh: &ManifoldMesh,
    seams: &mut SeamSet,
    kind: SeamKind,
    points: &[(u32, u32)],
) -> Result<(), Setup> {
    for pair in points.windows(2) {
        let at = |(c, r): (u32, u32)| vertex_at(mesh, f64::from(10 * c), f64::from(10 * r), 0.0);
        seams.insert(edge_between(mesh, at(pair[0])?, at(pair[1])?)?, kind)?;
    }
    Ok(())
}

#[test]
fn a_seam_across_a_sheet_gives_two_charts_and_one_sewn_seam() -> Result<(), Setup> {
    let (mesh, _) = mesh_of(&sheet(4, 2))?;
    let mut seams = SeamSet::new();
    sheet_seam(
        &mesh,
        &mut seams,
        SeamKind::Cut,
        &[(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
    )?;
    let staged = chart(&mesh, &seams)?;
    let (charts, map) = staged.value;
    assert_eq!(staged.warnings.iter().count(), 0, "no warnings yet");
    assert_eq!(charts.len(), 2);
    assert_eq!(charts[0].chart(), ChartId::new(0));
    assert_eq!(
        charts.iter().map(|c| c.faces().len()).sum::<usize>(),
        mesh.face_count()
    );
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.sides().len(), 2);
    Ok(())
}

#[test]
fn the_same_mesh_and_seams_give_the_same_charts() -> Result<(), Setup> {
    let (mesh, _) = mesh_of(&sheet(4, 2))?;
    let mut seams = SeamSet::new();
    sheet_seam(&mesh, &mut seams, SeamKind::Cut, &[(0, 1), (1, 1), (2, 1)])?;
    sheet_seam(&mesh, &mut seams, SeamKind::Dart, &[(2, 1), (2, 2)])?;
    assert_eq!(chart(&mesh, &seams)?, chart(&mesh, &seams)?);
    Ok(())
}

#[test]
fn a_tube_with_no_seam_is_not_a_disk_and_the_error_carries_the_cut_that_fixes_it()
-> Result<(), Setup> {
    let (mesh, _) = mesh_of(&tube(4, 2))?;
    let mut seams = SeamSet::new();
    let Err(EngineError::ChartNotDisk {
        chart: id,
        failure,
        proposals,
    }) = chart(&mesh, &seams)
    else {
        panic!("a tube is not a disk");
    };
    assert_eq!(id, ChartId::new(0));
    assert_eq!(failure, ChartFailure::MultipleBoundaries { count: 2 });
    let [proposal] = &proposals[..] else {
        panic!("one proposal: {proposals:?}");
    };
    // Accepting it, as the maker would, makes the stage succeed.
    for (&edge, &kind) in proposal.add() {
        seams.insert(edge, kind)?;
    }
    let (charts, _) = chart(&mesh, &seams)?.value;
    assert_eq!(charts.len(), 1);
    Ok(())
}

#[test]
fn a_closed_mesh_with_no_seam_is_closed_and_the_error_carries_a_cut() -> Result<(), Setup> {
    let (mesh, _) = mesh_of(OCTAHEDRON)?;
    let Err(EngineError::ChartNotDisk {
        failure, proposals, ..
    }) = chart(&mesh, &SeamSet::new())
    else {
        panic!("a closed mesh is not a disk");
    };
    assert_eq!(failure, ChartFailure::Closed);
    assert_eq!(proposals.len(), 1);
    Ok(())
}

/// Two triangles on one edge, the second folded back `degrees` from lying on the first.
fn book(degrees: f64) -> String {
    let lift = 8.0 * libm::tan(degrees.to_radians());
    format!("v 0 0 0\nv 10 0 0\nv 5 10 0\nv 5 8 {lift}\nf 1 2 3\nf 2 1 4\n")
}

#[test]
fn a_chart_folded_over_itself_is_refused_before_its_topology_is_judged() -> Result<(), Setup> {
    let (mesh, _) = mesh_of(&book(2.0))?;
    let Err(EngineError::FoldedChart { chart: id, edge }) = chart(&mesh, &SeamSet::new()) else {
        panic!("a folded chart is refused");
    };
    assert_eq!(id, ChartId::new(0));
    let (a, b) = (
        vertex_at(&mesh, 0., 0., 0.)?,
        vertex_at(&mesh, 10., 0., 0.)?,
    );
    assert_eq!(edge, edge_between(&mesh, a, b)?);
    Ok(())
}

#[test]
fn a_seam_on_an_edge_the_mesh_does_not_have_is_a_seams_refusal() -> Result<(), Setup> {
    let (mesh, _) = mesh_of(&sheet(2, 1))?;
    unknown_edge_is_refused(&mesh)
}

#[test]
fn an_unknown_seam_edge_is_refused_before_any_chart_is_judged() -> Result<(), Setup> {
    // A tube is not a disk, and a book is folded; the unknown edge comes first all the same.
    let (tube, _) = mesh_of(&tube(4, 2))?;
    unknown_edge_is_refused(&tube)?;
    let (book, _) = mesh_of(&book(2.0))?;
    unknown_edge_is_refused(&book)
}

fn unknown_edge_is_refused(mesh: &ManifoldMesh) -> Result<(), Setup> {
    let beyond = EdgeId::new(u32::try_from(mesh.edge_count())?);
    let mut seams = SeamSet::new();
    seams.insert(beyond, SeamKind::Cut)?;
    assert_eq!(
        chart(mesh, &seams),
        Err(EngineError::Seams(SeamsError::UnknownEdge { edge: beyond }))
    );
    Ok(())
}

/// A tetrahedron with a fin on its edge 0-1.
const FIN: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 0.5 -1 0\n\
f 1 3 2\nf 1 2 4\nf 2 3 4\nf 1 4 3\nf 1 2 5\n";

#[test]
fn the_meshs_copy_groups_sew_a_fin_to_the_body_it_was_split_from() -> Result<(), Setup> {
    let (mesh, report) = mesh_of(FIN)?;
    assert_eq!(mesh.edge_copies(), report.edge_copies());
    let [group] = mesh.edge_copies() else {
        panic!("one group: {:?}", mesh.edge_copies());
    };
    let mut seams = SeamSet::new();
    for &e in group {
        seams.insert(e, SeamKind::Mandatory)?;
    }
    let (_, map) = chart(&mesh, &seams)?.value;
    let [bundle] = map.bundles() else {
        panic!("one bundle: {:?}", map.bundles());
    };
    assert_eq!(bundle.sides().len(), 3);
    assert_eq!(bundle.chains().len(), 2);
    Ok(())
}

#[test]
fn a_seam_on_only_the_fins_copy_has_nothing_to_sew_to() -> Result<(), Setup> {
    let (mesh, _) = mesh_of(FIN)?;
    let [group] = mesh.edge_copies() else {
        panic!("one group");
    };
    let rim = *group
        .iter()
        .find(|e| {
            let h = e.half_edge();
            mesh.face(h).is_none() || mesh.face(h.twin()).is_none()
        })
        .ok_or("a boundary copy")?;
    let mut seams = SeamSet::new();
    seams.insert(rim, SeamKind::Mandatory)?;
    // A cut elsewhere opens the closed body into a disk, so the sewing map is what refuses.
    let (a, b) = (vertex_at(&mesh, 0., 1., 0.)?, vertex_at(&mesh, 0., 0., 1.)?);
    seams.insert(edge_between(&mesh, a, b)?, SeamKind::Cut)?;
    assert_eq!(
        chart(&mesh, &seams),
        Err(EngineError::Seams(SeamsError::UnpairedBoundarySeam {
            chain: SeamChainId::new(rim.index())
        }))
    );
    Ok(())
}

/// A Möbius strip two quads wide and `n` long round a ring of radius 10, turned half a turn
/// over the loop: repair cuts it across where no flips make the faces agree.
fn moebius(n: u32) -> String {
    let mut s = String::new();
    for i in 0..n {
        let a = std::f64::consts::TAU * f64::from(i) / f64::from(n);
        let (radial, up) = (libm::cos(a / 2.0), libm::sin(a / 2.0));
        for w in [-2.0, 0.0, 2.0] {
            let r = 10.0 + w * radial;
            let _ = writeln!(s, "v {} {} {}", r * libm::cos(a), r * libm::sin(a), w * up);
        }
    }
    // Vertex `side` (0, 1, 2 across the rung) of rung `i`; the half turn swaps the sides of
    // rung `n`, which is rung 0 again.
    let id = |i: u32, side: u32| if i == n { 3 - side } else { 3 * i + side + 1 };
    for i in 0..n {
        for side in 0..2 {
            let (a, b) = (id(i, side), id(i, side + 1));
            let (c, d) = (id(i + 1, side), id(i + 1, side + 1));
            let _ = writeln!(s, "f {a} {b} {d}\nf {a} {d} {c}");
        }
    }
    s
}

#[test]
fn a_cut_across_a_strip_sews_as_one_seam_whatever_edges_repair_split_it_into() -> Result<(), Setup>
{
    let (mesh, _) = mesh_of(&moebius(8))?;
    // The cut runs across the strip in three edges, so its two sides are three copies each.
    assert_eq!(mesh.edge_copies().len(), 3, "{:?}", mesh.edge_copies());
    let mut seams = SeamSet::new();
    for &e in mesh.edge_copies().iter().flatten() {
        seams.insert(e, SeamKind::Mandatory)?;
    }
    let (charts, map) = chart(&mesh, &seams)?.value;
    assert_eq!(charts.len(), 1, "the cut strip");
    let [bundle] = map.bundles() else {
        panic!("one seam: {:?}", map.bundles());
    };
    assert_eq!(bundle.chains().len(), 2, "{bundle:?}");
    let [a, b] = bundle.sides() else {
        panic!("{bundle:?}");
    };
    assert_eq!(a.half_edges().len(), 3);
    assert_eq!(b.half_edges().len(), 3);
    Ok(())
}
