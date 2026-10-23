# Architecture

<!-- index: the engine as built: the stage chain and its types, errors versus warnings, IDs and provenance, determinism and tolerances. -->

How the engine is put together, and why. Which parts are built is on the [status page](../start/status.md); parts not built yet are described in the specs this page links. For what each crate exports, see the [crate reference](../reference/crates/README.md); for the public API, see [reference/engine-api.md](../reference/engine-api.md).

## Two halves, one seam between them

OpenPlush turns a 3D model into a sewing pattern on the user's device. The work splits in two:

- **The engine** (`crates/`): pure Rust geometry. It never touches the filesystem, the clock, threads, the network, or a browser API, so it runs identically as a native CLI and as WebAssembly in a browser worker.
- **The shells** (`bindings/`, `apps/`, `packages/`): the CLI, the WASM facade, and the web app. They own all I/O and talk to the engine through one crate, `openplush-pipeline`. The CLI is described in [reference/cli.md](../reference/cli.md). Between the web app and the worker sits `packages/engine-client` ([reference/worker-protocol.md](../reference/worker-protocol.md)): it owns the worker and the request lifecycle, speaks only the protocol types generated from the facade's DTOs, and is the only package `apps/` may import for engine work.

The boundary is enforced, not just intended: `scripts/check-boundary.sh` reads `cargo metadata` and the source, and fails CI if an engine crate reaches a browser or OS API or if a binding depends on any workspace crate but the pipeline. `crates/clippy.toml` bans the listed std filesystem, clock, thread, network and process items at compile time, so an aliased `use std::time;` cannot sneak past the text scan.

Why: a geometry change should never need a UI change, and the user's model never leaves their machine. Rationale: [constitution, Principles I and VIII](https://github.com/openplush/openplush/blob/main/.specify/memory/constitution.md).

## The pipeline: each stage's type is its guarantee

The engine is a chain of stages. Each stage's output type promises what the next stage needs, so a later stage cannot be handed data that skipped an earlier check.

| Stage | Output type | It guarantees | Owner |
|---|---|---|---|
| Import | `TriangleSoup` (`openplush-mesh`) | Finite coordinates, in-range indices, a provenance record for every triangle corner. Says nothing about mesh quality: holes, duplicates and non-manifold edges pass through | `openplush-io`, run as `openplush-pipeline`'s `import` stage |
| Validate | `ValidationReport` (`openplush-mesh`) | Every defect of the soup located by face, vertex or vertex pair, judged as repair will see the mesh; changes nothing and never fails | `openplush-mesh`, run as `openplush-pipeline`'s `validate` stage |
| Repair | `ManifoldMesh` (`openplush-mesh`) | A consistently oriented 2-manifold, possibly with boundary, with no degenerate or zero-area face and no isolated vertex | `openplush-mesh`, run as `openplush-pipeline`'s `repair` stage |
| Seams and charts | `Vec<ChartMesh>` and the `SewingMap` (`openplush-seams`), from `openplush-pipeline`'s `chart` stage | Every chart a topological disk that does not fold back over itself, cut open along its seams; every seam chain sewn to its partner sides, the copies of an edge repair split read from the mesh (`ManifoldMesh::edge_copies`, so `repair → chart` needs nothing else); else the first violated invariant, typed | `openplush-seams`, run as `openplush-pipeline`'s `chart` stage |
| Flatten | A parameterization with every triangle unflipped (det J > 0) | Every seam edge keeps its exact 3D length, because the boundary is pinned first and the interior solved inside it ([flattening](flattening.md)) | `openplush-flatten`, run chart by chart as `openplush-pipeline`'s `flatten` stage, or a step at a time by its `Pipeline` session, which keeps each chart's layout until a seam edit changes that chart |
| Pattern, layout, export | Pieces with matched seam lengths, pages, SVG/PDF bytes | Sides sewn together have equal lengths, every piece at real-world scale with its seam allowance; pages tile the pieces at true scale with labels, registration marks and a scale check | `openplush-pattern`, run as `openplush-pipeline`'s `size` stage and chart by chart as its `pattern` stage; `openplush-layout` and `openplush-export`, run as its `layout`, `export_svg` and `export_pdf` stages |

`ManifoldMesh` has **no public constructor**: repair will be its only producer, so unrepaired geometry cannot reach the flattener by construction. `ManifoldMesh::debug_validate` checks all five half-edge invariants and is what the property tests run against every mesh they build.

Every stage's pre- and postconditions, built or not, are in the [engine API contract](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/contracts/engine-api.md) and the [data model](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md).

