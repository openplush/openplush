//! The seam graph: the vertices where seam chains must end, and the chains between them.

use std::collections::BTreeMap;

use openplush_mesh::{EdgeId, HalfEdgeId, ManifoldMesh, VertexId};

use crate::{SeamChainId, SeamKind, SeamSet, SeamsError};

/// Why a seam chain ends at a vertex. A vertex takes the first class that fits, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeamNode {
    /// Three or more seam edges meet.
    Junction,
    /// A seam meets a hole or opening: a mesh-boundary edge with no seam touches the vertex.
    BoundaryEnd,
    /// A seam ends inside the mesh.
    DartTip,
    /// Two seam edges of different kinds meet, as where an opening starts along a seam.
    KindChange,
}

/// A maximal path of seam edges between nodes, or a closed loop with no node on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamChain {
    id: SeamChainId,
    kind: SeamKind,
    edges: Vec<EdgeId>,
    vertices: Vec<VertexId>,
}

impl SeamChain {
    /// Its lowest `EdgeId`.
    pub fn id(&self) -> SeamChainId {
        self.id
    }

    /// The kind every one of its edges has.
    pub fn kind(&self) -> SeamKind {
        self.kind
    }

    /// Its edges in walking order.
    pub fn edges(&self) -> &[EdgeId] {
        &self.edges
    }

    /// Its vertices in walking order, one more than its edges: edge `i` joins vertex `i` and
    /// `i + 1`. A chain that returns to where it started lists that vertex at both ends.
    pub fn vertices(&self) -> &[VertexId] {
        &self.vertices
    }
}

/// Every seam vertex that ends a chain, and every chain, for one mesh and seam set.
///
/// The same seam set gives the same chains in the same order. An open chain with two different
/// ends runs from the lower-numbered one; one that starts and ends at the same node leaves it
/// along the lower-numbered of its two end edges; a closed chain starts on its lowest edge,
/// walked from that edge's lower-numbered end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeamGraph {
    nodes: BTreeMap<VertexId, SeamNode>,
    chains: Vec<SeamChain>,
}

impl SeamGraph {
    /// The graph of `seams` on `mesh`.
    ///
    /// # Errors
    /// `UnknownEdge`, naming the lowest, when a seam is on an edge `mesh` does not have.
    pub fn new(mesh: &ManifoldMesh, seams: &SeamSet) -> Result<Self, SeamsError> {
        if let Some((edge, _)) = seams
            .iter()
            .find(|(e, _)| e.index() as usize >= mesh.edge_count())
        {
            return Err(SeamsError::UnknownEdge { edge });
        }
        let stops = stops(mesh, seams);
        let mut walk = Walk {
            mesh,
            stops: &stops,
            taken: vec![false; mesh.edge_count()],
        };
        let mut chains = Vec::new();
        for (&v, stop) in &stops {
            let Stop::Node(_) = stop else { continue };
            for (h, kind) in star(mesh, seams, v) {
                if !walk.taken(h) {
                    chains.push(normalized(walk.chain(h, kind)));
                }
            }
        }
        // What is left are loops with no node on them.
        for (edge, kind) in seams.iter() {
            let h = edge.half_edge();
            if !walk.taken(h) {
                let h = if mesh.origin(h) < mesh.dest(h) {
                    h
                } else {
                    h.twin()
                };
                chains.push(walk.chain(h, kind));
            }
        }
        chains.sort_by_key(SeamChain::id);
        let nodes = stops
            .into_iter()
            .filter_map(|(v, stop)| match stop {
                Stop::Node(node) => Some((v, node)),
                Stop::Pass(_) => None,
            })
            .collect();
        Ok(Self { nodes, chains })
    }

    /// Every node, in ascending vertex order.
    pub fn nodes(&self) -> impl Iterator<Item = (VertexId, SeamNode)> + '_ {
        self.nodes.iter().map(|(&v, &node)| (v, node))
    }

    /// Every chain, in ascending ID order.
    pub fn chains(&self) -> &[SeamChain] {
        &self.chains
    }
}

