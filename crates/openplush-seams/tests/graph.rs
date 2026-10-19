//! The seam graph: nodes where chains must end, and every seam edge in exactly one chain, each
//! chain numbered and walked the same way every time.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use openplush_geom::DVec3;
use openplush_mesh::{EdgeId, ManifoldMesh, RepairPlan, SourceCorner, TriangleSoup, VertexId};
use openplush_seams::{SeamChain, SeamGraph, SeamKind, SeamNode, SeamSet, SeamsError};

type Setup = Box<dyn std::error::Error>;

/// Across a 4-column sheet's middle row, or once round a 4-column tube.
const ROW_1: [(u32, u32); 5] = [(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)];

/// A `cols` by `rows` grid of quads, two counter-clockwise triangles each, split along the
/// diagonal from `(c, r)` to `(c + 1, r + 1)`. A flat sheet, or with `tube` an open tube whose
/// last column joins the first.
struct Grid {
    mesh: ManifoldMesh,
    cols: u32,
    rows: u32,
    tube: bool,
}

impl Grid {
    fn sheet(cols: u32, rows: u32) -> Result<Self, Setup> {
        Self::build(cols, rows, false)
    }

    fn tube(cols: u32, rows: u32) -> Result<Self, Setup> {
        Self::build(cols, rows, true)
    }

    fn build(cols: u32, rows: u32, tube: bool) -> Result<Self, Setup> {
        let ring = if tube { cols } else { cols + 1 };
        let point = |c: u32, r: u32| {
            let (c, r) = (f64::from(c), f64::from(r));
            if tube {
                let a = TAU * c / f64::from(cols);
                DVec3::new(10.0 * a.cos(), 10.0 * a.sin(), 10.0 * r)
            } else {
                DVec3::new(10.0 * c, 10.0 * r, 0.0)
            }
        };
        let positions = (0..=rows)
            .flat_map(|r| (0..ring).map(move |c| (c, r)))
            .map(|(c, r)| point(c, r))
            .collect();
        let id = |c: u32, r: u32| r * ring + c % ring;
        let triangles: Vec<[u32; 3]> = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (c, r)))
            .flat_map(|(c, r)| {
                let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
                [[a, b, d], [a, d, e]]
            })
            .collect();
        let corners = (0..)
            .take(3 * triangles.len())
            .map(|vertex| SourceCorner {
                primitive: 0,
                vertex,
            })
            .collect();
        let soup = TriangleSoup::new(positions, triangles, corners, None)?;
        let (mesh, _) = openplush_mesh::into_manifold(&soup, &RepairPlan::default())?;
        Ok(Self {
            mesh,
            cols,
            rows,
            tube,
        })
    }

    /// The vertex at column `c`, row `r`, found by position so repair's numbering is not assumed.
    fn at(&self, c: u32, r: u32) -> Result<VertexId, Setup> {
        let c = if self.tube { c % self.cols } else { c };
        assert!(
            c <= self.cols && r <= self.rows,
            "({c}, {r}) is on the grid"
        );
        let want = if self.tube {
            let a = TAU * f64::from(c) / f64::from(self.cols);
            DVec3::new(10.0 * a.cos(), 10.0 * a.sin(), 10.0 * f64::from(r))
        } else {
            DVec3::new(10.0 * f64::from(c), 10.0 * f64::from(r), 0.0)
        };
        (0..self.mesh.vertex_count() as u32)
            .map(VertexId::new)
            .find(|&v| self.mesh.position(v) == want)
            .ok_or_else(|| format!("no vertex at ({c}, {r})").into())
    }

    /// The edge from `a` to `b`, each a `(column, row)`.
    fn edge(&self, a: (u32, u32), b: (u32, u32)) -> Result<EdgeId, Setup> {
        let (from, to) = (self.at(a.0, a.1)?, self.at(b.0, b.1)?);
        self.mesh
            .outgoing(from)
            .find(|&h| self.mesh.dest(h) == to)
            .map(|h| h.edge())
            .ok_or_else(|| format!("no edge {a:?} to {b:?}").into())
    }

    /// Seams of `kind` along the path through `points`.
    fn seam(
        &self,
        seams: &mut SeamSet,
        kind: SeamKind,
        points: &[(u32, u32)],
    ) -> Result<(), Setup> {
        for pair in points.windows(2) {
            seams.insert(self.edge(pair[0], pair[1])?, kind)?;
        }
        Ok(())
    }

    /// The graph, after checking what must hold for any seam set (see `well_formed`).
    fn graph(&self, seams: &SeamSet) -> Result<SeamGraph, Setup> {
        let graph = SeamGraph::new(&self.mesh, seams)?;
        well_formed(&self.mesh, seams, &graph);
        Ok(graph)
    }
}

