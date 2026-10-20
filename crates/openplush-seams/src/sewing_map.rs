//! The sewing map: for each seam chain, the chart boundaries it runs along and which way each runs,
//! with the chains of a split edge's copies joined into one bundle.

use std::collections::BTreeMap;

use openplush_mesh::{EdgeId, HalfEdgeId, ManifoldMesh};

use crate::{ChartId, SeamChain, SeamChainId, SeamGraph, SeamSet, SeamsError, charts};

/// One side of a seam bundle: a chart's half-edges along it, one per edge of the lead chain in its
/// walking order, each on the chart's side of its edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainSide {
    chart: ChartId,
    half_edges: Vec<HalfEdgeId>,
}

impl ChainSide {
    /// The chart this side belongs to.
    pub fn chart(&self) -> ChartId {
        self.chart
    }

    /// Its half-edges, one per lead-chain edge in walking order, each running with the walk: the
    /// lead chain's own first side leaves each chain vertex along the walk, and a joined copy's
    /// half-edges start and end at the same positions.
    pub fn half_edges(&self) -> &[HalfEdgeId] {
        &self.half_edges
    }
}

/// A seam chain, with the chains of its copies, and every side sewn along it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamBundle {
    chain: SeamChainId,
    chains: Vec<SeamChainId>,
    sides: Vec<ChainSide>,
}

impl SeamBundle {
    /// The lead chain: the lowest of its chains, whose walk every side follows.
    pub fn chain(&self) -> SeamChainId {
        self.chain
    }

    /// Every chain in the bundle, ascending: the lead and the chains of its copies.
    pub fn chains(&self) -> &[SeamChainId] {
        &self.chains
    }

    /// Its sides, the lead chain's first (running with the walk, then its twins), then each
    /// copy's in chain order: two per chain between faces, one per chain on the boundary.
    pub fn sides(&self) -> &[ChainSide] {
        &self.sides
    }
}

/// Every chain's bundle, in lead-chain order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SewingMap {
    bundles: Vec<SeamBundle>,
}

/// The seam edges of each chain mapped to the chain's index, and the seam copies of each edge.
struct Lookup {
    chain_of: BTreeMap<EdgeId, usize>,
    partners: BTreeMap<EdgeId, Vec<EdgeId>>,
}

impl SewingMap {
    /// The map of `graph`'s chains over the charts of `seams`, with `groups` repair's copy groups
    /// (`RepairReport::edge_copies`); `graph` is `SeamGraph::with_copies(mesh, seams, groups)`,
    /// so two copies never share a chain.
    ///
    /// A chain between faces has two sides, one per face across its edges; the faces on one side
    /// of a chain are in one chart, since no seam crosses the fan at a chain's inner vertices. The
    /// chains of the seam copies in a group join the lowest one's bundle, so a fin has three sides
    /// and a sandwiched edge four.
    ///
    /// # Errors
    /// `UnpairedBoundarySeam` for a chain along mesh-boundary edges with no seam copy to sew to,
    /// naming the lowest such chain; `CopyGroupMismatch`, naming the lead chain, when a copy's
    /// chain is not the same length, does not run end to end with it, or is already joined
    /// elsewhere; `UnknownEdge` as `charts`.
    pub fn new(
        mesh: &ManifoldMesh,
        seams: &SeamSet,
        graph: &SeamGraph,
        groups: &[Vec<EdgeId>],
    ) -> Result<Self, SeamsError> {
        let mut chart_of = vec![ChartId::new(0); mesh.face_count()];
        for chart in charts(mesh, seams)? {
            for f in chart.faces() {
                chart_of[f.index() as usize] = chart.id();
            }
        }
        let lookup = lookup(seams, graph, groups);
        let chains = graph.chains();
        let mut joined = vec![false; chains.len()];
        let mut bundles = Vec::new();
        for (i, lead) in chains.iter().enumerate() {
            if joined[i] {
                continue;
            }
            joined[i] = true;
            let walk = along(mesh, lead);
            let mut sides = sides_of(mesh, &chart_of, &walk);
            let mut ids = vec![lead.id()];
            for (j, copy) in copies_of(mesh, chains, &lookup, lead, &walk)? {
                if std::mem::replace(&mut joined[j], true) {
                    return Err(SeamsError::CopyGroupMismatch { chain: lead.id() });
                }
                ids.push(chains[j].id());
                sides.extend(sides_of(mesh, &chart_of, &copy));
            }
            if ids.len() == 1 && mesh.face(walk[0]).is_none()
                || mesh.face(walk[0].twin()).is_none() && ids.len() == 1
            {
                return Err(SeamsError::UnpairedBoundarySeam { chain: lead.id() });
            }
            ids.sort();
            bundles.push(SeamBundle {
                chain: lead.id(),
                chains: ids,
                sides,
            });
        }
        Ok(Self { bundles })
    }

