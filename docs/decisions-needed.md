# Decisions needed

Open questions for the maintainer, found while working feature 001. Each item gives what the question is, the options, a recommendation, and what it blocks. Delete an item once it is decided; the decision goes in the plan's Decision Ledger or an ADR.

## 1. Where the ID newtype macro lives

**What.** Eight ID newtypes repeat the same `new`/`index` boilerplate:
- `VertexId`, `FaceId`, `HalfEdgeId`, `EdgeId` and `ComponentId` in `openplush-mesh`;
- `ChartId` and `SeamChainId` in `openplush-seams`;
- `PieceId` in `openplush-pattern`.

AGENTS.md says the third copy becomes a macro, so the rule is already overdue. The macro has to live in a crate that every owner depends on, which makes placing it a crate-graph decision. Today's edges are `mesh → geom` and `seams → mesh`; `pattern` depends on no workspace crate.

**Options.**
- **A. `openplush-mesh` exports `id_newtype!`.** It already owns five of the eight types, and `seams` already depends on it. One new edge, `pattern → mesh`. That edge points downward in the plan's DAG (`pattern → flatten → seams → mesh`), so the boundary check allows it with no ADR. `pattern` will depend on mesh types anyway once pieces reference faces.
- **B. `openplush-geom` exports it.** Two new edges, `seams → geom` and `pattern → geom`, both downward. Cost: `geom` (vectors, predicates, tolerances, units) takes on an unrelated responsibility.
- **C. A new leaf crate `openplush-ids`.** Cleanest single responsibility. Costs a crate, a DAG row in plan.md and `scripts/check-boundary.sh`, three new edges, and an `architecture.md` update.
- **D. A copy of the macro in each owning crate.** No graph change, but three copies of the macro, which is what DRY forbids.

**Recommendation.** A. It adds the fewest edges, and only one of them is not already implied by the planned graph. The macro is about 20 lines with no dependencies. Add "…and the ID newtype macro" to `openplush-mesh`'s row in `crates/AGENTS.md`.

**Blocks.** Nothing functionally. The longer it waits, the more copies there are to collapse.

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

## 5. No JS license gate exists, so the first npm dependency can't be checked

**What.** AGENTS.md says every dependency must pass "`cargo deny` / the JS license check". There is no JS license check: `scripts/check.sh shell` only installs, typechecks and tests, and no task in tasks.md creates one. T040 (`packages/engine-client` and its mock worker) would add the repo's first npm dependencies, `typescript` and `vitest`, plus their transitive tree, and nothing would check them against Constitution IX.

**Options.**
- **A. Add a task for a JS license gate and land it before T040.** `pnpm licenses list --json` (built into the pinned pnpm 10) piped through `jq` against the same allowlist as `deny.toml`, run in the `shell` step, with a self-test in `scripts/tests/` like the other gates. No new dependency.
- **B. Use a license-checker package** (e.g. `license-checker-rseidelsohn`). That is a dependency whose own tree must also pass the check.
- **C. Land T040 now** and add the gate later. This breaks the Licenses rule for as long as the gap lasts.

**Recommendation.** A. It is about 40 lines plus a self-test and needs no new tooling. Adding a gate is not a trivial-lane change, so it needs a task ID from you; something like "T176 JS license gate (blocks T040)".

**Blocks.** T040, and every later `apps/` or `packages/` task.

## 6. Post-spike spec sweep

**What.** The spikes (ADRs 0001 and 0005–0008, all Proposed) leave spec text that only changes once you accept them, or that touches a contract or another feature:
- data-model §7: the corner rule (miter to the fold line, bevel below about 29°, merge short edges instead of "simplify first"). R2's `i_overlay` row. Both follow ADR 0006.
- `contracts/project.schema.json`: `repair.voxelRemesh` contradicts D-024 (no remeshing in v0.1).
- 002's quickstart and tasks.md use `.glb` fixtures and call `sphere.glb` "shared with 001". 001 ships `.obj` (#71).
- 001 quickstart lists `quadruped`, `teddy` and `golden/`, none of which exist yet. The contract's CLI example uses `model.glb`.

**Options.**
- **A.** Accept or amend ADRs 0001 and 0005–0008, then one spec-only PR updates all of the above. The schema field is marked `reserved` with a note, rather than removed.
- **B.** Fold each item into the implementation task that next touches it: §7 with T085, the schema with T130, 002's fixtures with 002's first task.

**Recommendation.** A for the ADR-dependent items, once you've read the ADRs, because stale spec text is how the wrong rule gets built. B for 002, since it is a separate feature.

**Blocks.** Nothing now. T085 must not start from §7's current text.
