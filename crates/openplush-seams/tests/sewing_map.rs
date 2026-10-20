//! The sewing map: each seam chain's sides, one per chart boundary it runs along, in the chain's
//! walking order. Every expectation is worked out from the grid by hand, and an oracle checks each
//! map against the mesh and the seam graph it came from.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use common::{Grid, ROW_1, Setup};
use openplush_mesh::{FaceId, ManifoldMesh};
use openplush_seams::{
    Chart, ChartId, SeamChainId, SeamGraph, SeamKind, SeamSet, SeamsError, SewingMap, charts,
};

/// The map, after checking what must hold for any seam set (see `well_formed`).
fn map(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<Result<SewingMap, SeamsError>, Setup> {
    let graph = SeamGraph::new(mesh, seams)?;
    let charts = charts(mesh, seams)?;
    let map = SewingMap::new(mesh, seams, &graph, &[]);
    if let Ok(map) = &map {
        well_formed(mesh, &graph, &charts, map);
    }
    Ok(map)
}

fn chart_of(charts: &[Chart], f: FaceId) -> Option<ChartId> {
    charts
        .iter()
        .find(|c| c.faces().contains(&f))
        .map(Chart::id)
}

/// One bundle per chain, in chain order, each with two sides: the first runs with the chain's
/// walk (half-edge `i` leaves vertex `i` along edge `i`), the second against it on the twins;
/// every half-edge of a side has a face, all in the side's chart.
fn well_formed(mesh: &ManifoldMesh, graph: &SeamGraph, charts: &[Chart], map: &SewingMap) {
    assert_eq!(map.bundles().len(), graph.chains().len());
    for (bundle, chain) in map.bundles().iter().zip(graph.chains()) {
        assert_eq!(bundle.chain(), chain.id());
        assert_eq!(map.bundle(chain.id()), Some(bundle));
        assert_eq!(bundle.sides().len(), 2, "{bundle:?}: two sides");
        let (along, against) = (&bundle.sides()[0], &bundle.sides()[1]);
        let (edges, vertices) = (chain.edges(), chain.vertices());
        assert_eq!(along.half_edges().len(), edges.len(), "{bundle:?}");
        assert_eq!(against.half_edges().len(), edges.len(), "{bundle:?}");
        for (i, (&h, &g)) in along
            .half_edges()
            .iter()
            .zip(against.half_edges())
            .enumerate()
        {
            assert_eq!(h.edge(), edges[i], "{bundle:?} at {i}");
            assert_eq!(mesh.origin(h), vertices[i], "{bundle:?} at {i}");
            assert_eq!(g, h.twin(), "{bundle:?} at {i}");
            for (side, s) in [(along, h), (against, g)] {
                let f = mesh.face(s);
                assert!(f.is_some(), "{bundle:?} at {i}: no face");
                let chart = f.and_then(|f| chart_of(charts, f));
                assert_eq!(chart, Some(side.chart()), "{bundle:?} at {i}");
            }
        }
    }
}

#[test]
fn no_seams_give_an_empty_map() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let map = map(&grid.mesh, &SeamSet::new())??;
    assert!(map.bundles().is_empty());
    assert_eq!(map.bundle(SeamChainId::new(0)), None);
    Ok(())
}

#[test]
fn a_chain_the_map_does_not_have_finds_no_bundle() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let map = map(&grid.mesh, &seams)??;
    let [bundle] = map.bundles() else {
        panic!("one chain: {:?}", map.bundles());
    };
    // The row's edges are numbered after the bottom row's, so there is room on both sides.
    assert!(bundle.chain().index() > 0, "{bundle:?}");
    for absent in [
        SeamChainId::new(0),
        SeamChainId::new(bundle.chain().index() + 1),
    ] {
        assert_eq!(map.bundle(absent), None, "{absent:?}");
    }
    Ok(())
}

#[test]
fn a_seam_across_a_sheet_joins_its_two_charts() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let map = map(&grid.mesh, &seams)??;
    let [bundle] = map.bundles() else {
        panic!("one chain: {:?}", map.bundles());
    };
    let [a, b] = bundle.sides() else {
        panic!("{bundle:?}");
    };
    assert_ne!(a.chart(), b.chart(), "the two charts of the sheet");
    assert_eq!(a.half_edges().len(), 4);
    Ok(())
}

#[test]
fn a_dart_has_both_sides_in_one_chart() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    let map = map(&grid.mesh, &seams)??;
    let [bundle] = map.bundles() else {
        panic!("one chain: {:?}", map.bundles());
    };
    let [a, b] = bundle.sides() else {
        panic!("{bundle:?}");
    };
    assert_eq!(a.chart(), b.chart());
    assert_eq!(a.chart(), ChartId::new(0));
    assert_ne!(a.half_edges(), b.half_edges());
    Ok(())
}

#[test]
fn an_opening_along_a_seam_is_its_own_bundle_between_the_same_charts() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1[..3])?;
    grid.seam(&mut seams, SeamKind::Opening, &ROW_1[2..])?;
    let map = map(&grid.mesh, &seams)??;
    let [sewn, open] = map.bundles() else {
        panic!("two chains: {:?}", map.bundles());
    };
    let charts_of = |sides: &[_]| -> Vec<ChartId> {
        let mut c: Vec<ChartId> = sides
            .iter()
            .map(openplush_seams::ChainSide::chart)
            .collect();
        c.sort();
        c
    };
    assert_eq!(charts_of(sewn.sides()), charts_of(open.sides()));
    assert_eq!(
        sewn.sides()[0].half_edges().len() + open.sides()[0].half_edges().len(),
        4
    );
    Ok(())
}

#[test]
fn a_ring_seam_and_a_lengthwise_cut_on_a_tube_each_have_two_sides() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(
        &mut seams,
        SeamKind::Cut,
        &[(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)],
    )?;
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 0), (0, 1), (0, 2)])?;
    let map = map(&grid.mesh, &seams)??;
    // The ring, closed on itself at the junction, and the two halves of the lengthwise cut.
    assert_eq!(map.bundles().len(), 3);
    let mut charts: Vec<ChartId> = map
        .bundles()
        .iter()
        .flat_map(|b| b.sides().iter().map(openplush_seams::ChainSide::chart))
        .collect();
    charts.sort();
    charts.dedup();
    assert_eq!(charts.len(), 2, "the tube's two rings of faces");
    Ok(())
}

/// A seam along one rim of a sheet, beside an interior seam, is refused naming the rim's chain.
/// On the bottom rim the faceless half-edges are on the side running against the walk; on the
/// top rim they are on the side running with it.
fn rim_is_refused(rim: &[(u32, u32)]) -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    grid.seam(&mut seams, SeamKind::Mandatory, rim)?;
    let graph = SeamGraph::new(&grid.mesh, &seams)?;
    let rim = graph
        .chains()
        .iter()
        .find(|c| c.kind() == SeamKind::Mandatory)
        .ok_or("the rim chain")?
        .id();
    assert_eq!(
        map(&grid.mesh, &seams)?,
        Err(SeamsError::UnpairedBoundarySeam { chain: rim })
    );
    Ok(())
}

#[test]
fn a_seam_along_the_mesh_boundary_has_no_partner_and_is_refused() -> Result<(), Setup> {
    rim_is_refused(&[(0, 0), (1, 0), (2, 0)])?;
    rim_is_refused(&[(0, 2), (1, 2), (2, 2)])
}
