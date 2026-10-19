//! The sewing map: for each seam chain, the chart boundaries it runs along and which way each runs.

use openplush_mesh::{HalfEdgeId, ManifoldMesh};

use crate::{ChartId, SeamChainId, SeamGraph, SeamSet, SeamsError, charts};

/// One side of a seam chain: a chart's half-edges along it, one per chain edge in the chain's
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

    /// Its half-edges, one per chain edge in walking order; the first side's leave each chain
    /// vertex along the walk, the second side's are their twins.
    pub fn half_edges(&self) -> &[HalfEdgeId] {
        &self.half_edges
    }
}

/// A seam chain with every side that is sewn along it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamBundle {
    chain: SeamChainId,
    sides: Vec<ChainSide>,
}

impl SeamBundle {
    /// The chain.
    pub fn chain(&self) -> SeamChainId {
        self.chain
    }

    /// Its sides: two for a chain between faces, the one running with the walk first.
    pub fn sides(&self) -> &[ChainSide] {
        &self.sides
    }
}

/// Every chain's bundle, in chain order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SewingMap {
    bundles: Vec<SeamBundle>,
}

impl SewingMap {
    /// The map of `graph`'s chains, `SeamGraph::new(mesh, seams)`, over the charts of `seams`.
    ///
    /// A chain between faces has two sides, one per face across its edges; the faces on one side
    /// of a chain are in one chart, since no seam crosses the fan at a chain's inner vertices.
    ///
    /// # Errors
    /// `UnpairedBoundarySeam` for a chain along mesh-boundary edges, which has one side and no
    /// record of its partner, naming the lowest such chain; `UnknownEdge` as `charts`.
    pub fn new(
        mesh: &ManifoldMesh,
        seams: &SeamSet,
        graph: &SeamGraph,
    ) -> Result<Self, SeamsError> {
        let mut chart_of = vec![ChartId::new(0); mesh.face_count()];
        for chart in charts(mesh, seams)? {
            for f in chart.faces() {
                chart_of[f.index() as usize] = chart.id();
            }
        }
        let bundles = graph
            .chains()
            .iter()
            .map(|chain| {
                let along: Vec<HalfEdgeId> = chain
                    .edges()
                    .iter()
                    .zip(chain.vertices())
                    .map(|(&e, &from)| {
                        let h = e.half_edge();
                        if mesh.origin(h) == from { h } else { h.twin() }
                    })
                    .collect();
                let against: Vec<HalfEdgeId> = along.iter().map(|h| h.twin()).collect();
                let side = |half_edges: Vec<HalfEdgeId>| {
                    let f = mesh
                        .face(half_edges[0])
                        .ok_or(SeamsError::UnpairedBoundarySeam { chain: chain.id() })?;
                    // `charts` covers every face, so every entry was set.
                    let chart = chart_of[f.index() as usize];
                    Ok(ChainSide { chart, half_edges })
                };
                Ok(SeamBundle {
                    chain: chain.id(),
                    sides: vec![side(along)?, side(against)?],
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { bundles })
    }

    /// Every bundle, in ascending chain order.
    pub fn bundles(&self) -> &[SeamBundle] {
        &self.bundles
    }

    /// `chain`'s bundle, if it is one of this map's.
    pub fn bundle(&self, chain: SeamChainId) -> Option<&SeamBundle> {
        self.bundles
            .binary_search_by_key(&chain, SeamBundle::chain)
            .ok()
            .map(|i| &self.bundles[i])
    }
}
