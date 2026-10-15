---
description: "Task list for 001 pattern pipeline"
---

# Tasks: 3D Model → Plush Sewing Pattern

**Input**: Design documents in `/specs/001-pattern-pipeline/`
**Prerequisites**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)
**Generated**: 2026-10-13, after spec Q1–Q3 closed

**Tests are mandatory here, not optional.** Constitution IV requires test-first with independent oracles, and `AGENTS.md` requires the failing test before the implementation. Every test task below must be written *and observed failing* before its implementation task starts. The tasks template calls tests optional; this project overrides that.

## Format: `[ID] [P?] [Story] Description`

- **[P]** — can run in parallel (different files, no dependency)
- **[Story]** — which user story it serves
- Paths are exact and follow [plan.md](./plan.md#project-structure)

## PR packaging

**A task is a unit of work, not a unit of review.** 179 tasks do not mean 179 PRs, and it certainly does not mean one PR per phase. Policy in [`AGENTS.md`](../../AGENTS.md#commits-and-prs), detail in [docs/how-to/pull-requests.md](../../docs/how-to/pull-requests.md). In short: 400 lines of hand-written source, one test task plus the implementation that satisfies it, one story and one crate per PR.

### Batch these — several tasks, one PR

Small config and scaffolding tasks sharing one concern:

| PR | Tasks |
|---|---|
| `chore/workspace-config` | T009, T010, T011, T012 |
| `chore/ci-engine` | T013 |
| `chore/boundary-check` | T016 — alone, see *Isolate these* |
| `chore/ci-wasm-shell` | T014, T015 |
| `chore/ci-determinism-golden` | T017, T018 (T042 landed alone as `chore/determinism-lint`: T017 and T018 have nothing to check until export, the CLI fixture runs and the first `insta` snapshot exist) |
| `chore/repo-governance` | T019, T020, T021 |
| `chore/clippy-boundary-lints` | T172 (landed as #33) |
| `chore/check-script` | T173 (landed as #34) |
| `001/25-T028-engine-error` (#57) | T028 |
| `001/26-T029-warning-remedy` (#58), stacked on 25 | T029, T030 |
| `001/27-T031-staged-result` (#59), stacked on 26 | T031 |
| `001/NN-T032-testkit` | T032, T033, T034 (landed as `001/33-T032-fixture-loader` and `001/34-T034-oracles`; T033 waits for the first SVG or metrics output to compare) |
| `001/NN-T050-obj-import` | T050, T053 |
| `001/NN-T055-validators` | T055, T056, T057 |

`NN` is the slice's position in feature 001, assigned when the PR is opened (the next unused number). It is not fixed here because it depends on what lands first. Setup tasks (Phase 1) are repo plumbing, so they are `chore/` branches, not 001 slices.

T022–T027 were planned as one batch and landed as one slice per task, because each was over budget on its own: `001/11-T022-geom-core` (#37), `001/12-T023-robust-predicates` (#38), `001/13-T024-triangle-soup` (#39), `001/14-T025-halfedge-mesh` (#40), `001/15-T026-debug-validate` (#41), `001/16-T027-invariant-proptests` (#42).

### Split these — one task, several PRs

Tasks that will exceed the budget on their own. Split by capability, each with its own test, and record the branch names back in this file so the task stays traceable:

| Task | Why it splits |
|---|---|
| T060 weld | Spatial hash, deterministic tie-break, and glTF split-vertex handling are three testable capabilities |
| T062 non-manifold edge cutting | Dihedral sorting, orientation pairing, and edge duplication |
| T066 hole filling | Classification is separate from Liepa triangulation and fairing |
| T073 chart construction | Flood fill, vertex duplication, and provenance mapping |
| T080 SLIM interior | Energy and line search may land before the convergence and iteration policy |
| T085 allowance strips | Strip construction, then corner joins, then the tight-curve union |
| T089 page tiling | Tiling geometry, then registration marks and page map, then the scale square |
| T105 dart suggestion | Four distinct steps, each independently checkable |

### Isolate these — own PR regardless of size

Not because they are large, but because their blast radius is:

| Task | Why |
|---|---|
| **T079** boundary prescription | D-013. S7 found no failure on the corpus (ADR 0005), but it is still the decision the whole pattern stage rests on |
| **T067** `into_manifold` typestate gate | The one thing preventing unrepaired geometry from reaching the flattener |
| **T087** parity assertion | The last line of defence against the failure this project exists to prevent |
| **T016** boundary check script | Constitution I's only enforcement; if it is wrong, the boundary is decorative |
| **T029** `Remedy` enum | Makes FR-065 a compile error instead of a review item; the shape must be right before anything depends on it |

### Never in one PR

T084's `Fabric` with any detector that consumes it. The spike tasks (T001–T008) with any implementation — a spike's output is an ADR and a decision, and bundling it hides the decision inside a diff.

---

## Scope note

**US4 (auto-seam layout) has no tasks here.** D-015 defers it past v0.1, and feature [002](../002-seam-placement-hints/spec.md) is its groundwork. FR-015, FR-016, and FR-043 are likewise out of v0.1 — see the plan's Phasing table. Nothing below implements them, and a task that appears to need them is a task that is wrong.

---

## Phase 0: Research Spikes (time-boxed, blocking)

**Purpose**: Eight questions in [research.md R6](./research.md) have load-bearing answers. Each either confirms an architectural choice or forces a change before code depends on it.

- [x] T001 [P] **S1** — Does `baby_shark` build and run on `wasm32-unknown-unknown`? Record in `docs/adr/0001-preprocessing.md`. If no: isotropic remeshing becomes native-only and WASM flattening runs on the raw mesh, which changes the conditioning the interior solve (T080) can assume. Box: 0.5 day *2026-10-14: answered in [ADR 0001](../../docs/adr/0001-preprocessing.md) (Proposed) and `research/s1_baby_shark_wasm/` (`001/30-T001-s1-baby-shark-wasm`). Builds and runs on wasm32, byte-identical to native, but 0.3.12 fails `cargo deny` (rayon) and the boundary check (wasm-bindgen, getrandom) with no features to opt out, and native-only would fail the same gates. Neither target remeshes; T080 conditions on the repaired mesh, and S2 must size charts at raw input*
- [ ] T002 [P] **S2** — SLIM plus `faer` on a 50k-triangle chart in WASM: wall time per iteration, native vs. browser. Feeds SC-005's 10 s budget. If the budget is missed, the remeshed-proxy size in the plan's Technical Context is the dial. Box: 2 days. *2026-10-14: there is no remeshed proxy (D-024, S1), so S2 measures charts at input size; a missed budget goes back to the maintainer instead of turning a dial*
- [x] T003 [P] **S3** — `i_overlay` offset robustness: fuzz 10k random valid outlines with mixed per-edge allowances; catalogue panics (upstream issue 74). Determines how defensive the allowance construction (T085) must be. Box: 1 day *2026-10-15: answered in [ADR 0006](../../docs/adr/0006-allowance-construction.md) (Proposed) and `research/s3_i_overlay_fuzz/` (#81). No panics and no wrong booleans on valid input; invalid input, sub-grid edges and chained booleans are silently wrong, so T085 validates input, merges short edges, makes one `i64` call with fold guards, checks the result into typed errors, and catches panics*
- [x] T004 [P] **S4** — `jagua-rs` WASM bundle size and discrete rotation-set support. A bundle-size surprise changes the v0.1 layout design even though nesting is v0.2. Box: 0.5 day *2026-10-15: answered in [ADR 0007](../../docs/adr/0007-layout-nesting-engine.md) (Proposed) and `research/s4_jagua_rs_wasm/` (#79). Rotation sets {0°}/{0°,180°} and mirror axes supported; bit-identical native vs wasm32; +364 KB raw / +135 KB gzip, so v0.1 layout design unchanged. 1.1.0 fails `cargo deny` and the boundary check until upstream feature-gates `rayon`/`getrandom`/`web-time`; `sparrow` is not adoptable as a dependency*
- [x] T005 [P] **S5** — `svg2pdf` under WASM vs. writing PDF directly. Decides the approach for T091. Box: 0.5 day *2026-10-15: answered in [ADR 0008](../../docs/adr/0008-pdf-export.md) (Proposed) and `research/s5_svg2pdf_wasm/` (#80). T091 writes the PDF directly with `pdf-writer` 0.15 and labels in standard-14 Helvetica with no embedded font; `svg2pdf` is not adopted (4/3 scale trap at its default dpi, silent text loss, no allowlisted font, +213 KB gzip). T091 returns a typed error for a label outside WinAnsi*
- [x] T006 [P] **S6** — Confirm licenses for `faer`, `i_overlay`, `sparrow`, `svg2pdf`, `threecrate`. Any non-allowlist result needs an ADR or a replacement (Constitution IX). Box: 0.5 day. **Done 2026-10-14: all pass; `sparrow` needs git-source + clarify entries before v0.2 — [research S6 result](./research.md#s6-result--licenses-2026-10-14--closed)**
- [x] T007 **S7** — **Boundary-pinned flattening (D-013).** On every fixture: how often do prescribed boundary lengths fail to close, and does the result stay injective? Measure the interior-distortion penalty against free-boundary SLIM. Parity is a hard contract, so a bad result changes how aggressive the fallback (T082) must be, never the contract. Highest-risk spike in the list: it tests the decision the whole pattern stage rests on. Box: 2 days. **Done 2026-10-15:** 36 runs (the 10 fixture charts, the same 10 jittered, 16 refined): 0 closure failures, 0 injectivity failures. Pinning costs nothing on developable charts and moves curvature into interior compression (hemisphere cap: 0.61 pinned vs 0.87 free, while the free solve drifts seam lengths 41–47%). T082 is a rare-path typed error; T105 thresholds on area-integrated compression — [research S7 result](./research.md#s7-result--boundary-pinned-flattening-2026-10-15--closed), [ADR 0005](../../docs/adr/0005-boundary-pinned-flattening.md). Branches `001/35-T007-s7-fixture-corpus` (#77, loader and cut), `001/38-T007-s7-fixture-run` (#78, run, measures, ADR)
- [ ] T008 [P] **S8** — Get the seven fabric preset value sets in research R7 reviewed by someone who sews with these fabrics, or replace them from supplier spec sheets. Every number is currently a guess. Box: 0.5 day plus outside review

**Checkpoint**: S7 and S6 must be answered before Phase 3. S1 through S5 and S8 may run alongside Phases 1 and 2.

---

## Phase 1: Setup (shared infrastructure)

**Purpose**: An empty but fully gated repository. A contributor can clone, build nothing, and still have every CI gate run.

- [x] T009 Create Cargo workspace: `Cargo.toml` with the workspace members from the plan tree, `rust-toolchain.toml` pinning MSRV, `.cargo/config.toml` — *merged in #26*
- [x] T010 [P] `deny.toml` — license allowlist exactly as Constitution IX states (MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0); deny GPL, AGPL, LGPL — *merged in #26*
- [x] T011 [P] `pnpm-workspace.yaml` and root `package.json` with the `apps/*` and `packages/*` globs — *merged in #26*
- [x] T012 [P] `rustfmt.toml` and clippy lint configuration; document the deny-warnings policy — *merged in #26*
- [x] T013 [P] `.github/workflows/engine.yml` — fmt, clippy with `-D warnings`, `cargo test --workspace`, `cargo deny check` — *merged in #27*
- [x] T014 [P] `.github/workflows/wasm.yml` — build `wasm32-unknown-unknown`, run `wasm-bindgen-test` — *merged in #29*
- [x] T015 [P] `.github/workflows/shell.yml` — typecheck, Vitest, Playwright — *merged in #29*
- [x] T016 `scripts/check-boundary.sh` — parse `cargo metadata`; fail if `crates/` depends on `wasm-bindgen`, `web-sys`, `js-sys`, or `std::fs`, or if anything in `bindings/` depends on a workspace crate other than `openplush-pipeline` (D-020). Wire into `engine.yml`. **This is Constitution I's enforcement; without it the boundary is a suggestion** — *merged in #28*
- [ ] T017 [P] `.github/workflows/determinism.yml` — run the fixture corpus native and WASM, compare export hashes (SC-008)
- [ ] T018 [P] `.github/workflows/golden.yml` — `insta` snapshot diff, PR comment with rendered SVG diffs
- [x] T019 [P] `.github/CODEOWNERS`, lane labels (`lane:engine`, `lane:shell`, `lane:fixtures`, `lane:docs`), issue forms — *merged in #36*
- [x] T020 [P] `docs/` Diátaxis skeleton (`tutorials/`, `how-to/`, `reference/`, `explanation/`) and `docs/adr/0000-template.md` in MADR format — *merged in #36*
- [x] T021 [P] `CODE_OF_CONDUCT.md` and `GOVERNANCE.md` — *merged in #36*
- [x] T172 `crates/clippy.toml` — compiler-resolved boundary layer behind T016: `disallowed-methods`/`disallowed-types` for `std::fs`, `std::thread::spawn`, `std::time::{Instant, SystemTime}`, `std::net`, `std::process`, so aliased forms (`use std::time;`, `use std as s;`) that T016's text scan cannot see fail clippy. Repeats the root `clippy.toml` entries (clippy reads only the nearest file); `openplush-testkit` keeps file access. Self-test proving each aliased form is rejected. *Added 2026-10-14 from PR review; ID out of phase order because T022+ were already numbered* — *merged in #33*
- [x] T173 `scripts/check.sh` — one entry point for every gate (`engine`, `wasm`, `shell`, or all), plus `--setup` to install `cargo-deny` and the lockfile-matched `wasm-bindgen-cli`; CI calls it so local and CI cannot drift. CONTRIBUTING documents three commands: clone, `scripts/check.sh --setup`, `scripts/check.sh` (Constitution IX: ≤ 3). *Added 2026-10-14 from PR review* — *merged in #34; CI workflows switched to it in #43*
- [ ] T176 `scripts/check-js-licenses.sh`, wired into `scripts/check.sh`'s `shell` step, with `scripts/tests/check-js-licenses.test.sh` — the JS half of Constitution IX's license gate: every installed npm package, dev dependencies included, must **declare** (in its own `package.json`) a license expression that `deny.toml`'s allowlist (read with python3 `tomllib`) satisfies: `OR` passes if any alternative is allowed, `AND` only if every part is, and a `WITH` exception is never itself allowed, so it passes only as an `OR` alternative to an allowed license, as under `cargo deny`. pnpm's own license column is only a guess when the field is missing, so it is used to list packages, never to judge them. A missing, `SEE LICENSE IN` or `UNLICENSED` license fails, and so does an expression mixing `AND` and `OR`: stricter than `cargo deny`, deliberately, until a real dependency needs one. Self-test with a stubbed `pnpm` so the engine job needs no Node. Needs T173; blocks T040. *Added 2026-10-15: AGENTS.md names a JS license check that did not exist, and T040 adds the first npm dependencies; ID out of phase order*

**Checkpoint**: CI runs and passes on an empty workspace.

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: The types and test machinery every story needs. **No user story work may begin until this phase completes** — every story touches `ManifoldMesh` or `WarningSet`.

### Geometry and mesh core

- [x] T022 `crates/openplush-geom/src/lib.rs` — `DVec2` and `DVec3`, plus `epsilon.rs` implementing the policy in data-model section 11: relative epsilons vs. absolute millimetres, each named and documented — *merged in #37*
- [x] T023 [P] `crates/openplush-geom/src/predicates.rs` — robust orientation and in-circle predicates with a `proptest` suite against exact rational evaluation — *merged in #38*
- [x] T024 `crates/openplush-mesh/src/soup.rs` — `TriangleSoup` carrying per-corner provenance (`SourceCorner`) — *merged in #39*
- [x] T025 `crates/openplush-mesh/src/halfedge.rs` — `ManifoldMesh` struct-of-arrays arenas exactly as data-model section 2 specifies: paired-index twins, explicit boundary half-edges, stored `prev`, per-corner attributes — *merged in #40*
- [x] T026 `crates/openplush-mesh/src/halfedge.rs` — `debug_validate()` checking all five invariants from section 2 — *merged in #41*
- [x] T027 [P] `crates/openplush-mesh/tests/invariants.rs` — `proptest` generating random meshes; every constructed `ManifoldMesh` satisfies `debug_validate()` — *merged in #42*
- [ ] T178 `crates/openplush-mesh/src/halfedge/` — split `ManifoldMesh::from_oriented_triangles` (`build.rs`), `ManifoldMesh::debug_validate` (`validate.rs`) and the invariant-3 bowtie test in `tests.rs`, removing their `#[expect(clippy::too_many_lines)]`. Refactor only, one PR per function. The two function PRs open with characterization tests that pass on the unsplit code and pin the order in which violations are reported (two violations, the earlier one wins); the bowtie test, being a test, only gets shorter, in the `validate.rs` PR. Before T060. *Added 2026-10-15; ID out of phase order*

### Error and warning channels

- [x] T175 The element IDs and proposal types of data-model §13a, **types only** (no algorithms), each in its owning crate so T028–T031 can name them: `ComponentId` in `openplush-mesh`; `SeamKind`, `ChartId`, `SeamChainId`, `ChartFailure`, `Proposal`, `ProposalKind`, `SeamReason`, `ProposalId` (with its FNV-1a derivation) in `openplush-seams`; `PieceId` in `openplush-pattern`; `Format` in `openplush-io`. One PR per crate: `001/21-T175-seams-types` (#49), `001/22-T175-component-id` (#50), `001/23-T175-piece-id` (#51), `001/24-T175-import-format` (#52). Test first: `ProposalId` is stable under unrelated renumbering and matches independently computed FNV-1a vectors, one per kind byte and per seam-kind byte, plus a multi-byte edge index to pin little-endian. *Added 2026-10-14: T028 referenced Phase 3 types (T069's `SeamKind`, charts, pieces), so Phase 2 could not compile without them* — *merged in #49, #50, #51, #52*
  - *2026-10-14 review follow-up, no task ID:* `ProposalId::from_hex` returns `SeamsError`, which `openplush-pipeline` did not re-export, so a shell could not name the error. `001/28-T175-seams-error-reexport` (#66) re-exports it and marks it `#[non_exhaustive]`
- [ ] T177 `openplush-mesh` exports `id_newtype!`, generating the shared shape of the eight `u32` IDs (derives, `new`, `index`); `VertexId`, `FaceId`, `HalfEdgeId`, `EdgeId`, `ComponentId`, `ChartId`, `SeamChainId` and `PieceId` use it, `ProposalId` (a `u64`) does not. One PR per crate; `openplush-pattern` gains the direct dependency on `openplush-mesh` that the DAG already allows. Test first: a type defined through the macro round-trips, is `const`-constructible, orders by index and prints `Name(3)` under `Debug`, as `MeshError` messages do today. *Added 2026-10-15: the third copy of the same boilerplate (AGENTS.md, DRY); ID out of phase order*
- [x] T028 `crates/openplush-pipeline/src/error.rs` — `EngineError` per contracts/engine-api.md section 5, every variant carrying a location — *merged in #57*
- [x] T029 `crates/openplush-pipeline/src/warning/mod.rs` — `WarningSet`, `Warning`, and `Remedy` as an **enum** of `Actionable` and `Advisory`, never `Option<Proposal>`. FR-065 is enforced by the type: a warning that explains nothing must not be constructible — *merged in #58*
- [x] T030 `crates/openplush-pipeline/src/warning/tests.rs` — `WarningSet` ordering is total (code string, target, remedy, explanation; data-model §13a) and independent of insertion order (Constitution V). *2026-10-14: a unit test until the first shipped `WarningCode` exists (none ship yet; the test uses `#[cfg(test)]` codes an integration test cannot see); it moves to `tests/warning_determinism.rs` with that code* — *merged in #58*
- [x] T031 `crates/openplush-pipeline/src/stage.rs` — `Staged<T>` and `StageResult<T>`. *2026-10-14: narrowed from "and the stage signatures as unimplemented stubs": most types those signatures name do not exist yet, and stub bodies cannot panic (`unimplemented`/`todo` are denied). Each stage signature lands with the task that owns its types; T043 grows with them* — *merged in #59*

### Test machinery and fixtures

- [x] T032 `crates/openplush-testkit/src/fixtures.rs` — fixture loader by name — *merged in #74*
- [ ] T033 [P] `crates/openplush-testkit/src/golden.rs` — `insta`-based golden compare for SVG and metrics JSON *2026-10-15: deferred until the first SVG or metrics output exists; a golden compare with nothing to compare would snapshot synthetic data*
- [x] T034 [P] `crates/openplush-testkit/src/oracles.rs` — the independent oracles Constitution IV demands: developable surfaces flatten with distortion near zero, injectivity, area conservation, seam-length parity — *merged in #75 (chart oracles) and #76 (outline and parity oracles)*
- [x] T035 [P] `fixtures/meshes/` — capsule, cube, sphere, plus `LICENSES.md`. CC0 or CC-BY only. *2026-10-14: `LICENSES.md` sits at `fixtures/LICENSES.md`, covering `defects/` and `seams/` as well; the fixtures are generated by `research/fixtures_gen/` (`001/29-T035-T037-fixtures`)* — *merged in #71*
- [x] T036 [P] `fixtures/defects/` — one model per defect class in data-model section 3: non-manifold edge, bowtie vertex, flipped normals, zero-area face, duplicate face, hole, unwelded glTF split-vertex export, self-intersection, multi-component. **SC-007 measures against exactly this directory**, so a missing class is a silently unmeasured requirement — *merged in #71*
- [x] T037 [P] `fixtures/seams/` — reference seam sets as JSON so pattern tests need no UI — *merged in #71*

### Shell and binding skeletons

- [x] T038 `bindings/openplush-cli` — `clap` skeleton with the subcommand surface from quickstart.md — *merged in #68*
- [x] T039 [P] `bindings/openplush-wasm` — `wasm-bindgen` facade skeleton — *merged in #69 (skeleton: builds as a cdylib, no exports yet)*
- [ ] T040 `packages/engine-client` — worker host, typed protocol, and a **mock worker** implementing contracts/worker-protocol.md so shell tests need no WASM build. Needs T176 (its npm dependencies must pass the license gate) and T041 (its protocol types are generated, never hand-written)
- [ ] T041 `scripts/gen-ts-types.sh` — generate TypeScript types from the Rust protocol definitions; CI fails on a diff, so protocol drift breaks the build rather than the pattern. Needs T179
- [ ] T179 The binding's protocol layer, so T041 has Rust definitions to generate from and T040 has types to use. Two PRs, one per crate: (a) `openplush-pipeline` — `EngineError::code()` (the variant name), `EngineError::locations()` and `EngineError::proposals()`, each exhaustive inside the crate, so a new variant without a code, locations or proposals fails to compile rather than reaching the shell unlocated; (b) `bindings/openplush-wasm/src/protocol/` — the DTOs of contracts/worker-protocol.md §2, §4 (handles), §5 and §6, the binding's own code list, the conversion from `EngineError`, and the panic handling the contract specifies; §3 payload types come with the task that adds each operation. (b) needs the worker-protocol amendments that record how a panic reaches the shell, which locations §6 can carry (source vertices and proposal IDs have no category today), and how the DTOs are encoded and mirrored in TypeScript; each is a contract change with its own ADR, written ahead of Phase N's T169 because (b) cannot start without them. (a) documents the new methods in `docs/reference/engine-api.md`. Test first: every variant's code; every variant but `Parse` (located by its format, which the message names) and `Cancelled` has element locations; each DTO serializes to the contract's JSON shape. *Added 2026-10-15: no task owned the binding's protocol types; ID out of phase order*
- [x] T042 CI determinism lint — ban `HashMap` and `HashSet` iteration and unstable sorts on output-affecting paths (Constitution V), via clippy `disallowed_methods` plus a reviewed allowlist — *merged in #70; follow-up #73*

**Checkpoint**: foundation ready. US1 can start immediately; the other stories build on its stages.

---

## Phase 3: US1 — Seams in, printable pattern out (Priority: P1) 🎯 MVP

**Goal**: A maker imports a model, sets its real-world size, draws seams, and prints a true-scale pattern with seam allowances.

**Independent Test**: Run the cube, capsule, and sphere fixtures through import to print; measure printed pieces against expected dimensions.

This phase is large because US1 *is* the product. Every other story decorates this loop.

### Tests for US1 (write first, observe failing)

- [ ] T043 [P] [US1] `crates/openplush-pipeline/tests/contract_surface.rs` — the stage signatures in contracts/engine-api.md compile and are callable as documented. *2026-10-14: grows one signature at a time, since T031 no longer stubs them: each US1 stage's assertion lands in the PR that adds that stage's `openplush-pipeline` signature, T092 at the latest; other stories' signatures get theirs in those stories*
- [ ] T044 [P] [US1] `crates/openplush-flatten/tests/developable.rs` — cylinder strip, cone, and folded flat sheet each flatten with distortion near zero. **The oracle that catches a flattener that is confidently wrong**
- [ ] T045 [P] [US1] `crates/openplush-pattern/tests/parity.rs` — on every fixture, both sides of every seam match within max(1.0 mm, 0.5%). Hard assertion, not a tolerance report (D-013)
- [ ] T046 [P] [US1] `crates/openplush-export/tests/scale.rs` — the scale-check square measures exactly 50.00 mm in the output file (SC-003)
- [ ] T047 [P] [US1] `bindings/openplush-cli/tests/end_to_end.rs` — capsule from import to SVG through the CLI path, matching a golden snapshot
- [ ] T048 [P] [US1] `crates/openplush-mesh/tests/defect_corpus.rs` — every model in `fixtures/defects/` is detected and reported (SC-007, 100%)
- [ ] T049 [P] [US1] `crates/openplush-pipeline/tests/determinism.rs` — same input twice gives byte-identical export (SC-008)

### Import and model health (FR-001 … FR-006)

- [ ] T050 [US1] `crates/openplush-io/src/obj.rs` — OBJ plus MTL and textures, via `tobj`
- [ ] T051 [P] [US1] `crates/openplush-io/src/gltf.rs` — glTF 2.0 and GLB via `gltf`; skinned and animated data ignored with a warning
- [ ] T052 [P] [US1] `crates/openplush-io/src/stl_ply.rs` — STL and PLY (the minor default was yes)
- [ ] T053 [US1] `crates/openplush-io/src/triangulate.rs` — polygons to triangles, preserving per-corner UVs and material assignment (FR-002). A NaN/∞ coordinate anywhere fails the import with `EngineError::NonFiniteInput` naming every non-finite source vertex (D-021); nothing is dropped
- [ ] T054 [US1] `crates/openplush-mesh/src/validate/nonmanifold.rs` — non-manifold edges and bowtie vertices, each located
- [ ] T055 [P] [US1] `crates/openplush-mesh/src/validate/degenerate.rs` — zero-area, duplicate, and repeated-index faces
- [ ] T056 [P] [US1] `crates/openplush-mesh/src/validate/topology.rs` — holes, boundary loops, connected components
- [ ] T057 [P] [US1] `crates/openplush-mesh/src/validate/orientation.rs` — inconsistent winding, non-orientable detection
- [ ] T058 [US1] `crates/openplush-mesh/src/validate/selfintersect.rs` — BVH triangle-triangle tests. **Warn only** (data-model section 3 step 8)
- [ ] T059 [US1] `crates/openplush-mesh/src/validate/report.rs` — `ValidationReport` with every defect locatable in 3D (FR-003)
- [ ] T060 [US1] `crates/openplush-mesh/src/repair/weld.rs` — repair step 1 (step 0 happens at import, D-021). **The weld is the one that matters most in practice**: glTF exporters split vertices at UV and normal discontinuities, so an unwelded import looks full of holes and nothing downstream works
- [ ] T061 [US1] `crates/openplush-mesh/src/repair/degenerate.rs` — step 2, including the double-sided-sheet flag
- [ ] T062 [US1] `crates/openplush-mesh/src/repair/nonmanifold_edge.rs` — step 3, cut and stitch per Guéziec et al. 2001; new boundary edges become mandatory seams
- [ ] T063 [US1] `crates/openplush-mesh/src/repair/bowtie.rs` — step 4, split one vertex per fan
- [ ] T064 [US1] `crates/openplush-mesh/src/repair/orientation.rs` — step 5, BFS per component from the largest face; signed volume positive when closed
- [ ] T065 [US1] `crates/openplush-mesh/src/repair/components.rs` — step 6; components under 1% of total area become candidate detail parts (FR-005)
- [ ] T066 [US1] `crates/openplush-mesh/src/repair/holes.rs` — step 7, classify opening vs. accidental; fill with Liepa 2003 minimal-area triangulation and fairing
- [ ] T067 [US1] `crates/openplush-mesh/src/repair/mod.rs` — `into_manifold()` returning `(ManifoldMesh, RepairReport)`. **Typestate gate**: `ManifoldMesh` gets no public constructor, so unrepaired data cannot reach the flattener by construction (FR-004, Constitution VII)
- [ ] T068 [US1] `crates/openplush-pattern/src/scale.rs` — real-world size from a dimension or measured reference distance; warn outside the keychain-to-1 m range (FR-006, D-015)

### Seams and charts (FR-010 … FR-014)

- [ ] T069 [US1] `crates/openplush-seams/src/set.rs` — `SeamSet` as non-destructive `BTreeMap<EdgeId, SeamKind>` (`SeamKind` itself comes from T175); the mesh never changes when seams change
- [ ] T070 [US1] `crates/openplush-seams/src/graph.rs` — seam graph node classification and maximal seam chains (the unit of notching, labelling, and parity)
- [ ] T071 [US1] `crates/openplush-seams/src/path.rs` — shortest surface path between two picked points, for seam drawing (FR-010)
- [ ] T072 [US1] `crates/openplush-seams/src/dart.rs` — darts as seams ending inside a piece; dart tips stay single vertices, a slit rather than a separation (FR-011)
- [ ] T073 [US1] `crates/openplush-seams/src/chart.rs` — flood fill across non-seam edges, duplicating vertices along interior seam edges; provenance maps back to global IDs
- [ ] T074 [US1] `crates/openplush-seams/src/sewing_map.rs` — `SewingMap` with `k >= 2` sides per chain. v0.1 exposes `k = 2`; the model supports more from day one so FR-016 is not a migration later
- [ ] T075 [US1] `crates/openplush-seams/src/topology.rs` — Euler characteristic check; flattenable iff genus 0 with exactly one boundary loop
- [ ] T174 [US1] `crates/openplush-seams/src/fold.rs` — refuse a **folded chart** before flattening: adjacent faces whose normals are within a new named angle tolerance (`FOLD_ANGLE`, added to data-model §11 and `openplush_geom::epsilon` by this task) of antiparallel across a shared edge, i.e. a surface lying back over itself. A typed error naming the chart and the edge, upstream of T079–T080; never left for the flattener to discover as an injectivity failure. Needs an `EngineError` entry (contracts/engine-api.md §5) agreed in this task's PR. Test first with S7's folded synthetic caps (`research/s7_boundary_pinned/`). *Added 2026-10-14 from S7 finding 3*
- [ ] T076 [US1] `crates/openplush-seams/src/proposals.rs` — minimal extra cuts for each failure mode in data-model section 4: closed region, multiple boundaries, positive genus. Returned **inside** `EngineError::ChartNotDisk` so the error is the suggestion (FR-013)
- [ ] T077 [US1] `crates/openplush-seams/src/opening.rs` — mark any chain as the stuffing opening (FR-012)

### Flattening (FR-020, FR-023, FR-026)

- [ ] T078 [US1] `crates/openplush-flatten/src/solver.rs` — `faer` integration with `default-features = false`, since the default features pull in `rayon` and break the WASM target
- [ ] T079 [US1] `crates/openplush-flatten/src/boundary.rs` — **prescribe boundary edge lengths to their 3D lengths** (D-013, BFF-style). This is what makes seam parity exact without any cross-chart coupling: both sides of a seam come from the same 3D edge, so pinning each chart independently makes them agree
- [ ] T080 [US1] `crates/openplush-flatten/src/interior.rs` — SLIM symmetric-Dirichlet minimisation with the boundary held fixed and a flip-preventing line search
- [ ] T081 [US1] `crates/openplush-flatten/src/injectivity.rs` — verify det J > 0 on every face; return `EngineError::Flip` with the offending faces. Constitution III post-flatten invariant
- [ ] T082 [US1] `crates/openplush-flatten/src/fallback.rs` — when the prescribed boundary cannot close, return `BoundaryNotRealizable` plus a dart proposal. **Never relax parity to escape this** — that is the one thing the fallback must not do, and S7 (T007) sizes how often it fires
- [ ] T083 [US1] `crates/openplush-flatten/src/session.rs` — `FlattenSession::step()` so the single-threaded WASM worker can yield, report progress, and honour cancellation without ever exposing a half-flattened result

### Pattern, layout, export (FR-030, FR-031, FR-060, FR-061)

- [ ] T084 [US1] `crates/openplush-pattern/src/fabric.rs` — the `Fabric` entity from data-model section 6a and the seven presets from research R7, shipped as data. Derived values (rotation set, mirrored printing, pile-scaled allowance) are computed, never stored
- [ ] T085 [US1] `crates/openplush-pattern/src/allowance.rs` — per-edge allowance as a union of per-segment strips via `i_overlay`, not a single-distance offset, because seam, opening, and hem allowances differ along one outline (FR-030). Defensiveness level set by S3 (T003): [ADR 0006](../../docs/adr/0006-allowance-construction.md)
- [ ] T086 [US1] `crates/openplush-pattern/src/lines.rs` — sewing line and cut line drawn distinctly (FR-031)
- [ ] T087 [US1] `crates/openplush-pattern/src/parity.rs` — assert parity within max(1.0 mm, 0.5%) and return `EngineError::SeamParity` otherwise. Should be unreachable; it exists because a silently mismatched seam is the failure Constitution III calls worse than a crash
- [ ] T088 [US1] `crates/openplush-layout/src/shelf.rs` — simple shelf packing for v0.1 (optimised nesting is v0.2, FR-062)
- [ ] T089 [US1] `crates/openplush-layout/src/pages.rs` — tile for Letter and A4 with overlap, registration marks, page map, and a scale-check square on every page (FR-061)
- [ ] T090 [US1] `crates/openplush-export/src/svg.rs` — SVG at true scale, document dimensions in millimetres, 1 user unit = 1 mm (FR-060)
- [ ] T091 [US1] `crates/openplush-export/src/pdf.rs` — tiled PDF, approach chosen by S5 (T005): direct `pdf-writer`, [ADR 0008](../../docs/adr/0008-pdf-export.md). Browsers print SVG at unreliable scale, which is why PDF is in v0.1 at all
- [ ] T092 [US1] `crates/openplush-pipeline/src/stages.rs` — wire every stage; memoise by content hash so a seam edit re-runs chart and flatten only for affected charts (FR-014), and a scale or shrink change re-runs pattern onward only
- [ ] T093 [US1] `bindings/openplush-cli/src/commands/` — `validate`, `repair`, and `pattern` subcommands exactly as quickstart.md documents (Constitution II)

### Web shell for US1

- [ ] T094 [US1] `apps/web/src/viewer/` — Three.js scene, model load, orbit controls
- [ ] T095 [US1] `apps/web/src/scale/` — size entry, unit selection, human-silhouette reference (FR-006)
- [ ] T096 [US1] `apps/web/src/health/` — defect list with 3D highlight; nothing changes until the maker confirms (FR-004)
- [ ] T097 [US1] `apps/web/src/seams/` — seam drawing with `three-mesh-bvh` edge picking, plus the optional symmetry-mirroring mode
- [ ] T098 [US1] `apps/web/src/export/` — page size choice, export, download
- [ ] T099 [US1] `packages/engine-client/src/worker.ts` — the real worker host with transferable buffers, replacing the mock for integration

**Checkpoint**: US1 is independently functional. A maker can import, seam, and print a true-scale pattern. This is the MVP.

---

## Phase 4: US2 — See where the pattern will fail (Priority: P1)

**Goal**: Before cutting, the maker sees where fabric would stretch, pucker, or not fit, and gets concrete suggestions.

**Independent Test**: Sphere fixture with one seam shows high distortion near the poles and suggests darts there.

### Tests for US2

- [ ] T100 [P] [US2] `crates/openplush-flatten/tests/metrics.rs` — singular-value metrics on a known-stretched fixture match hand-computed values
- [ ] T101 [P] [US2] `crates/openplush-seams/tests/dart_suggestion.rs` — sphere with one seam: suggestions cluster at the poles, and applying one measurably lowers distortion
- [ ] T102 [P] [US2] `apps/web/tests/heatmap.spec.ts` — the heatmap palette is color-blind-safe and identical between the 3D and 2D views (FR-083)

### Implementation

- [ ] T103 [US2] `crates/openplush-flatten/src/metrics.rs` — per-triangle Jacobian singular values: max stretch, max compression, area ratio, conformal ratio. Interpreted against the active fabric's tolerances, not fixed constants
- [ ] T104 [US2] `crates/openplush-flatten/src/summary.rs` — numeric per-piece summary (FR-021)
- [ ] T105 [US2] `crates/openplush-seams/src/suggest_dart.rs` — the four-step MVP algorithm from data-model section 5: cluster over-tolerance faces, find the max angle-defect vertex, propose a dart from the nearest boundary toward that cone point, accept for display only if distortion drops by the threshold
- [ ] T106 [US2] `crates/openplush-seams/src/concavity.rs` — warn when a concave region has no seam through it, with an `Actionable` remedy proposing a seam along the crease (FR-025). **This is the warning that matters most for a plush**: stuffing pushes an unsupported concavity open and the shape is simply lost
- [ ] T107 [US2] `crates/openplush-pipeline/src/proposals.rs` — every proposal is preview-then-accept, applied only on acceptance (FR-022, Constitution VII)
- [ ] T108 [US2] `apps/web/src/quality/` — heatmap on both the 3D model and the flat pieces, with a viridis or cividis palette and a per-piece summary panel
- [ ] T109 [US2] `apps/web/src/quality/suggestions.tsx` — suggestion list, preview on hover, apply on accept

**Checkpoint**: US1 and US2 both work. The maker can now see and fix problems before cutting fabric.

---

## Phase 5: US3 — Sewing marks that make assembly unambiguous (Priority: P2)

**Goal**: Pieces carry match notches, partner labels, cut counts, mirror flags, and a marked stuffing opening.

**Independent Test**: For every seam in the capsule fixture, both sides carry matching notch codes at equal arc-length positions.

### Tests for US3

- [ ] T110 [P] [US3] `crates/openplush-pattern/tests/notches.rs` — paired notches sit at the same fractional arc length on both sides of every chain
- [ ] T111 [P] [US3] `crates/openplush-pattern/tests/dedupe.rs` — a symmetric model yields one template labelled cut 2 mirrored, not two identical pieces
- [ ] T112 [P] [US3] `crates/openplush-pattern/tests/clip_marks.rs` — tight concave curves get clip marks, tight convex curves get notch-out marks, and zero-allowance edges get neither

### Implementation

- [ ] T113 [US3] `crates/openplush-pattern/src/notches.rs` — notches at chain ends plus every 50 mm (provisional); side B placed at the same fractional arc length as side A
- [ ] T114 [US3] `crates/openplush-pattern/src/pairing.rs` — unambiguous pairing codes. Notch multiplicity differs between adjacent chains on the same piece, so a sewist cannot rotate a piece and match the wrong edge. **The whole point is that a wrong match must be impossible, not merely unlikely**
- [ ] T115 [US3] `crates/openplush-pattern/src/labels.rs` — piece label block: ID, name, cut quantity, mirror flag, fabric, grain and nap arrows, seam-partner labels (FR-033)
- [ ] T116 [US3] `crates/openplush-pattern/src/dedupe.rs` — identical and mirror-image pieces collapse to one template (FR-034)
- [ ] T117 [US3] `crates/openplush-pattern/src/marks.rs` — clip marks on tight concave curves, notch-out marks on tight convex curves, spacing proportional to curvature (FR-035)
- [ ] T118 [US3] `crates/openplush-pattern/src/opening.rs` — the opening chain is labelled and uses the opening allowance (FR-012)
- [ ] T119 [US3] `apps/web/src/pieces/` — piece inspector showing labels, notches, and partners

**Checkpoint**: the output is a usable sewing pattern rather than a set of shapes.

---

## Phase 6: US7 — Save, reopen, share a project (Priority: P2)

**Goal**: One portable file, reopened later or on another machine, editing continues with undo history from that point.

**Independent Test**: Save a project mid-edit, reopen it, and confirm seams, fabrics, markings, scale, and acknowledged warnings all survive.

### Tests for US7

- [ ] T120 [P] [US7] `crates/openplush-project/tests/roundtrip.rs` — save then load is lossless for every field in contracts/project.schema.json
- [ ] T121 [P] [US7] `crates/openplush-project/tests/schema.rs` — written documents validate against the JSON Schema
- [ ] T122 [P] [US7] `crates/openplush-project/tests/provenance_stability.rs` — **change repair settings and confirm seams still land on the same edges.** This is the test that protects the provenance-key design; if it fails, reopening a project silently moves the maker's seams
- [ ] T123 [P] [US7] `crates/openplush-project/tests/migration.rs` — a v1 document opens in a later engine unchanged

### Implementation

- [ ] T124 [US7] `crates/openplush-project/src/document.rs` — the document type matching the schema; re-export the public document API from `openplush-pipeline` as `openplush_pipeline::project` (D-020)
- [ ] T125 [US7] `crates/openplush-project/src/provenance.rs` — stable keys from source provenance: sorted source-vertex pairs for edges, sorted source-corner triples for faces. Never `EdgeId`, which moves when repair settings change
- [ ] T126 [US7] `crates/openplush-project/src/serde.rs` — JSON plus embedded binary mesh chunk; store the repair **plan**, not the repaired mesh, so a later engine with better repair improves old projects
- [ ] T127 [US7] `crates/openplush-project/src/migrate.rs` — `schemaVersion` with forward migrations (FR-070)
- [ ] T128 [US7] `crates/openplush-pipeline/src/undo.rs` — undo and redo for every edit, as diffs over `SeamSet` and the option structs (FR-071)
- [ ] T129 [US7] `apps/web/src/project/` — save, load, and recent-project UI
- [ ] T130 [US7] `bindings/openplush-cli/src/commands/project.rs` — CLI save and load, for headless parity. Uses `openplush_pipeline::project`, never `openplush-project` directly (D-020)

**Checkpoint**: work is durable. Everything before this was ephemeral.

---

## Phase 7: US5 — Markings, colors, and detail parts (Priority: P2)

**Goal**: The maker places images on the model, paints fabric regions, and marks small separate parts as details; these appear on pieces as placement marks or appliqué outlines.

**Independent Test**: A marking crossing a seam is split across both pieces with alignment marks; separate eye meshes produce placement marks and no pattern pieces of their own.

### Tests for US5

- [ ] T131 [P] [US5] `crates/openplush-decal/tests/seam_crossing.rs` — a marking straddling a seam is clipped per chart with matching alignment ticks on both sides
- [ ] T132 [P] [US5] `crates/openplush-decal/tests/reseam_survival.rs` — markings survive re-seaming and re-flattening, because they are stored in barycentric coordinates rather than pattern space
- [ ] T133 [P] [US5] `crates/openplush-decal/tests/detail_parts.rs` — a detail component produces a placement mark and no piece

### Implementation

- [ ] T134 [US5] `crates/openplush-decal/src/attach.rs` — surface-attached storage as barycentric coordinates on mesh faces (FR-040)
- [ ] T135 [US5] `crates/openplush-decal/src/clip.rs` — per-chart clipping with alignment ticks across seams
- [ ] T136 [US5] `crates/openplush-decal/src/detail.rs` — closest-point projection of a detail component's centroid and footprint onto the main surface; distinct mark for safety eyes (FR-042)
- [ ] T137 [US5] `crates/openplush-pattern/src/regions.rs` — painted fabric regions; region boundaries become seams, so a piece can never span two fabrics (FR-041)
- [ ] T138 [US5] `crates/openplush-export/src/grouping.rs` — group pieces by fabric in the output
- [ ] T139 [US5] `apps/web/src/markings/` — place, move, and scale markings; paint fabric regions; mark components as details

**Checkpoint**: patterns carry the maker's design intent, not just its shape.

---

## Phase 8: US6 — Fabric-aware orientation and layout (Priority: P3)

**Goal**: The maker picks a fabric, sets fur direction on the model, and gets pieces oriented so nap runs correctly, laid out to use less fabric.

**Independent Test**: With a nap direction drawn on the model, every piece shows grain and nap arrows consistent with the 3D direction, and no piece is rotated against the nap.

### Tests for US6

- [ ] T140 [P] [US6] `crates/openplush-fields/tests/direction_field.rs` — a field with sparse constraints is smooth and honours them; singularities are handled rather than producing NaN
- [ ] T141 [P] [US6] `crates/openplush-pattern/tests/grain_angle.rs` — vector and line field averaging differ correctly on a 180-degree-ambiguous case
- [ ] T142 [P] [US6] `crates/openplush-pattern/tests/rotation_sets.rs` — napped fabric permits only 0 degrees; non-napped permits 0 and 180
- [ ] T143 [P] [US6] `crates/openplush-pattern/tests/fabric_warnings.rs` — each of the five fabric-driven warnings in data-model section 6a fires on a fixture built to trigger it, and each carries a `Remedy`

### Implementation

- [ ] T144 [US6] `crates/openplush-fields/src/curvature.rs` — per-vertex and per-face curvature, shared with US2's metrics
- [ ] T145 [US6] `crates/openplush-fields/src/direction.rs` — globally optimal direction field per Knöppel et al. 2013; vector field (1-RoSy) for nap, line field (2-RoSy) for grain. Same sparse solver path as flattening
- [ ] T146 [US6] `crates/openplush-fields/src/default.rs` — with no strokes, project a global down-and-back vector onto each tangent plane and interpolate across singularities (FR-051)
- [ ] T147 [US6] `crates/openplush-pattern/src/grain.rs` — push the 3D direction through each face Jacobian, average per chart, rotate the piece so the fabric axis maps to pattern plus-Y
- [ ] T148 [US6] `crates/openplush-pattern/src/coherence.rs` — coherence R per piece; warn below 0.85, suggest a split below 0.7, with the split curve as an `Actionable` remedy (FR-052)
- [ ] T149 [US6] `crates/openplush-pattern/src/pile.rs` — pile-scaled default allowance, and the small-piece and tight-curve warnings at the provisional 3x multiple (FR-055). Note that at 30 mm pile this warns on most plush faces, which is informative rather than wrong, and is the strongest argument for keeping the multiple configurable
- [ ] T150 [US6] `crates/openplush-pattern/src/mirror_print.rs` — pile fabrics print mirrored with a trace-on-wrong-side label by default, switchable to right-side printing (FR-054, D-019)
- [ ] T151 [US6] `crates/openplush-pattern/src/stretch_mismatch.rs` — compare per-direction stretch across each shared seam **after** grain angles are known, warn past the provisional 20-point threshold (FR-058). Evaluated post-orient because two pieces of the same knit cut at 90 degrees to each other mismatch as badly as two different fabrics
- [ ] T152 [US6] `crates/openplush-pattern/src/zero_allowance.rs` — zero allowance for edge-stitched and appliqué pieces on non-fraying fabric; such pieces are excluded from clip marking since there is no allowance to clip (FR-057)
- [ ] T153 [US6] `crates/openplush-layout/src/rotations.rs` — honour the per-fabric rotation set and mirrored-copy reflection axis (FR-053)
- [ ] T154 [US6] `crates/openplush-layout/src/roll.rs` — layout within a given fabric roll width, reporting the fabric length needed (FR-062)
- [ ] T155 [US6] `apps/web/src/fabric/` — fabric picker with the seven presets, profile editor, nap-direction strokes, grain and nap arrow display

**Checkpoint**: all v0.1 user stories are independently functional.

---

## Phase 9: Warning plumbing and export gating (cross-cutting)

**Purpose**: FR-064 and FR-065 span every stage, so they land once the warnings they carry exist. Doing this earlier means plumbing with nothing to plumb; later means retrofitting every stage.

- [ ] T156 [P] `crates/openplush-pipeline/tests/warning_flow.rs` — a warning raised at any stage reaches export intact, with its remedy
- [ ] T157 [P] `crates/openplush-export/tests/warning_page.rs` — unresolved warnings appear in the first-page summary, and the summary is part of the golden snapshot so a warning silently vanishing from the printed page is a test failure
- [ ] T158 `crates/openplush-pipeline/src/warning_flow.rs` — accumulate `WarningSet` across stages, deterministically ordered
- [ ] T159 `crates/openplush-export/src/warning_summary.rs` — render the first-page block: code, location, explanation, remedy (FR-064)
- [ ] T160 `crates/openplush-project/src/ack.rs` — persist acknowledgements keyed by code plus provenance target, so they survive re-repair and reopening
- [ ] T161 `apps/web/src/warnings/` — warning panel, one-time acknowledgement before first export, and remedy actions. **Export is never blocked** (FR-064)

---

## Phase 10: Polish and cross-cutting concerns

- [ ] T162 [P] `apps/web/src/a11y/` — keyboard operation for every non-3D control; verify heatmap palettes against a color-blindness simulator (FR-083)
- [ ] T163 [P] PWA manifest and service worker; verify the app works offline after first load (FR-080)
- [ ] T164 [P] `apps/web/tests/no_network.spec.ts` — automated assertion of zero network requests during a working session (SC-006)
- [ ] T165 [P] Performance pass against SC-005: 50k triangles, seam edit to updated preview within 10 s. Budget informed by S2 (T002)
- [ ] T166 [P] `docs/tutorials/first-plush.md` — the walkthrough a first-time maker follows
- [ ] T167 [P] `docs/explanation/why-seams-go-there.md` — the teaching document; the geometry reasoning behind seam placement, and the groundwork feature 002 builds on
- [ ] T168 [P] `docs/reference/` — fabric profiles, warning codes with their meanings and remedies, CLI reference. *2026-10-14: written incrementally, not here ([ADR 0003](../../docs/adr/0003-as-built-docs.md)): each PR whose change AGENTS.md (Specs and docs) lists documents it. T168 is the closing pass: an index of those pages and a check that they match the code*
- [ ] T169 [P] `docs/adr/` — write up every spike outcome and any decision that changed a principle or public contract
- [ ] T170 Run the full [quickstart.md](./quickstart.md) walkthrough end to end and fix what does not work. **Including printing on paper and measuring the scale square with a ruler** — a pattern that prints at 97% is a ruined project and nothing on screen reveals it
- [ ] T171 SC-004 usability test: five or more first-time makers, prepared model, printable pattern within 15 minutes. Needs recruiting and a written protocol; gates the release claim, not the build

---

## Dependencies and Execution Order

### Phase dependencies

- **Phase 0 (spikes)**: start immediately. T007 (S7) and T006 (S6) block Phase 3
- **Phase 1 (setup)**: no dependencies
- **Phase 2 (foundational)**: needs Phase 1. **Blocks every user story**
- **Phase 3 (US1)**: needs Phase 2 plus S7. The other stories depend on its stages
- **Phase 4 (US2)**: needs US1's flatten stage (T078–T083)
- **Phase 5 (US3)**: needs US1's pattern stage (T084–T087)
- **Phase 6 (US7)**: needs Phase 2 only — can run parallel to Phases 4 and 5
- **Phase 7 (US5)**: needs US1's chart stage (T073) and pattern stage
- **Phase 8 (US6)**: needs US1's pattern stage and T084's `Fabric`
- **Phase 9 (warnings)**: needs the warnings from Phases 4 and 8 to exist
- **Phase 10 (polish)**: needs everything intended for v0.1

### Honest note on story independence

The tasks template wants every story independently deliverable. **US1 is not independent of the pipeline — it *is* the pipeline**, and US2, US3, US5, and US6 genuinely cannot ship before it. US7 is the only story that can be built in parallel with US1's later stages. Pretending otherwise would produce a task graph that lies about what can be parallelised, so the graph above reflects the real dependencies instead.

### Parallel opportunities

- All of Phase 0 except T007's dependents
- Phase 1: everything except T009, T016, T172 (needs T016), T173 (needs T013 and T016), and T176 (needs T173)
- Phase 2: T023, T027, T033–T037, T039; T175's four per-crate PRs in parallel, before T028; T177 and T178 alongside anything. T179, then T041, then T040; T176 (Phase 1) before T040
- Phase 3: all test tasks T044–T049 together (T043 grows with each stage's signature); then the import parsers T050–T052; then the validators T055–T057
- Phase 6 (US7) alongside Phases 4 and 5
- Within every story, all test tasks run in parallel before any implementation starts

### Within each story

1. Tests written and observed failing
2. Types before algorithms
3. Engine before shell — always. A shell task blocked on an engine task is normal; the reverse means the boundary has been violated
4. Story complete and independently verified before the next priority starts

---

## Implementation Strategy

### MVP first

1. Phase 0 spikes S6 and S7, Phase 1, Phase 2
2. Phase 3 (US1) complete
3. **Stop and validate**: run the quickstart, print a pattern, measure it with a ruler
4. Phases 4 and 9 — the quality view and the warning channel, which together are what makes the output trustworthy rather than merely present

### Then, in order

5. Phase 5 (US3) — the pattern becomes genuinely sewable
6. Phase 6 (US7) — work becomes durable
7. Phase 7 (US5), Phase 8 (US6)
8. Phase 10 — polish, docs, usability test

### Rules that hold throughout

- Never update a golden snapshot to make a test pass. If output legitimately changed, say so and leave the update to a human reviewer (`AGENTS.md`)
- Never skip, weaken, or delete a test to get to green
- One task per PR, referenced as `Task: T0NN (specs/001-pattern-pipeline/tasks.md)`
- A violated invariant returns a typed error. Never clamp, skip, or quietly fix up bad geometry — a silently wrong pattern is discovered after the fabric is cut, which is the whole reason this project has a constitution

---

## Traceability

| Spec requirement | Tasks |
|---|---|
| FR-001 … FR-006 import, health, scale | T050–T068 |
| FR-010 … FR-014 seams | T069–T077 |
| FR-015, FR-016 auto-seams, inserted pieces | **deferred (D-015)** — none |
| FR-020 … FR-026 flatten and quality | T078–T083, T100–T107, T174 |
| FR-030 … FR-036 pattern features | T084–T087, T110–T119 |
| FR-040 … FR-042 markings and details | T131–T139 |
| FR-043 texture transfer | **deferred** — none |
| FR-050 … FR-058 fabric and grain | T084, T140–T155 |
| FR-060 … FR-063 output | T088–T091, T154 |
| FR-064, FR-065 warning model | T156–T161 |
| FR-070, FR-071 project | T120–T130 |
| FR-080 … FR-083 platform and privacy | T038, T093, T130, T162–T164 |
| SC-001 parity | T045, T087 |
| SC-002 stretch | T044, T103 |
| SC-003 scale square | T046, T089, T170 |
| SC-004 usability | T171 |
| SC-005 performance | T002, T165 |
| SC-006 no network | T164 |
| SC-007 defect corpus | T036, T048 |
| SC-008 determinism | T017, T030, T042, T049 |
