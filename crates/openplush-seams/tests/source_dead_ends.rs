//! Dead-ended cuts read from the source model. A tip is an interior vertex on exactly one edge of
//! the graph over UV cuts, material boundaries and mandatory seams; the cut from it to the first
//! vertex where three or more of those edges meet, or to the surface boundary, is proposed as a
//! dart of its own and left out of the source seam proposal. Most meshes are 6 by 6 sheets whose
//! corner UVs are cut along chosen paths of `(column, row)`; rows 0 and 6 and columns 0 and 6 are
//! the surface boundary.

#[allow(
    dead_code,
    reason = "each test file uses only some of the shared meshes"
)]
mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use common::{Grid, Setup, edge_between, repaired_with_uvs, vertex_at, wedge_uvs};
use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, ManifoldMesh, MaterialIndex, SourceMaterials, VertexId};
use openplush_seams::{ProposalKind, SeamKind, SeamReason, SeamSet, UvCuts, uv_cuts};

const SOURCE_DART: ProposalKind = ProposalKind::Seam {
    reason: SeamReason::SourceDart,
};

/// What a dart with one tip says.
fn one_tip(tip: VertexId) -> String {
    format!(
        "The cut marked in the model that ends at vertex {} stops partway across the surface, so \
         it will be sewn as a dart. If that is not meant, decline it and fix the cut in the \
         modelling tool.",
        tip.index()
    )
}

/// What a dart with a free end at each of `low` and `high` says.
fn two_tips(low: VertexId, high: VertexId) -> String {
    format!(
        "The cut marked in the model between vertices {} and {} stops partway across the surface \
         at both ends, so it will be sewn as a dart. If that is not meant, decline it and fix the \
         cut in the modelling tool.",
        low.index(),
        high.index()
    )
}

/// The edges along `path`.
fn along(grid: &Grid, path: &[(u32, u32)]) -> Result<BTreeSet<EdgeId>, Setup> {
    path.windows(2).map(|w| grid.edge(w[0], w[1])).collect()
}

/// One proposed dart: its edges and its explanation.
#[derive(Debug, PartialEq, Eq)]
struct Dart {
    edges: BTreeSet<EdgeId>,
    explanation: String,
}

/// The darts in proposal order, after checking each is a `SourceDart` of `Dart` edges only, that
/// none shares an edge with another or with the source seam proposal, and that together they
/// and that proposal hold every cut and material boundary.
fn darts(cuts: &UvCuts) -> Result<Vec<Dart>, Setup> {
    let proposals = cuts.darts()?;
    let mut seen: BTreeSet<EdgeId> = seam_edges(cuts)?;
    for p in &proposals {
        assert_eq!(p.kind(), &SOURCE_DART);
        assert!(p.add().values().all(|&k| k == SeamKind::Dart), "{p:?}");
        for &e in p.add().keys() {
            assert!(seen.insert(e), "{e:?} is in two proposals");
        }
    }
    let every: BTreeSet<EdgeId> = cuts
        .edges()
        .iter()
        .chain(cuts.material_boundaries())
        .copied()
        .collect();
    assert_eq!(seen, every, "the proposals hold every source seam once");
    Ok(proposals
        .iter()
        .map(|p| Dart {
            edges: p.add().keys().copied().collect(),
            explanation: p.explanation().to_owned(),
        })
        .collect())
}

/// The edges the source seam proposal adds.
fn seam_edges(cuts: &UvCuts) -> Result<BTreeSet<EdgeId>, Setup> {
    Ok(cuts
        .proposal()?
        .map(|p| p.add().keys().copied().collect())
        .unwrap_or_default())
}

#[test]
fn a_cut_that_stops_inside_the_surface_is_one_dart_from_its_tip() -> Result<(), Setup> {
    let cut = [(0, 3), (1, 3), (2, 3), (3, 3)];
    let grid = Grid::cut_sheet(6, 6, &[&cut], None)?;
    let found = uv_cuts(&grid.mesh, &SeamSet::new())?;
    let edges = along(&grid, &cut)?;
    assert!(found.edges().iter().eq(&edges), "the path is cut");
    let tip = grid.at(3, 3)?;
    assert_eq!(
        darts(&found)?,
        [Dart {
            edges,
            explanation: one_tip(tip)
        }]
    );
    assert_eq!(
        found.proposal()?,
        None,
        "the cut is the dart and nothing else"
    );
    assert_eq!(
        uv_cuts(&grid.mesh, &SeamSet::new())?.darts()?,
        found.darts()?
    );
    Ok(())
}