/// Every seam edge in exactly one chain; each chain a walk whose vertex `i` and `i + 1` are its
/// edge `i`'s ends, of one kind, numbered by its lowest edge, its inner vertices never nodes and
/// its ends nodes unless it is closed; the chains in ID order.
fn well_formed(mesh: &ManifoldMesh, seams: &SeamSet, graph: &SeamGraph) {
    let nodes: BTreeMap<VertexId, SeamNode> = graph.nodes().collect();
    let mut seen: Vec<EdgeId> = Vec::new();
    for chain in graph.chains() {
        let (edges, vertices) = (chain.edges(), chain.vertices());
        assert_eq!(vertices.len(), edges.len() + 1, "{chain:?}");
        for (i, &e) in edges.iter().enumerate() {
            let h = e.half_edge();
            let ends = [mesh.origin(h), mesh.dest(h)];
            let walked = [vertices[i], vertices[i + 1]];
            assert!(
                ends == walked || ends == [walked[1], walked[0]],
                "{chain:?} at {i}"
            );
            assert_eq!(seams.get(e), Some(chain.kind()), "{chain:?} at {i}");
        }
        assert_eq!(
            edges.iter().min().map(|e| e.index()),
            Some(chain.id().index())
        );
        for v in &vertices[1..vertices.len() - 1] {
            assert!(
                !nodes.contains_key(v),
                "{chain:?} passes through node {v:?}"
            );
        }
        let (first, last) = (vertices[0], vertices[vertices.len() - 1]);
        if first != last || nodes.contains_key(&first) {
            assert!(
                nodes.contains_key(&first) && nodes.contains_key(&last),
                "{chain:?}"
            );
        }
        seen.extend_from_slice(edges);
    }
    let ids: Vec<_> = graph.chains().iter().map(SeamChain::id).collect();
    assert!(
        ids.is_sorted() && ids.windows(2).all(|w| w[0] != w[1]),
        "{ids:?}"
    );
    seen.sort();
    let all: Vec<EdgeId> = seams.iter().map(|(e, _)| e).collect();
    assert_eq!(seen, all, "every seam edge is in exactly one chain");
}

fn nodes(graph: &SeamGraph) -> Vec<(VertexId, SeamNode)> {
    graph.nodes().collect()
}

fn ends(chain: &SeamChain) -> (VertexId, VertexId) {
    let v = chain.vertices();
    (v[0], v[v.len() - 1])
}

#[test]
fn no_seams_give_no_nodes_and_no_chains() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let graph = grid.graph(&SeamSet::new())?;
    assert_eq!(nodes(&graph), []);
    assert!(graph.chains().is_empty());
    Ok(())
}

#[test]
fn a_seam_across_a_sheet_is_one_chain_between_two_boundary_ends() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let graph = grid.graph(&seams)?;
    let (left, right) = (grid.at(0, 1)?, grid.at(4, 1)?);
    let mut want = vec![
        (left, SeamNode::BoundaryEnd),
        (right, SeamNode::BoundaryEnd),
    ];
    want.sort();
    assert_eq!(nodes(&graph), want);
    let [chain] = graph.chains() else {
        panic!("one chain: {:?}", graph.chains());
    };
    assert_eq!(chain.kind(), SeamKind::Cut);
    assert_eq!(chain.edges().len(), 4);
    // Two different ends: the walk starts at the lower-numbered one.
    assert_eq!(ends(chain), (left.min(right), left.max(right)));
    Ok(())
}

