# openplush-seams

<!-- index: seam sets, the seam graph, charts and their checks, the sewing map, darts and openings. -->

What `openplush-seams` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

- `Proposal` (built only by `Proposal::new`), `ProposalId`, `ProposalKind`, `SeamReason` (`Crease`, `SeamHint`, `SourceModel`, `SourceDart`)
- `SeamKind`, `ChartFailure`
- IDs `ChartId`, `SeamChainId`
- `SeamSet` (`new`, `get`, `insert`, `remove`, `iter`, `len`, `is_empty`; edge flags in ascending edge order, with the mandatory-seam rule in its rustdoc)
- `SeamsError`
- `uv_cuts` and `UvCuts` (`edges`, `material_boundaries`, `shared_edges`, `faces_without_uv`, `projection_points`, `proposal`, `darts`)

### Seam graph

`SeamGraph::new(mesh, seams)` and `SeamGraph::with_copies(mesh, seams, groups)` (`groups` repair's `edge_copies`; two seam copies of one split edge never share a chain), the seam graph: `nodes()` (each `SeamNode`: `Junction`, `BoundaryEnd`, `DartTip`, `CopyEnd` or `KindChange`, the first that fits, in vertex order) and `chains()` (each `SeamChain`'s `id`, `kind`, `edges` and `vertices` in walking order, in ID order; the walking rules in its rustdoc), refusing a seam on an edge the mesh does not have with `UnknownEdge`.

### Charts

`charts(mesh, seams)`, each `Chart` (`id`, its lowest face, and `faces` in ascending order) the faces reachable without crossing a seam, in ID order, so a dart or a seam that stops short slits a chart rather than splitting it; same `UnknownEdge` refusal.

### Disk check

`check_disk(mesh, seams, chart)` passes a chart that, cut open along its seams, is a disk (genus 0, one boundary loop), counting the cut-open surface without building it, and otherwise returns its `ChartFailure`: `PositiveGenus { genus }` whenever it has handles, else `Closed` with no boundary or `MultipleBoundaries { count }` with more than one, so a slit that reaches no boundary is a second loop.

### Chart meshes

`ChartMesh::new(mesh, seams, chart)`, the chart cut open along its seams as flat arrays shaped like `FlattenSession::new`'s input: `chart`, `xyz` (model-space positions), `faces` (each chart face's `FaceId`, ascending, with three local `VertexId`s in the face's corner order) and `vtx_to_global`; a mesh vertex is copied once per fan of chart faces between cuts, so a seam through the chart separates its sides and a dart tip stays one vertex, and local vertices are numbered in order of first appearance; a local corner's global half-edge is its face's half-edge at the same position.

### Closing a chart