## Errors and warnings are different channels

Every stage returns `StageResult<T>`, which is `Result<Staged<T>, EngineError>`:

- **`Err(EngineError)`**: a declared invariant is violated, and there is no output at all. A stage never returns partial or "fixed up" geometry, because a pattern that looks right and is wrong is only discovered after cutting fabric. Every error variant names *where* the problem is (edge, vertex, chart, piece, source vertex), so the UI can highlight it; the exceptions are `Parse`, located only by the file format, `UnknownFormat` and `MissingResource`, which name a file rather than an element, `Cancelled`, and the refusals `Mesh` and `Seams` carry about a whole soup or a whole proposal (no triangles, an unusable extent, an empty dart), which have no one element to point at.
- **`Ok(Staged { value, warnings })`**: the output is valid but may sew badly. A `Warning` cannot exist without a remedy: `Remedy` is either `Actionable` (a seam edit the engine can make, offered for preview) or `Advisory` (a material or sizing change it will not make). Warnings never block export.

Both are built (`openplush-pipeline`), and the `import`, `validate`, `repair`, `chart`, `flatten`, `size`, `pattern`, `layout`, `export_svg` and `export_pdf` stages return them; `import` raises `import-ignored` for the parts of a glTF or FBX file it does not apply, `flatten` raises `flatten-iteration-cap` for a chart whose flatten stopped at the iteration cap, and `size` raises `size-out-of-range`. Exact messages and ordering: [reference/engine-api.md](../reference/engine-api.md).

## IDs and provenance

Every element is named by a newtype ID, never a raw integer: `VertexId`, `EdgeId`, `HalfEdgeId`, `FaceId` and `ComponentId` (mesh), `ChartId` and `SeamChainId` (seams), `PieceId` (pattern), `ProposalId` (seams). Vertex, edge, half-edge and face IDs are array positions; component, chart and seam-chain IDs are numbered by their lowest member (a chart by its lowest face), so they are deterministic without a hash map.

Mesh IDs (vertex, edge, face, component) are stable for one repaired mesh; chart, seam-chain and piece IDs only for one seam set, so a seam edit can renumber them. What survives re-import and re-repair is **provenance**: each triangle corner carries a `SourceCorner` (which drawn primitive, which vertex in it) from the source file. Saved projects will refer to geometry by provenance keys, not by ID; the keys exist (`openplush-project`'s `VertexKey`, `EdgeKey` and `FaceKey`), and the document that stores them does not yet. An edge is named by its two ends' lowest source vertices, and a face by its three corners'. A hole patch's own vertices, which have no source, are named by their loop and their place in its patch. An edge key resolves to the edges between its two ends, and a face key to the faces with exactly its corners, so a change of repair settings does not move a seam, and a key resolves to nothing when the mesh no longer has it.

A `ProposalId` is different again: a hash of the edit itself (the kind of fix, which edges, which seam kinds), so the same fix proposed twice has the same ID even after unrelated changes renumber the charts.

## Determinism and tolerances

Same input, same output, to the byte, on every platform:

- Ordered containers and total-order sorts on every output path, and plain IEEE-754 arithmetic (the rules: [AGENTS.md](https://github.com/openplush/openplush/blob/main/AGENTS.md), Determinism), because one reordered seam changes the printed pattern.
- Geometric predicates (`orient2d`, `orient3d`, `incircle`) are exact (`robust` crate, [ADR 0002](../adr/0002-robust-predicates.md)), so a near-degenerate triangle gets the same answer everywhere.
- Tolerances come from `openplush_geom::epsilon` only, and come in two kinds kept apart by type:
  - **Model-space** (`ModelTolerances`): relative to the model's bounding-box diagonal, so the same shape at any size makes the same decisions.
  - **Physical** (`SeamParity`, `FOLD_ANGLE`, `DELAUNAY_COT`): millimetres after scaling, because a sewist's millimetre does not depend on the model's units, and angles and their cotangents, which do not scale at all. The exact allowance is in `SeamParity`'s rustdoc; the fold angle and its reason in `FOLD_ANGLE`'s.

Model units become millimetres through `Scale::to_mm`. `Mm::try_new` is public, so another route is possible, but the type makes it visible.

## Crate layering

Crates depend downward only; the graph and its rules are in the [plan's crate DAG](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/plan.md#crate-dependency-dag-enforced), and `scripts/check-boundary.sh` enforces it. Which crates have code and which are scaffolds: the [crate reference](../reference/crates/README.md).