#[test]
fn a_kind_change_ends_one_chain_and_starts_the_next() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 1), (1, 1), (2, 1)])?;
    grid.seam(&mut seams, SeamKind::Opening, &[(2, 1), (3, 1), (4, 1)])?;
    let graph = grid.graph(&seams)?;
    let middle = grid.at(2, 1)?;
    assert!(nodes(&graph).contains(&(middle, SeamNode::KindChange)));
    assert_eq!(nodes(&graph).len(), 3);
    let kinds: Vec<_> = graph.chains().iter().map(SeamChain::kind).collect();
    let mut want = vec![SeamKind::Cut, SeamKind::Opening];
    if graph.chains()[0]
        .edges()
        .contains(&grid.edge((2, 1), (3, 1))?)
    {
        want.reverse();
    }
    assert_eq!(kinds, want);
    Ok(())
}

#[test]
fn the_same_kind_on_both_sides_is_no_node() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Opening, &[(0, 1), (1, 1), (2, 1)])?;
    grid.seam(&mut seams, SeamKind::Opening, &[(2, 1), (3, 1), (4, 1)])?;
    let graph = grid.graph(&seams)?;
    assert_eq!(nodes(&graph).len(), 2);
    assert_eq!(graph.chains().len(), 1);
    Ok(())
}

#[test]
fn three_seams_meeting_make_a_junction_that_ends_three_chains() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    grid.seam(&mut seams, SeamKind::Cut, &[(2, 1), (2, 2)])?;
    let graph = grid.graph(&seams)?;
    let middle = grid.at(2, 1)?;
    assert!(nodes(&graph).contains(&(middle, SeamNode::Junction)));
    assert_eq!(nodes(&graph).len(), 4);
    assert_eq!(graph.chains().len(), 3);
    Ok(())
}

#[test]
fn a_seam_ending_inside_the_sheet_ends_at_a_dart_tip() -> Result<(), Setup> {
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Dart, &[(2, 0), (2, 1)])?;
    let graph = grid.graph(&seams)?;
    let (mouth, tip) = (grid.at(2, 0)?, grid.at(2, 1)?);
    let mut want = vec![(mouth, SeamNode::BoundaryEnd), (tip, SeamNode::DartTip)];
    want.sort();
    assert_eq!(nodes(&graph), want);
    assert_eq!(graph.chains().len(), 1);
    Ok(())
}

#[test]
fn a_seam_along_part_of_the_boundary_chains_like_any_seam() -> Result<(), Setup> {
    // The corner (0, 0) has seam-valence 1, but a plain boundary edge leaves it too, so it is
    // a boundary end rather than a dart tip; (1, 0) has only seams along the boundary.
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Mandatory, &[(0, 0), (1, 0), (2, 0)])?;
    let graph = grid.graph(&seams)?;
    let (a, b) = (grid.at(0, 0)?, grid.at(2, 0)?);
    let mut want = vec![(a, SeamNode::BoundaryEnd), (b, SeamNode::BoundaryEnd)];
    want.sort();
    assert_eq!(nodes(&graph), want);
    let [chain] = graph.chains() else {
        panic!("one chain: {:?}", graph.chains());
    };
    assert_eq!(chain.vertices()[1], grid.at(1, 0)?);
    Ok(())
}

#[test]
fn a_seam_round_the_whole_boundary_has_no_boundary_end() -> Result<(), Setup> {
    let grid = Grid::sheet(2, 1)?;
    let mut seams = SeamSet::new();
    let rim = [(0, 0), (1, 0), (2, 0), (2, 1), (1, 1), (0, 1), (0, 0)];
    grid.seam(&mut seams, SeamKind::Cut, &rim)?;
    let graph = grid.graph(&seams)?;
    assert_eq!(nodes(&graph), []);
    assert_eq!(graph.chains().len(), 1);
    Ok(())
}

