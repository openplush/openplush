//! The fold check: a chart whose surface lies back over itself is refused before flattening, with
//! the chart and the lowest folded edge named. The oracle recomputes every dihedral from positions.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use std::f64::consts::{PI, TAU};

use common::{Setup, repaired};
use openplush_geom::DVec3;
use openplush_geom::epsilon::FOLD_ANGLE;
use openplush_mesh::{EdgeId, FaceId, ManifoldMesh};
use openplush_seams::{SeamKind, SeamSet, SeamsError, charts, check_fold};

/// The synthetic cap of `research/boundary_pinned/`: a disk of `rings` concentric rings (ring `k` has `6k` vertices) mapped onto
/// the unit sphere with the polar angle proportional to the radius, the rim at `rim_deg`. Past
/// 180° the surface wraps round the south pole and lies back over itself.
fn cap(rim_deg: f64, rings: u32) -> Result<ManifoldMesh, Setup> {
    let mut polar = vec![(0.0, 0.0)];
    let mut ring_ids: Vec<Vec<u32>> = vec![vec![0]];
    for k in 1..=rings {
        let n = 6 * k;
        let ids = (0..n)
            .map(|j| {
                polar.push((
                    f64::from(k) / f64::from(rings),
                    TAU * f64::from(j) / f64::from(n),
                ));
                u32::try_from(polar.len() - 1)
            })
            .collect::<Result<_, _>>()?;
        ring_ids.push(ids);
    }
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let rim = &ring_ids[1];
    for j in 0..rim.len() {
        triangles.push([0, rim[j], rim[(j + 1) % rim.len()]]);
    }
    for k in 2..=rings as usize {
        zip_rings(&ring_ids[k - 1], &ring_ids[k], &mut triangles);
    }
    let scale = rim_deg.to_radians();
    let positions = polar
        .iter()
        .map(|&(rho, azimuth)| {
            let theta = scale * rho;
            DVec3::new(
                theta.sin() * azimuth.cos(),
                theta.sin() * azimuth.sin(),
                theta.cos(),
            )
        })
        .collect();
    repaired(positions, triangles)
}

/// Triangles between two rings, walked by angle so each step advances the ring whose next vertex
/// comes first; counter-clockwise seen from outside the cap's pole.
fn zip_rings(inner: &[u32], outer: &[u32], triangles: &mut Vec<[u32; 3]>) {
    let (n1, n2) = (inner.len(), outer.len());
    let (mut i, mut j) = (0, 0);
    while i < n1 || j < n2 {
        if i < n1 && (j >= n2 || (i + 1) * n2 <= (j + 1) * n1) {
            triangles.push([inner[i], outer[j % n2], inner[(i + 1) % n1]]);
            i += 1;
        } else {
            triangles.push([inner[i % n1], outer[j], outer[(j + 1) % n2]]);
            j += 1;
        }
    }
}

/// Two triangles on the edge from `(0, 0, 0)` to `(10, 0, 0)`, the second folded back over the
/// first about that edge until their normals are `apart_deg` from antiparallel.
fn book(apart_deg: f64) -> Result<ManifoldMesh, Setup> {
    let lift = 8.0 * apart_deg.to_radians().tan();
    repaired(
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(5.0, 10.0, 0.0),
            DVec3::new(5.0, 8.0, lift),
        ],
        vec![[0, 1, 2], [1, 0, 3]],
    )
}

fn unit_normal(mesh: &ManifoldMesh, f: FaceId) -> Result<DVec3, Setup> {
    let h = mesh.face_half_edge(f);
    let [a, b, c] = [
        mesh.position(mesh.origin(h)),
        mesh.position(mesh.dest(h)),
        mesh.position(mesh.dest(mesh.next(h))),
    ];
    let n = DVec3::new(b.x - a.x, b.y - a.y, b.z - a.z).cross(DVec3::new(
        c.x - a.x,
        c.y - a.y,
        c.z - a.z,
    ));
    n.normalize().ok_or_else(|| "zero-area face".into())
}

/// Oracle: every interior edge whose two faces' unit normals are within `FOLD_ANGLE` of
/// antiparallel, from positions alone, in ascending order.
fn folded_edges(mesh: &ManifoldMesh) -> Result<Vec<EdgeId>, Setup> {
    let limit = -(FOLD_ANGLE.cos());
    let mut folded = Vec::new();
    for e in 0..u32::try_from(mesh.edge_count())? {
        let h = EdgeId::new(e).half_edge();
        let (Some(f), Some(g)) = (mesh.face(h), mesh.face(h.twin())) else {
            continue;
        };
        if unit_normal(mesh, f)?.dot(unit_normal(mesh, g)?) < limit {
            folded.push(EdgeId::new(e));
        }
    }
    Ok(folded)
}

