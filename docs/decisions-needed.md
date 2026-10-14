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

## 3. How TypeScript protocol types are generated

**What.** T041 (`scripts/gen-ts-types.sh`) generates TypeScript types from the Rust protocol definitions, and CI fails on a diff. Neither plan.md nor research.md names a generator, and the brief says any dependency they don't name waits for the maintainer.

**Options.**
- **A. `ts-rs`** (MIT): a derive on each DTO, and `cargo test` exports `.ts` files. Mature and widely used [likely]. It needs `serde` on the DTOs to match the wire format, and `serde` is already planned (`openplush-project`).
- **B. `tsify`** (MIT OR Apache-2.0): a derive that ties into `wasm-bindgen`, so exported functions get typed signatures. Couples the types to the WASM build [likely].
- **C. `specta`** (MIT): language-agnostic export. Its API has churned between majors [verify].
- **D. A hand-written generator** reading a small JSON schema emitted from Rust. No dependency, but a second piece of code to keep correct.

**Recommendation.** A (`ts-rs`), with an ADR for the "code generation" dependency category. The DTOs live in `bindings/openplush-wasm`, so the dependency stays out of `crates/`. Generation runs in a native test, so the WASM build is unaffected.

**Blocks.** T041, and the typed half of T040 (the mock worker can use hand-typed placeholders, but they would be thrown away).

## 4. Panics at the WASM boundary

**What.** `crates/AGENTS.md` says "`openplush-wasm` wraps each export and converts panics to typed errors." On `wasm32-unknown-unknown`, stable Rust builds with `panic = "abort"`. A panic traps the instance, so `catch_unwind` cannot catch it [likely; verify on the pinned toolchain]. As written, the rule can't be implemented on stable.

**Options.**
- **A. Treat a trap as a worker crash.** `packages/engine-client` catches the worker's `error` event and answers the in-flight request with a binding-level code (e.g. `EngineCrashed`, next to `StaleHandle`), then restarts the worker. Handles already don't survive a restart (worker-protocol §4), so the replay path exists anyway. Reword the AGENTS.md line to match.
- **B. Nightly `-Zbuild-std` with `panic=unwind`** plus wasm exception handling, so `catch_unwind` works. Needs nightly, which conflicts with the pinned stable toolchain (Constitution, Technical Constraints).
- **C. Stay as written** and discover this at T039 implementation time.

**Recommendation.** A. It works on stable and fails loudly. A panic is already a bug (no `unwrap` on user data), so the shell only needs to report it and recover, not continue. This is a rule change in `crates/AGENTS.md` and a worker-protocol addition, so it needs your sign-off. A contract change may also need an ADR.

**Blocks.** The first real export in T039 (the skeleton has none), and T040's error handling.