#[test]
fn a_cut_from_boundary_to_boundary_has_no_tip() -> Result<(), Setup> {
    let row = [(0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (6, 3)];
    let grid = Grid::cut_sheet(6, 6, &[&row], None)?;
    let found = uv_cuts(&grid.mesh, &SeamSet::new())?;
    assert_eq!(darts(&found)?, [] as [Dart; 0]);
    assert_eq!(seam_edges(&found)?, along(&grid, &row)?);
    Ok(())
}

#[test]
fn a_cut_ending_on_another_cut_has_no_tip() -> Result<(), Setup> {
    let row = [(0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (6, 3)];
    let up = [(3, 0), (3, 1), (3, 2), (3, 3)];
    let grid = Grid::cut_sheet(6, 6, &[&row, &up], None)?;
    let found = uv_cuts(&grid.mesh, &SeamSet::new())?;
    assert_eq!(darts(&found)?, [] as [Dart; 0]);
    let mut both = along(&grid, &row)?;
    both.extend(along(&grid, &up)?);
    assert_eq!(seam_edges(&found)?, both);
    Ok(())
}

#[test]
fn a_chain_runs_from_its_tip_to_the_first_junction_and_no_further() -> Result<(), Setup> {
    let row = [(0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (6, 3)];
    let spur = [(3, 3), (3, 4), (4, 4), (4, 5)];
    let grid = Grid::cut_sheet(6, 6, &[&row, &spur], None)?;
    let found = uv_cuts(&grid.mesh, &SeamSet::new())?;
    assert_eq!(
        darts(&found)?,
        [Dart {
            edges: along(&grid, &spur)?,
            explanation: one_tip(grid.at(4, 5)?)
        }]
    );
    assert_eq!(seam_edges(&found)?, along(&grid, &row)?);
    Ok(())
}

#[test]
fn a_cut_ending_on_a_material_boundary_has_no_tip() -> Result<(), Setup> {
    let up = [(3, 0), (3, 1), (3, 2), (3, 3)];
    let two = SourceMaterials::new(
        vec!["Body".to_owned(), "Head".to_owned()],
        vec![Some(MaterialIndex::new(0)), Some(MaterialIndex::new(1))],
    );
    let grid = Grid::cut_sheet(6, 6, &[&up], Some((3, two)))?;
    let found = uv_cuts(&grid.mesh, &SeamSet::new())?;
    let rim = [(0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3), (6, 3)];
    assert!(found.material_boundaries().iter().eq(&along(&grid, &rim)?));
    assert_eq!(darts(&found)?, [] as [Dart; 0]);

    let plain = Grid::cut_sheet(6, 6, &[&up], None)?;
    let found = uv_cuts(&plain.mesh, &SeamSet::new())?;
    assert_eq!(
        darts(&found)?,
        [Dart {
            edges: along(&plain, &up)?,
            explanation: one_tip(plain.at(3, 3)?)
        }],
        "without the boundary the same cut stops partway"
    );
    Ok(())
}

#[test]
fn a_cut_ending_on_a_mandatory_seam_has_no_tip() -> Result<(), Setup> {
    let up = [(3, 0), (3, 1), (3, 2), (3, 3)];
    let grid = Grid::cut_sheet(6, 6, &[&up], None)?;
    let across = [(1, 3), (2, 3), (3, 3), (4, 3), (5, 3)];
    let onward = [(3, 3), (3, 4), (3, 5)];
    for mandatory in [&across[..], &onward[..]] {
        let mut seams = SeamSet::new();
        grid.seam(&mut seams, SeamKind::Mandatory, mandatory)?;
        let found = uv_cuts(&grid.mesh, &seams)?;
        // The mandatory seam's own free ends inside the sheet are not cuts, so no darts either.
        assert_eq!(darts(&found)?, [] as [Dart; 0], "{mandatory:?}");
        assert_eq!(seam_edges(&found)?, along(&grid, &up)?, "{mandatory:?}");
    }
    Ok(())
}

#[test]
fn a_seam_already_in_the_project_does_not_rescue_a_tip() -> Result<(), Setup> {
    let cut = [(0, 3), (1, 3), (2, 3), (3, 3)];
    let grid = Grid::cut_sheet(6, 6, &[&cut], None)?;
    let rest = [(3, 3), (4, 3), (5, 3), (6, 3)];
    let crossing = [(3, 1), (3, 2), (3, 3), (3, 4), (3, 5)];
    for kind in [
        SeamKind::Cut,
        SeamKind::Dart,
        SeamKind::Opening,
        SeamKind::FabricBoundary,
    ] {
        for existing in [&rest[..], &crossing[..]] {
            let mut seams = SeamSet::new();
            grid.seam(&mut seams, kind, existing)?;
            let found = uv_cuts(&grid.mesh, &seams)?;
            assert_eq!(
                darts(&found)?,
                [Dart {
                    edges: along(&grid, &cut)?,
                    explanation: one_tip(grid.at(3, 3)?)
                }],
                "{kind:?} {existing:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn there_is_one_dart_per_tip_and_the_tips_ascend() -> Result<(), Setup> {
    // Laid out so ordering by each chain's lowest edge would put them in another order.
    let cuts: [&[(u32, u32)]; 3] = [
        &[(1, 6), (1, 5), (1, 4), (1, 3), (1, 2), (1, 1)],
        &[(6, 1), (5, 1), (4, 1)],
        &[(6, 5), (5, 5), (4, 5)],
    ];
    let tips = [(1, 1), (4, 1), (4, 5)];
    let grid = Grid::cut_sheet(6, 6, &cuts, None)?;
    let mut want: BTreeMap<VertexId, Dart> = BTreeMap::new();
    for (cut, (c, r)) in cuts.iter().zip(tips) {
        let tip = grid.at(c, r)?;
        let edges = along(&grid, cut)?;
        let explanation = one_tip(tip);
        want.insert(tip, Dart { edges, explanation });
    }
    let mut by_lowest_edge: Vec<(Option<EdgeId>, VertexId)> = want
        .iter()
        .map(|(&tip, d)| (d.edges.first().copied(), tip))
        .collect();
    by_lowest_edge.sort();
    assert!(
        !by_lowest_edge.iter().map(|(_, tip)| tip).eq(want.keys()),
        "{by_lowest_edge:?}"
    );
    let found = darts(&uv_cuts(&grid.mesh, &SeamSet::new())?)?;
    assert_eq!(found, want.into_values().collect::<Vec<_>>());
    Ok(())
}

#[test]
fn a_slit_with_two_free_ends_is_one_chain_owned_by_its_lower_tip() -> Result<(), Setup> {
    let slit = [(1, 4), (2, 4), (3, 4), (4, 4), (5, 4)];
    let grid = Grid::cut_sheet(6, 6, &[&slit], None)?;
    let found = uv_cuts(&grid.mesh, &SeamSet::new())?;
    let (a, b) = (grid.at(1, 4)?, grid.at(5, 4)?);
    let (low, high) = (a.min(b), a.max(b));
    assert_eq!(
        darts(&found)?,
        [Dart {
            edges: along(&grid, &slit)?,
            explanation: two_tips(low, high)
        }]
    );
    Ok(())
}

#[test]
fn a_slit_takes_its_place_among_the_darts_by_its_lower_tip() -> Result<(), Setup> {
    // The other cut's tip lies between the slit's two in vertex order, so a slit placed by its
    // upper tip, or proposed once per tip, would come out elsewhere.
    let slit = [(3, 1), (3, 2), (3, 3), (3, 4), (3, 5)];
    let other = [(0, 3), (1, 3)];
    let grid = Grid::cut_sheet(6, 6, &[&slit, &other], None)?;
    let (a, b) = (grid.at(3, 1)?, grid.at(3, 5)?);
    let (low, high) = (a.min(b), a.max(b));
    let tip = grid.at(1, 3)?;
    assert!(low < tip && tip < high, "{low:?} {tip:?} {high:?}");
    let found = darts(&uv_cuts(&grid.mesh, &SeamSet::new())?)?;
    let slit_dart = Dart {
        edges: along(&grid, &slit)?,
        explanation: two_tips(low, high),
    };
    let other_dart = Dart {
        edges: along(&grid, &other)?,
        explanation: one_tip(tip),
    };
    assert_eq!(found, [slit_dart, other_dart]);
    Ok(())
}

/// A closed globe: a pole above, five rings of eight round it, a pole below. Vertex 0 is the top
/// pole, ring `k` (1 to 5) holds vertices `8(k - 1) + 1` to `8k`, and 41 is the bottom pole.
struct Globe {
    positions: Vec<DVec3>,
    triangles: Vec<[u32; 3]>,
}

const SEGMENTS: u32 = 8;
const RINGS: u32 = 5;

impl Globe {
    fn new() -> Self {
        let point = |k: u32, i: u32| {
            let (lat, around) = (
                PI * f64::from(k) / f64::from(RINGS + 1),
                2.0 * PI * f64::from(i) / f64::from(SEGMENTS),
            );
            DVec3::new(
                10.0 * lat.sin() * around.cos(),
                10.0 * lat.sin() * around.sin(),
                10.0 * lat.cos(),
            )
        };
        let bottom = RINGS * SEGMENTS + 1;
        let mut positions = vec![DVec3::new(0.0, 0.0, 10.0)];
        positions.extend((1..=RINGS).flat_map(|k| (0..SEGMENTS).map(move |i| point(k, i))));
        positions.push(DVec3::new(0.0, 0.0, -10.0));
        let mut triangles = Vec::new();
        for i in 0..SEGMENTS {
            let j = (i + 1) % SEGMENTS;
            triangles.push([0, Self::id(1, i), Self::id(1, j)]);
            for k in 1..RINGS {
                let (a, b) = (Self::id(k, i), Self::id(k, j));
                let (c, d) = (Self::id(k + 1, j), Self::id(k + 1, i));
                triangles.extend([[a, d, c], [a, c, b]]);
            }
            triangles.push([Self::id(RINGS, i), bottom, Self::id(RINGS, j)]);
        }
        Self {
            positions,
            triangles,
        }
    }

    fn id(k: u32, i: u32) -> u32 {
        (k - 1) * SEGMENTS + i % SEGMENTS + 1
    }

    /// The meridian through segment 0, top pole to bottom, as soup index pairs.
    fn meridian() -> Vec<[u32; 2]> {
        let mut path = vec![0];
        path.extend((1..=RINGS).map(|k| Self::id(k, 0)));
        path.push(RINGS * SEGMENTS + 1);
        path.windows(2)
            .map(|w| [w[0].min(w[1]), w[0].max(w[1])])
            .collect()
    }

    /// Round ring 3, the equator.
    fn equator() -> Vec<[u32; 2]> {
        (0..SEGMENTS)
            .map(|i| {
                let (a, b) = (Self::id(3, i), Self::id(3, i + 1));
                [a.min(b), a.max(b)]
            })
            .collect()
    }

    /// The mesh with exactly `cuts` as UV cuts.
    fn cut(&self, cuts: &[[u32; 2]]) -> Result<ManifoldMesh, Setup> {
        let uvs = wedge_uvs(&self.triangles, &cuts.iter().copied().collect());
        Ok(repaired_with_uvs(self.positions.clone(), self.triangles.clone(), Some(uvs))?.0)
    }

    fn edges(&self, mesh: &ManifoldMesh, pairs: &[[u32; 2]]) -> Result<BTreeSet<EdgeId>, Setup> {
        let at = |i: u32| self.positions[i as usize];
        pairs
            .iter()
            .map(|&[a, b]| edge_between(mesh, at(a), at(b)))
            .collect()
    }

    fn vertex(&self, mesh: &ManifoldMesh, i: u32) -> Result<VertexId, Setup> {
        vertex_at(mesh, self.positions[i as usize])
    }
}

#[test]
fn a_pole_to_pole_meridian_on_a_closed_globe_is_one_chain_with_a_tip_at_each_pole()
-> Result<(), Setup> {
    let globe = Globe::new();
    let meridian = Globe::meridian();
    let mesh = globe.cut(&meridian)?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    let edges = globe.edges(&mesh, &meridian)?;
    assert!(found.edges().iter().eq(&edges), "the meridian is cut");
    let (top, bottom) = (globe.vertex(&mesh, 0)?, globe.vertex(&mesh, 41)?);
    assert_eq!(
        darts(&found)?,
        [Dart {
            edges,
            explanation: two_tips(top.min(bottom), top.max(bottom))
        }]
    );
    assert_eq!(found.proposal()?, None);
    Ok(())
}

#[test]
fn a_meridian_crossing_the_equator_is_two_darts_meeting_there() -> Result<(), Setup> {
    let globe = Globe::new();
    let (meridian, equator) = (Globe::meridian(), Globe::equator());
    let mesh = globe.cut(&[meridian.clone(), equator.clone()].concat())?;
    let found = uv_cuts(&mesh, &SeamSet::new())?;
    let (top, bottom) = (globe.vertex(&mesh, 0)?, globe.vertex(&mesh, 41)?);
    let half = |pairs: &[[u32; 2]], tip: VertexId| -> Result<(VertexId, Dart), Setup> {
        let edges = globe.edges(&mesh, pairs)?;
        Ok((
            tip,
            Dart {
                edges,
                explanation: one_tip(tip),
            },
        ))
    };
    let want = BTreeMap::from([half(&meridian[..3], top)?, half(&meridian[3..], bottom)?]);
    assert_eq!(darts(&found)?, want.into_values().collect::<Vec<_>>());
    assert_eq!(seam_edges(&found)?, globe.edges(&mesh, &equator)?);
    Ok(())
}