/// The verdict for the one chart of a mesh with no seams.
fn verdict(mesh: &ManifoldMesh) -> Result<Result<(), SeamsError>, Setup> {
    let seams = SeamSet::new();
    let charts = charts(mesh, &seams)?;
    assert_eq!(charts.len(), 1);
    Ok(check_fold(mesh, &seams, &charts[0]))
}

#[test]
fn a_cap_short_of_the_pole_is_not_folded() -> Result<(), Setup> {
    let mesh = cap(175.0, 8)?;
    assert_eq!(folded_edges(&mesh)?, [], "the oracle agrees");
    assert_eq!(verdict(&mesh)?, Ok(()));
    Ok(())
}

#[test]
fn a_cap_past_the_pole_is_refused_at_its_lowest_folded_edge() -> Result<(), Setup> {
    let mesh = cap(190.0, 8)?;
    let folded = folded_edges(&mesh)?;
    assert!(!folded.is_empty(), "the oracle finds the fold");
    let refused = verdict(&mesh)?;
    let Err(SeamsError::FoldedChart { chart, edge }) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(chart.index(), 0);
    assert_eq!(edge, folded[0]);
    Ok(())
}

#[test]
fn the_fold_angle_is_the_threshold_either_side_of_five_degrees() -> Result<(), Setup> {
    assert_eq!(FOLD_ANGLE, 5.0 * PI / 180.0);
    assert_eq!(verdict(&book(10.0)?)?, Ok(()));
    let mesh = book(2.0)?;
    let refused = verdict(&mesh)?;
    let Err(SeamsError::FoldedChart { chart, edge }) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(chart.index(), 0);
    let h = edge.half_edge();
    let mut ends = [mesh.origin(h), mesh.dest(h)].map(|v| mesh.position(v));
    ends.sort_by(|a, b| a.x.total_cmp(&b.x));
    assert_eq!(
        ends,
        [DVec3::new(0.0, 0.0, 0.0), DVec3::new(10.0, 0.0, 0.0)],
        "the shared edge"
    );
    Ok(())
}

#[test]
fn a_fold_along_a_seam_is_two_pieces_not_a_fold() -> Result<(), Setup> {
    let mesh = book(0.0)?;
    let mut seams = SeamSet::new();
    seams.insert(folded_edges(&mesh)?[0], SeamKind::Cut)?;
    let charts = charts(&mesh, &seams)?;
    assert_eq!(charts.len(), 2);
    for chart in &charts {
        assert_eq!(check_fold(&mesh, &seams, chart), Ok(()));
    }
    Ok(())
}

#[test]
fn a_flat_sheet_has_no_fold() -> Result<(), Setup> {
    let flat = repaired(
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(5.0, 10.0, 0.0),
            DVec3::new(5.0, -10.0, 0.0),
        ],
        vec![[0, 1, 2], [1, 0, 3]],
    )?;
    assert_eq!(verdict(&flat)?, Ok(()));
    Ok(())
}

#[test]
fn the_folded_chart_is_named_when_it_is_not_the_first() -> Result<(), Setup> {
    // Face 0 lies flat beside face 1, cut from it by a seam; face 2 is folded back over face 1
    // about their shared edge from (0, 0, 0) to (5, -10, 0), 2° from antiparallel.
    let lift = 80.0 * 2.0_f64.to_radians().tan() / 125.0_f64.sqrt();
    let mesh = repaired(
        vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
            DVec3::new(5.0, 10.0, 0.0),
            DVec3::new(5.0, -10.0, 0.0),
            DVec3::new(8.0, 0.0, lift),
        ],
        vec![[0, 1, 2], [1, 0, 3], [3, 0, 4]],
    )?;
    let mut seams = SeamSet::new();
    let shared = |mesh: &ManifoldMesh, a: DVec3, b: DVec3| -> Result<EdgeId, Setup> {
        (0..u32::try_from(mesh.edge_count())?)
            .map(|e| EdgeId::new(e).half_edge())
            .find(|&h| {
                let ends = [mesh.position(mesh.origin(h)), mesh.position(mesh.dest(h))];
                ends == [a, b] || ends == [b, a]
            })
            .map(|h| h.edge())
            .ok_or_else(|| "no such edge".into())
    };
    let cut = shared(&mesh, DVec3::new(0.0, 0.0, 0.0), DVec3::new(10.0, 0.0, 0.0))?;
    seams.insert(cut, SeamKind::Cut)?;
    let charts = charts(&mesh, &seams)?;
    assert_eq!(charts.len(), 2);
    assert_eq!(check_fold(&mesh, &seams, &charts[0]), Ok(()));
    let fold = shared(
        &mesh,
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(5.0, -10.0, 0.0),
    )?;
    assert_eq!(
        check_fold(&mesh, &seams, &charts[1]),
        Err(SeamsError::FoldedChart {
            chart: charts[1].id(),
            edge: fold
        })
    );
    assert_eq!(charts[1].id().index(), 1);
    Ok(())
}