/// What a walk along seams does on reaching a vertex: stop at a node, or carry on along the
/// other of the vertex's two seam edges, both leaving it.
#[derive(Debug, Clone, Copy)]
enum Stop {
    Node(SeamNode),
    Pass([HalfEdgeId; 2]),
}

/// Every vertex at the end of a seam edge, classified.
fn stops(mesh: &ManifoldMesh, seams: &SeamSet) -> BTreeMap<VertexId, Stop> {
    seams
        .iter()
        .flat_map(|(e, _)| {
            let h = e.half_edge();
            [mesh.origin(h), mesh.dest(h)]
        })
        .map(|v| (v, classify(mesh, seams, v)))
        .collect()
}

/// The seam half-edges leaving `v`, each with its kind.
fn star(mesh: &ManifoldMesh, seams: &SeamSet, v: VertexId) -> Vec<(HalfEdgeId, SeamKind)> {
    mesh.outgoing(v)
        .filter_map(|h| Some((h, seams.get(h.edge())?)))
        .collect()
}

fn classify(mesh: &ManifoldMesh, seams: &SeamSet, v: VertexId) -> Stop {
    let star = star(mesh, seams, v);
    if star.len() >= 3 {
        return Stop::Node(SeamNode::Junction);
    }
    let on_boundary = |h: HalfEdgeId| mesh.face(h).is_none() || mesh.face(h.twin()).is_none();
    if mesh
        .outgoing(v)
        .any(|h| on_boundary(h) && seams.get(h.edge()).is_none())
    {
        return Stop::Node(SeamNode::BoundaryEnd);
    }
    match star[..] {
        [(a, ka), (b, kb)] if ka == kb => Stop::Pass([a, b]),
        [_, _] => Stop::Node(SeamNode::KindChange),
        // `v` ends a seam edge, so this is one edge.
        _ => Stop::Node(SeamNode::DartTip),
    }
}

/// Walks chains, marking each edge it takes.
struct Walk<'a> {
    mesh: &'a ManifoldMesh,
    stops: &'a BTreeMap<VertexId, Stop>,
    taken: Vec<bool>,
}

impl Walk<'_> {
    fn taken(&self, h: HalfEdgeId) -> bool {
        self.taken[h.edge().index() as usize]
    }

    /// The chain that leaves along `first`, up to the next node or back to where it started.
    fn chain(&mut self, first: HalfEdgeId, kind: SeamKind) -> SeamChain {
        let start = self.mesh.origin(first);
        let (mut edges, mut vertices) = (Vec::new(), vec![start]);
        let mut h = first;
        loop {
            self.taken[h.edge().index() as usize] = true;
            edges.push(h.edge());
            let v = self.mesh.dest(h);
            vertices.push(v);
            // INVARIANT: `stops` classifies both ends of every seam edge.
            match self.stops[&v] {
                Stop::Pass([a, b]) if v != start => h = if a.edge() == h.edge() { b } else { a },
                _ => break,
            }
        }
        let lowest = edges.iter().fold(first.edge(), |low, &e| low.min(e));
        SeamChain {
            id: SeamChainId::new(lowest.index()),
            kind,
            edges,
            vertices,
        }
    }
}

/// A chain between nodes, turned to run from its lower-numbered end, or, when both ends are
/// the same node, to leave it along the lower-numbered end edge.
fn normalized(mut chain: SeamChain) -> SeamChain {
    let (first, last) = (chain.vertices[0], chain.vertices[chain.vertices.len() - 1]);
    let backwards = if first == last {
        chain.edges.last() < chain.edges.first()
    } else {
        last < first
    };
    if backwards {
        chain.edges.reverse();
        chain.vertices.reverse();
    }
    chain
}