`close_chart(mesh, seams, chart)`, `None` for a disk, else one `CloseChart` proposal (naming `check_disk`'s failure) of `Cut` edges that leaves the chart one disk: searches run on the chart cut open along its seams, over edges with a chart face on both sides, each costing its 3D length, ties to the lower ID; a closed chart is cut along the shortest path between its two vertices of largest angle defect; otherwise, starting from the boundary loop with the lowest half-edge, the shortest path from the loops joined so far to the nearest other loop is cut until one loop remains, and any handles left are cut by tree-cotree (a shortest-path tree from the boundary or the lowest vertex, a maximum spanning dual tree weighted by the loop each edge closes, and the loops of the edges left over); the result is checked to leave one disk, or refused with `ChartNotClosed`.

### Openings

`mark_opening(seams, chain)` turns every edge of a seam chain into an `Opening` (the stuffing opening) or, checking first in the chain's walking order, changes nothing and refuses a mandatory edge with `MandatorySeam` or an edge with no seam (a chain from an older graph) with `NotASeam`.

### Fold check

`check_fold(mesh, seams, chart)` passes a chart unless two of its faces share an edge no seam cuts with normals within `FOLD_ANGLE` of antiparallel (the surface lies back over itself, which no flattener can fix), else `FoldedChart` naming the chart and the lowest such edge; a fold along a seam is two pieces, not a fold, and a face with no area never folds (repair rejects it).

### Sewing map

`SewingMap::new(mesh, seams, graph, groups)`, one `SeamBundle` per lead chain of `graph` (`SeamGraph::with_copies(mesh, seams, groups)`, `groups` repair's `edge_copies`) in chain order, each with its `chain` (the lead), `chains` (the lead and the chains of the seam copies in its group, ascending) and `sides`, every `ChainSide` a `chart` and its `half_edges`, one per lead-chain edge in walking order: the lead's first side leaves each chain vertex along the walk, its second is the twins, then each copy chain's sides, running with the lead by position (a fin has three sides, a sandwiched edge four); `bundles()` and `bundle(chain)` (by lead or by any joined chain); a chain along mesh-boundary edges with no seam copy to sew to is refused with `UnpairedBoundarySeam` naming the lowest such chain, never a self-pairing, and a copy chain that is not the lead's length, does not run end to end with it, or is already joined is `CopyGroupMismatch` naming the lead.

### Shortest path

`shortest_path(mesh, from, to)`, the shortest edge walk between two picked vertices by 3D length, as its edges in walking order from `from` (empty for the same pick twice), the same walk every time among equal ones (ties to the lower vertex, then the lower edge); refuses a pick the mesh lacks with `UnknownVertex` and picks no edges join with `NoPath`.

### Darts

`mark_dart(mesh, seams, edges)` marks a walk of edges a `Dart`, or changes nothing: the base is whichever end is anchored (on a seam or the mesh boundary), either direction of drawing is accepted, and every other vertex of the walk must touch no seam or boundary, so the dart ends inside the piece as a slit and never splits it or crosses a seam. It refuses:

- no edges (`EmptyDart`)
- an edge the mesh lacks (`UnknownEdge`)
- an edge that does not continue the walk or revisits a vertex (`NotAWalk`, naming it)
- a walk anchored at neither end (`DartNotAnchored`, naming its first vertex as drawn, a single edge's lower end)
- a tip or inner vertex on a seam or boundary (`DartTouchesSeam`, naming the first)

### UV cuts and material boundaries

`uv_cuts(mesh, seams)` returns the source model's UV cuts and material boundaries, the cuts among them that stop partway, plus the projection points where its UVs look auto-projected, as `UvCuts`, and refuses a seam on an edge the mesh does not have with `UnknownEdge`:

- `edges()`, ascending: every interior edge not already in `seams` whose corner UVs on its two sides differ by more than `UV_CUT` in either coordinate at either endpoint (a NaN difference counts as a cut), tested only when both its faces have a UV at every corner, so a boundary edge or one beside a face without UVs is never a cut.
- `faces_without_uv()`: how many faces lack a UV at some corner.
- `material_boundaries()`, ascending: every interior edge not already in `seams` whose two faces both come from the file and differ in material (a face's material is its first corner's primitive's), a face with no material differing from one with a material; an edge beside a face repair added is never one.
- `shared_edges()`: how many edges are both a cut and a material boundary.
- `projection_points()`, ascending: every interior vertex with at least five faces round it, all with a UV at every corner, whose corner UVs there differ pairwise by the same rule, as an automatic projection leaves at a sphere's poles (and as five hand-marked seams meeting do too); they change no cut.
- `proposal()`: one `ProposalKind::Seam { reason: SeamReason::SourceModel }` proposal, each boundary a `FabricBoundary` edge (one that is also a cut included, added once) and every other cut a `Cut` edge, leaving out every edge of a dart below, or `None` when nothing is left.
- `darts()`: one `ProposalKind::Seam { reason: SeamReason::SourceDart }` proposal per cut that stops partway, every edge a `Dart`, in ascending order of its tip. A tip is a `DartTip` node of the seam graph over the cuts, the boundaries and the `Mandatory` seams in `seams`, so a cut that ends on another cut, a boundary, a mandatory seam or the surface boundary has none; any other seam already in `seams` does not rescue it. The dart is the chain from the tip to the first junction, the surface boundary or a mandatory seam; a slit free at both ends is one dart at its lower tip. Its explanation reads "The cut marked in the model that ends at vertex *n* stops partway across the surface, so it will be sewn as a dart. If that is not meant, decline it and fix the cut in the modelling tool.", or, for a slit, "between vertices *a* and *b* … at both ends".

## Depends on

`openplush-geom`, `openplush-mesh`, `libm`, `thiserror`
