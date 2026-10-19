//! The cuts that close a chart that is not a disk.

use std::collections::BTreeMap;

use openplush_mesh::{EdgeId, ManifoldMesh};

use crate::paths::CutOpen;
use crate::{
    Chart, ChartFailure, Proposal, ProposalKind, SeamKind, SeamSet, SeamsError, charts, check_disk,
};

/// One proposal of `Cut` edges that leaves `chart`, from `charts(mesh, seams)`, a disk, or `None`
/// when it is one. Every search runs over the chart cut open along `seams`, on edges with a chart
/// face on both sides, each costing its 3D length. A closed chart is cut between its two vertices
/// of largest angle defect; otherwise its boundary loops are joined into one, then its handles
/// cut by tree-cotree.
///
/// # Errors
/// `ChartNotClosed` when the cuts found do not leave the chart's faces one disk; it is never
/// offered. `MandatorySeam` cannot arise, as only edges that are not seams are cut.
pub fn close_chart(
    mesh: &ManifoldMesh,
    seams: &SeamSet,
    chart: &Chart,
) -> Result<Option<Proposal>, SeamsError> {
    let Err(failure) = check_disk(mesh, seams, chart) else {
        return Ok(None);
    };
    let not_closed = || SeamsError::ChartNotClosed { chart: chart.id() };
    let mut working = seams.clone();
    let mut add = BTreeMap::new();
    if failure == ChartFailure::Closed {
        let found = CutOpen::new(mesh, &working, chart).between_sharpest();
        cut(&mut working, &mut add, found.ok_or_else(not_closed)?)?;
    } else {
        let found = CutOpen::new(mesh, &working, chart).joining_loops();
        cut(&mut working, &mut add, found.ok_or_else(not_closed)?)?;
        if let Err(ChartFailure::PositiveGenus { .. }) = check_disk(mesh, &working, chart) {
            let found = CutOpen::new(mesh, &working, chart).handle_loops();
            cut(&mut working, &mut add, found.ok_or_else(not_closed)?)?;
        }
    }
    if !leaves_disk(mesh, &working, chart)? {
        return Err(not_closed());
    }
    let explanation = explain(failure, add.len());
    let kind = ProposalKind::CloseChart {
        chart: chart.id(),
        failure,
    };
    Proposal::new(kind, add, explanation).map(Some)
}

/// Whether `chart`'s faces are still one chart under `seams`, and that chart a disk.
fn leaves_disk(mesh: &ManifoldMesh, seams: &SeamSet, chart: &Chart) -> Result<bool, SeamsError> {
    Ok(charts(mesh, seams)?
        .iter()
        .find(|c| c.faces() == chart.faces())
        .is_some_and(|c| check_disk(mesh, seams, c).is_ok()))
}

fn cut(
    working: &mut SeamSet,
    add: &mut BTreeMap<EdgeId, SeamKind>,
    edges: Vec<EdgeId>,
) -> Result<(), SeamsError> {
    for edge in edges {
        if add.insert(edge, SeamKind::Cut).is_none() {
            working.insert(edge, SeamKind::Cut)?;
        }
    }
    Ok(())
}

fn explain(failure: ChartFailure, edges: usize) -> String {
    let why = match failure {
        ChartFailure::Closed => "is closed all round".to_owned(),
        ChartFailure::MultipleBoundaries { count } => format!("has {count} separate open edges"),
        ChartFailure::PositiveGenus { genus: 1 } => "wraps round a handle".to_owned(),
        ChartFailure::PositiveGenus { genus } => format!("wraps round {genus} handles"),
    };
    let edges = match edges {
        1 => "one more edge".to_owned(),
        n => format!("{n} more edges"),
    };
    format!(
        "This part {why}, so it cannot lie flat as one piece. Cutting along {edges} opens it into \
         one piece that can."
    )
}

#[cfg(test)]
mod tests {
    use openplush_geom::DVec3;
    use openplush_mesh::{RepairPlan, SourceCorner, TriangleSoup, into_manifold};

    use super::*;

    /// A `cols` by `rows` sheet of unit quads, each split along its rising diagonal.
    fn sheet(cols: u32, rows: u32) -> ManifoldMesh {
        let id = |c: u32, r: u32| r * (cols + 1) + c;
        let positions = (0..=rows)
            .flat_map(|r| (0..=cols).map(move |c| DVec3::new(f64::from(c), f64::from(r), 0.0)))
            .collect();
        let triangles: Vec<[u32; 3]> = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (c, r)))
            .flat_map(|(c, r)| {
                let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
                [[a, b, d], [a, d, e]]
            })
            .collect();
        let corners = (0..3 * triangles.len() as u32)
            .map(|vertex| SourceCorner {
                primitive: 0,
                vertex,
            })
            .collect();
        let soup = TriangleSoup::new(positions, triangles, corners, None).expect("a valid soup");
        into_manifold(&soup, &RepairPlan::default())
            .expect("a manifold sheet")
            .0
    }

    /// The edge between the vertices at `a` and `b`.
    fn edge(mesh: &ManifoldMesh, a: DVec3, b: DVec3) -> EdgeId {
        let at = |p: DVec3| {
            (0..mesh.vertex_count() as u32)
                .map(openplush_mesh::VertexId::new)
                .find(|&v| mesh.position(v) == p)
                .expect("a vertex there")
        };
        let (from, to) = (at(a), at(b));
        mesh.outgoing(from)
            .find(|&h| mesh.dest(h) == to)
            .expect("an edge there")
            .edge()
    }

    #[test]
    fn a_chart_cut_in_two_is_not_left_a_disk() {
        let mesh = sheet(2, 1);
        let whole = charts(&mesh, &SeamSet::new()).expect("charts").remove(0);
        assert!(leaves_disk(&mesh, &SeamSet::new(), &whole).expect("known edges"));
        let mut seams = SeamSet::new();
        let across = edge(&mesh, DVec3::new(1.0, 0.0, 0.0), DVec3::new(1.0, 1.0, 0.0));
        seams.insert(across, SeamKind::Cut).expect("a free edge");
        // Both halves are disks, but neither is the chart's faces.
        assert!(!leaves_disk(&mesh, &seams, &whole).expect("known edges"));
    }

    #[test]
    fn a_chart_that_stays_whole_but_not_a_disk_is_not_left_one() {
        let mesh = sheet(3, 2);
        let whole = charts(&mesh, &SeamSet::new()).expect("charts").remove(0);
        let mut seams = SeamSet::new();
        // A slit between the two interior vertices: one chart, two boundary loops.
        let slit = edge(&mesh, DVec3::new(1.0, 1.0, 0.0), DVec3::new(2.0, 1.0, 0.0));
        seams.insert(slit, SeamKind::Cut).expect("a free edge");
        assert_eq!(charts(&mesh, &seams).expect("charts").len(), 1);
        assert!(!leaves_disk(&mesh, &seams, &whole).expect("known edges"));
    }
}