#[test]
fn a_node_takes_the_first_class_that_fits() -> Result<(), Setup> {
    // (2, 0) meets three seams and a plain boundary edge: a junction. (1, 0) meets a Cut and an
    // Opening and a plain boundary edge: a boundary end, not a kind change.
    let grid = Grid::sheet(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(2, 1), (2, 0), (1, 0)])?;
    grid.seam(&mut seams, SeamKind::Cut, &[(2, 0), (3, 1)])?;
    grid.seam(&mut seams, SeamKind::Opening, &[(1, 0), (1, 1)])?;
    let graph = grid.graph(&seams)?;
    let found: BTreeMap<_, _> = graph.nodes().collect();
    assert_eq!(found.get(&grid.at(2, 0)?), Some(&SeamNode::Junction));
    assert_eq!(found.get(&grid.at(1, 0)?), Some(&SeamNode::BoundaryEnd));
    Ok(())
}

#[test]
fn a_ring_with_no_node_is_one_closed_chain_from_its_lowest_edge() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    let graph = grid.graph(&seams)?;
    assert_eq!(nodes(&graph), []);
    let [chain] = graph.chains() else {
        panic!("one chain: {:?}", graph.chains());
    };
    let (edges, vertices) = (chain.edges(), chain.vertices());
    assert_eq!(edges.len(), 4);
    assert_eq!(vertices[0], vertices[4]);
    // It starts on its lowest edge, walked from that edge's lower-numbered end.
    let lowest = edges.iter().min().copied();
    assert_eq!(edges.first().copied(), lowest);
    assert!(vertices[0] < vertices[1], "{chain:?}");
    Ok(())
}

#[test]
fn a_chain_from_a_junction_back_to_itself_starts_on_its_lower_end_edge() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &ROW_1)?;
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 1), (1, 0)])?;
    let graph = grid.graph(&seams)?;
    let junction = grid.at(1, 1)?;
    assert!(nodes(&graph).contains(&(junction, SeamNode::Junction)));
    let ring = graph
        .chains()
        .iter()
        .find(|c| c.edges().len() == 4)
        .ok_or("the ring is one chain")?;
    assert_eq!(ends(ring), (junction, junction));
    let edges = ring.edges();
    assert!(edges[0] < edges[3], "{ring:?}");
    Ok(())
}

#[test]
fn a_ring_with_an_opening_in_it_is_two_chains_between_kind_changes() -> Result<(), Setup> {
    let grid = Grid::tube(4, 2)?;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(0, 1), (1, 1), (2, 1), (3, 1)])?;
    grid.seam(&mut seams, SeamKind::Opening, &[(3, 1), (4, 1)])?;
    let graph = grid.graph(&seams)?;
    let mut want = vec![
        (grid.at(0, 1)?, SeamNode::KindChange),
        (grid.at(3, 1)?, SeamNode::KindChange),
    ];
    want.sort();
    assert_eq!(nodes(&graph), want);
    assert_eq!(graph.chains().len(), 2);
    Ok(())
}

#[test]
fn a_seam_on_an_edge_the_mesh_does_not_have_is_refused_with_the_lowest() -> Result<(), Setup> {
    let grid = Grid::sheet(2, 1)?;
    let count = grid.mesh.edge_count() as u32;
    let mut seams = SeamSet::new();
    grid.seam(&mut seams, SeamKind::Cut, &[(1, 0), (1, 1)])?;
    seams.insert(EdgeId::new(count + 7), SeamKind::Cut)?;
    seams.insert(EdgeId::new(count), SeamKind::Dart)?;
    assert_eq!(
        SeamGraph::new(&grid.mesh, &seams).err(),
        Some(SeamsError::UnknownEdge {
            edge: EdgeId::new(count)
        })
    );
    Ok(())
}
