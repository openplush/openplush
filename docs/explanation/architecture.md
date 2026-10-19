# Architecture

How the engine is put together **as built today**, and why. Planned parts are marked *planned* and link to the spec that describes them; everything else exists on `main`. For what each crate exports, see [reference/crates.md](../reference/crates.md); for the public API, see [reference/engine-api.md](../reference/engine-api.md).

## Two halves, one seam between them

OpenPlush turns a 3D model into a sewing pattern on the user's device. The work splits in two:

- **The engine** (`crates/`): pure Rust geometry. It never touches the filesystem, the clock, threads, the network, or a browser API, so it runs identically as a native CLI and as WebAssembly in a browser worker.
- **The shells** (`bindings/`, `apps/`, `packages/`): the CLI, the WASM facade, and the web app. They own all I/O and talk to the engine through one crate, `openplush-pipeline`. The CLI ([reference/cli.md](../reference/cli.md)) and the WASM facade exist as skeletons that call no stage yet; the web app is *planned*. Between the web app and the worker sits `packages/engine-client` ([reference/worker-protocol.md](../reference/worker-protocol.md)): it owns the worker and the request lifecycle, speaks only the protocol types generated from the facade's DTOs, and is the only package `apps/` may import for engine work.

The boundary is enforced, not just intended: `scripts/check-boundary.sh` reads `cargo metadata` and the source, and fails CI if an engine crate reaches a browser or OS API or if a binding depends on any workspace crate but the pipeline. `crates/clippy.toml` bans the listed std filesystem, clock, thread, network and process items at compile time, so an aliased `use std::time;` cannot sneak past the text scan.

Why: a geometry change should never need a UI change, and the user's model never leaves their machine. Rationale: [constitution, Principles I and VIII](../../.specify/memory/constitution.md).

## The pipeline: each stage's type is its guarantee

The engine is a chain of stages. Each stage's output type promises what the next stage needs, so a later stage cannot be handed data that skipped an earlier check.

