# Decisions needed

Open questions for the maintainer, found while working feature 001. Each item: what, the options, a recommendation, and what it blocks. Nothing here is implemented; work continues around each item. Delete an item when it is decided (the decision goes in the plan's Decision Ledger or an ADR).

Last updated: 2026-10-14.

## 1. Where the ID newtype macro lives

**What.** Seven ID newtypes (`VertexId`, `FaceId`, `HalfEdgeId`, `EdgeId`, `ComponentId` in `openplush-mesh`; `ChartId`, `SeamChainId` in `openplush-seams`; `PieceId` in `openplush-pattern`) repeat the same `new`/`index` boilerplate. AGENTS.md says the third copy becomes a macro. A shared macro has to live in a crate every owner depends on, so placing it is a crate-graph decision.

**Options.**
- **A. `openplush-geom` exports `id_newtype!`.** Every engine crate already depends on `geom` (the bottom of the DAG). Cost: `geom` gains a responsibility (IDs) that isn't geometry.
- **B. A new leaf crate `openplush-ids`** holding only the macro. Cleanest single responsibility; costs a crate, a DAG row, and an `architecture.md` edge.
- **C. One macro per owning crate** (`mesh` and `seams` each get a `macro_rules!`). No graph change, but two copies of the macro, which DRY forbids.

**Recommendation.** A, with the macro in `openplush_geom::id` and a one-line note in `crates/AGENTS.md`'s crate table ("…and the ID newtype macro"). The macro is ~20 lines with no dependencies; a crate for it (B) is ceremony.

**Blocks.** Nothing yet. Becomes pressing when the next ID type lands (Phase 3).

## 2. Splitting the three over-long functions in `openplush-mesh/src/halfedge/`

**What.** `ManifoldMesh::from_oriented_triangles` (`build.rs`), `ManifoldMesh::debug_validate` (`validate.rs`), and the test `invariant_3_a_bowtie_with_one_linked_boundary_loop` (`tests.rs`) carry `#[expect(clippy::too_many_lines)]`. Splitting them is a pure refactor with no task ID.

**Options.**
- **A. Add a refactor task** (e.g. under Phase 2's "Geometry and mesh core") and split each one, one helper per check/step, behaviour unchanged, in a refactor-only PR.
- **B. Leave them** until repair (T060+) next edits `build.rs`, and split then as a separate refactor commit/PR ahead of the behaviour change.

**Recommendation.** A, before T060 starts: the weld/repair tasks will call `from_oriented_triangles` and a reviewer of those PRs should not also be reading a 100-line function. A refactor-only PR is easy to review against the existing proptests (T027).

**Blocks.** Nothing today; makes T060-series reviews harder if left.
