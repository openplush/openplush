//! The flatten stage: charts in, one flat layout per chart out, in chart order, with the first
//! refusal as the only other outcome. Meshes are written as OBJ and repaired, as a shell would.

use std::fmt::Write as _;

use openplush_io::obj;
use openplush_mesh::{RepairPlan, into_manifold};
use openplush_pipeline::{
    ChartId, ChartMesh, EdgeId, EngineError, FaceId, FlattenError, ManifoldMesh, SeamKind, SeamSet,
    VertexId, chart, flatten,
};

type Setup = Box<dyn std::error::Error>;

fn mesh_of(obj: &str) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(obj.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

/// A `cols` by `rows` grid of 10-unit quads in the plane, each split along its rising diagonal.
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

fn vertex_at(mesh: &ManifoldMesh, c: u32, r: u32) -> Result<VertexId, Setup> {
    let (x, y) = (f64::from(10 * c), f64::from(10 * r));
    (0..u32::try_from(mesh.vertex_count())?)
        .map(VertexId::new)
        .find(|&v| {
            let p = mesh.position(v);
            (p.x, p.y, p.z) == (x, y, 0.0)
        })
        .ok_or_else(|| format!("no vertex at ({c}, {r})").into())
}

fn edge_between(mesh: &ManifoldMesh, a: VertexId, b: VertexId) -> Result<EdgeId, Setup> {
    mesh.outgoing(a)
        .find(|&h| mesh.dest(h) == b)
        .map(|h| h.edge())
        .ok_or_else(|| "no such edge".into())
}

/// Seams of `kind` along a walk through the grid points `(column, row)`.
fn seam(
    mesh: &ManifoldMesh,
    seams: &mut SeamSet,
    kind: SeamKind,
    points: &[(u32, u32)],
) -> Result<(), Setup> {
    for pair in points.windows(2) {
        let (a, b) = (
            vertex_at(mesh, pair[0].0, pair[0].1)?,
            vertex_at(mesh, pair[1].0, pair[1].1)?,
        );
        seams.insert(edge_between(mesh, a, b)?, kind)?;
    }
    Ok(())
}

/// The largest relative difference between a face edge's 3D and flat lengths.
fn worst_stretch(chart: &ChartMesh, uv: &[openplush_pipeline::DVec2]) -> f64 {
    let mut worst = 0.0_f64;
    for (_, corners) in chart.faces() {
        for i in 0..3 {
            let (a, b) = (
                corners[i].index() as usize,
                corners[(i + 1) % 3].index() as usize,
            );
            let rest = (chart.xyz()[a] - chart.xyz()[b]).length();
            let flat = (uv[a] - uv[b]).length();
            worst = worst.max((flat - rest).abs() / rest);
        }
    }
    worst
}

#[test]
fn a_flat_sheet_is_laid_flat_at_its_own_lengths() -> Result<(), Setup> {
    let mesh = mesh_of(&sheet(3, 2))?;
    let (charts, _) = chart(&mesh, &SeamSet::new())?.value;
    let staged = flatten(&mesh, &charts)?;
    assert_eq!(staged.warnings.iter().count(), 0);
    let [layout] = &staged.value[..] else {
        panic!("one layout: {:?}", staged.value.len());
    };
    assert_eq!(layout.chart(), charts[0].chart());
    assert_eq!(layout.uv().len(), charts[0].xyz().len());
    let worst = worst_stretch(&charts[0], layout.uv());
    assert!(worst < 1e-6, "an edge stretched by {worst}");
    Ok(())
}

#[test]
fn every_chart_is_laid_flat_in_chart_order() -> Result<(), Setup> {
    let mesh = mesh_of(&sheet(4, 2))?;
    let mut seams = SeamSet::new();
    seam(
        &mesh,
        &mut seams,
        SeamKind::Cut,
        &[(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
    )?;
    let (charts, _) = chart(&mesh, &seams)?.value;
    let layouts = flatten(&mesh, &charts)?.value;
    assert_eq!(charts.len(), 2);
    assert_eq!(
        layouts.iter().map(|l| l.chart()).collect::<Vec<_>>(),
        charts.iter().map(|c| c.chart()).collect::<Vec<_>>()
    );
    for (c, l) in charts.iter().zip(&layouts) {
        assert_eq!(l.uv().len(), c.xyz().len());
        let worst = worst_stretch(c, l.uv());
        assert!(worst < 1e-6, "chart {:?}: {worst}", c.chart());
    }
    Ok(())
}

#[test]
fn a_chart_slit_by_a_dart_is_laid_flat() -> Result<(), Setup> {
    let mesh = mesh_of(&sheet(4, 2))?;
    let mut seams = SeamSet::new();
    seam(&mesh, &mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    let (charts, _) = chart(&mesh, &seams)?.value;
    let [layout] = &flatten(&mesh, &charts)?.value[..] else {
        panic!("one layout");
    };
    // The slit's two sides are separate chart vertices, each with its own place.
    assert_eq!(layout.uv().len(), mesh.vertex_count() + 1);
    Ok(())
}

#[test]
fn no_charts_flatten_to_no_layouts() -> Result<(), Setup> {
    let mesh = mesh_of(&sheet(1, 1))?;
    assert!(flatten(&mesh, &[])?.value.is_empty());
    Ok(())
}

#[test]
fn the_same_charts_flatten_to_the_same_bits() -> Result<(), Setup> {
    let mesh = mesh_of(&sheet(3, 3))?;
    let (charts, _) = chart(&mesh, &SeamSet::new())?.value;
    let bits = |m: &ManifoldMesh, c: &[ChartMesh]| -> Result<Vec<[u64; 2]>, Setup> {
        Ok(flatten(m, c)?.value[0]
            .uv()
            .iter()
            .map(|p| [p.x.to_bits(), p.y.to_bits()])
            .collect())
    };
    assert_eq!(bits(&mesh, &charts)?, bits(&mesh, &charts)?);
    Ok(())
}

#[test]
fn a_refusal_the_pipeline_has_no_variant_for_is_carried_whole_as_flatten() {
    let inner = FlattenError::Degenerate {
        chart: ChartId::new(3),
        face: FaceId::new(7),
    };
    let e = EngineError::Flatten(inner.clone());
    assert_eq!(e.code(), "Flatten");
    assert_eq!(e.to_string(), inner.to_string());
    let at = e.locations();
    assert_eq!(at.charts, [ChartId::new(3)]);
    assert_eq!(at.faces, [FaceId::new(7)]);
    assert!(e.proposals().is_empty());
    let solve = EngineError::Flatten(FlattenError::SolveFailed {
        chart: ChartId::new(2),
    });
    assert_eq!(solve.locations().charts, [ChartId::new(2)]);
    assert!(solve.locations().faces.is_empty());
}

#[test]
fn an_unclosable_boundary_reports_its_residual_in_model_units() {
    let e = EngineError::BoundaryNotRealizable {
        chart: ChartId::new(3),
        residual: 1.25,
        proposals: Vec::new(),
    };
    assert_eq!(
        e.to_string(),
        "chart 3's seam lengths cannot close flat (residual 1.250 in model units)"
    );
}