| Stage | Output type | It guarantees | Status |
|---|---|---|---|
| Import | `TriangleSoup` (`openplush-mesh`) | Finite coordinates, in-range indices, a provenance record for every triangle corner. Says nothing about mesh quality: holes, duplicates and non-manifold edges pass through | Built for OBJ (`openplush-io`); glTF, STL and PLY are planned |
| Validate | `ValidationReport` (`openplush-mesh`) | Every defect of the soup located by face, vertex or vertex pair, judged as repair will see the mesh; changes nothing and never fails | Built |
| Repair | `ManifoldMesh` (`openplush-mesh`) | A consistently oriented 2-manifold, possibly with boundary, with no degenerate or zero-area face and no isolated vertex | Type and its validator built; repair is planned |
| Seams and charts | Charts, each a topological disk | *planned* | Partly built: the seam graph, whose chains end at junctions, boundary ends, dart tips and kind changes (`openplush-seams`'s `SeamGraph`), charts as the face sets the seams separate (`charts`), the disk check each chart must pass to flatten (`check_disk`), each chart cut open along its seams as the flattener's flat arrays (`ChartMesh`), the cuts proposed to close a chart that is not a disk (`close_chart`), and marking a chain as the stuffing opening (`mark_opening`) |
| Flatten | A parameterization with every triangle unflipped (det J > 0) | *planned* | The unflipped check is built (`openplush-flatten`'s `injectivity::verify`); the solver is planned |
| Pattern, layout, export | Pieces with matched seam lengths, pages, SVG/PDF bytes | *planned* | Partly built: `Outline`, `PatternPiece`, the seam-side parity check and real-world scale (`openplush-pattern`), shelf packing, page tiling with labels and registration marks (`openplush-layout`), the cut line from per-edge seam allowances (`openplush-pattern`'s `allowance`, one boolean in `i_overlay`, ADR 0006), SVG export of one sheet, and tiled PDF export at true scale, every page with its label, registration crosshairs and a labelled scale-check square (`openplush-export`'s `svg` and `pdf`, which read layout's output, ADR 0012; PDF written directly with `pdf-writer`, ADR 0008), opening with a cover page that carries the scale check, a map of the tiles and the summary of unresolved warnings, which continues after the last tile when the cover runs out of room |

`ManifoldMesh` has **no public constructor**: repair will be its only producer, so unrepaired geometry cannot reach the flattener by construction. `ManifoldMesh::debug_validate` checks all five half-edge invariants and is what the property tests run against every mesh they build.

The planned stages and their pre- and postconditions are in the [engine API contract](../../specs/001-pattern-pipeline/contracts/engine-api.md) and the [data model](../../specs/001-pattern-pipeline/data-model.md).

## Errors and warnings are different channels

Every stage returns `StageResult<T>`, which is `Result<Staged<T>, EngineError>`:

- **`Err(EngineError)`**: a declared invariant is violated, and there is no output at all. A stage never returns partial or "fixed up" geometry, because a pattern that looks right and is wrong is only discovered after cutting fabric. Every error variant names *where* the problem is (edge, vertex, chart, piece, source vertex), so the UI can highlight it; the exceptions are `Parse`, located only by the file format, and `Cancelled`.
- **`Ok(Staged { value, warnings })`**: the output is valid but may sew badly. A `Warning` cannot exist without a remedy: `Remedy` is either `Actionable` (a seam edit the engine can make, offered for preview) or `Advisory` (a material or sizing change it will not make). Warnings never block export.

Both are built (`openplush-pipeline`); no stage produces them yet. Exact messages and ordering: [reference/engine-api.md](../reference/engine-api.md).

## IDs and provenance

Every element is named by a newtype ID, never a raw integer: `VertexId`, `EdgeId`, `HalfEdgeId`, `FaceId` and `ComponentId` (mesh), `ChartId` and `SeamChainId` (seams), `PieceId` (pattern), `ProposalId` (seams). Vertex, edge, half-edge and face IDs are array positions; component, chart and seam-chain IDs are numbered by their lowest member (a chart by its lowest face), so they are deterministic without a hash map.

Mesh IDs (vertex, edge, face, component) are stable for one repaired mesh; chart, seam-chain and piece IDs only for one seam set, so a seam edit can renumber them. What survives re-import and re-repair is **provenance**: each triangle corner carries a `SourceCorner` (which drawn primitive, which vertex in it) from the source file. Saved projects will refer to geometry by provenance, not by ID.

A `ProposalId` is different again: a hash of the edit itself (the kind of fix, which edges, which seam kinds), so the same fix proposed twice has the same ID even after unrelated changes renumber the charts.

## Determinism and tolerances

Same input, same output, to the byte, on every platform:

- Ordered containers and total-order sorts on every output path, and plain IEEE-754 arithmetic (the rules: [AGENTS.md](../../AGENTS.md), Determinism), because one reordered seam changes the printed pattern.
- Geometric predicates (`orient2d`, `orient3d`, `incircle`) are exact (`robust` crate, [ADR 0002](../adr/0002-robust-predicates.md)), so a near-degenerate triangle gets the same answer everywhere.
- Tolerances come from `openplush_geom::epsilon` only, and come in two kinds kept apart by type:
  - **Model-space** (`ModelTolerances`): relative to the model's bounding-box diagonal, so the same shape at any size makes the same decisions.
  - **Physical** (`SeamParity`): millimetres after scaling, because a sewist's millimetre does not depend on the model's units. The exact allowance is in `SeamParity`'s rustdoc.

Model units become millimetres through `Scale::to_mm`. `Mm::try_new` is public, so another route is possible, but the type makes it visible.

## Crate layering

Crates depend downward only; the graph and its rules are in the [plan's crate DAG](../../specs/001-pattern-pipeline/plan.md#crate-dependency-dag-enforced), and `scripts/check-boundary.sh` enforces it. Which crates have code and which are scaffolds: [reference/crates.md](../reference/crates.md).