    /// Every bundle, in ascending lead-chain order.
    pub fn bundles(&self) -> &[SeamBundle] {
        &self.bundles
    }

    /// The bundle `chain` is in, as its lead or as a joined copy, if it is one of this map's.
    pub fn bundle(&self, chain: SeamChainId) -> Option<&SeamBundle> {
        self.bundles
            .iter()
            .find(|b| b.chains.binary_search(&chain).is_ok())
    }
}

/// `chain`'s half-edges in walking order, each leaving the chain vertex before it.
fn along(mesh: &ManifoldMesh, chain: &SeamChain) -> Vec<HalfEdgeId> {
    chain
        .edges()
        .iter()
        .zip(chain.vertices())
        .map(|(&e, &from)| {
            let h = e.half_edge();
            if mesh.origin(h) == from { h } else { h.twin() }
        })
        .collect()
}

/// The sides along `walk`: the half-edges themselves when they have a face, then their twins
/// when those do. A chain is wholly interior or wholly on the boundary, so the first decides.
fn sides_of(mesh: &ManifoldMesh, chart_of: &[ChartId], walk: &[HalfEdgeId]) -> Vec<ChainSide> {
    let twins: Vec<HalfEdgeId> = walk.iter().map(|h| h.twin()).collect();
    [walk.to_vec(), twins]
        .into_iter()
        .filter_map(|half_edges| {
            let f = mesh.face(half_edges[0])?;
            Some(ChainSide {
                chart: chart_of[f.index() as usize],
                half_edges,
            })
        })
        .collect()
}

/// Which chain each seam edge is in, and each seam edge's seam copies from `groups`.
fn lookup(seams: &SeamSet, graph: &SeamGraph, groups: &[Vec<EdgeId>]) -> Lookup {
    let chain_of = graph
        .chains()
        .iter()
        .enumerate()
        .flat_map(|(i, c)| c.edges().iter().map(move |&e| (e, i)))
        .collect();
    let mut partners: BTreeMap<EdgeId, Vec<EdgeId>> = BTreeMap::new();
    for group in groups {
        let on_seams: Vec<EdgeId> = group
            .iter()
            .copied()
            .filter(|&e| seams.get(e).is_some())
            .collect();
        for &e in &on_seams {
            let others = on_seams.iter().copied().filter(|&o| o != e).collect();
            partners.insert(e, others);
        }
    }
    Lookup { chain_of, partners }
}

/// The chains of `lead`'s copies, each as the chain's index and its half-edges running with
/// `walk`: one copy edge per lead edge, all in one chain of the lead's length, each copy half-edge
/// starting and ending where the lead's does.
fn copies_of(
    mesh: &ManifoldMesh,
    chains: &[SeamChain],
    lookup: &Lookup,
    lead: &SeamChain,
    walk: &[HalfEdgeId],
) -> Result<Vec<(usize, Vec<HalfEdgeId>)>, SeamsError> {
    let mismatch = || SeamsError::CopyGroupMismatch { chain: lead.id() };
    let mut by_chain: BTreeMap<usize, Vec<HalfEdgeId>> = BTreeMap::new();
    for (i, &h) in walk.iter().enumerate() {
        let partners = lookup
            .partners
            .get(&h.edge())
            .map_or(&[][..], Vec::as_slice);
        for &p in partners {
            let j = *lookup.chain_of.get(&p).ok_or_else(mismatch)?;
            let side = by_chain.entry(j).or_default();
            if side.len() != i {
                return Err(mismatch());
            }
            side.push(with(mesh, p, h).ok_or_else(mismatch)?);
        }
    }
    let n = walk.len();
    for (&j, side) in &by_chain {
        if side.len() != n || chains[j].edges().len() != n || chains[j].id() == lead.id() {
            return Err(mismatch());
        }
    }
    Ok(by_chain.into_iter().collect())
}

/// The half-edge of `copy` that starts and ends where `h` does, by position.
fn with(mesh: &ManifoldMesh, copy: EdgeId, h: HalfEdgeId) -> Option<HalfEdgeId> {
    let at = |g: HalfEdgeId| (mesh.position(mesh.origin(g)), mesh.position(mesh.dest(g)));
    let want = at(h);
    [copy.half_edge(), copy.half_edge().twin()]
        .into_iter()
        .find(|&g| at(g) == want)
}
