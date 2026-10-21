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
- `needs:ID[,ID]` — unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:D-0NN` — a proposed decision this one waits on; only the maintainer clears it
- `human:XX` — needs a person (outside review, measured values); no agent picks it up
- Paths are exact and follow [plan.md](./plan.md#project-structure)

## PR packaging

**A task is a unit of work, not a unit of review.** 179 tasks do not mean 179 PRs, and it certainly does not mean one PR per phase. Policy in [`AGENTS.md`](../../AGENTS.md#commits-and-prs), detail in [docs/how-to/pull-requests.md](../../docs/how-to/pull-requests.md). In short: one intent per PR, what it brings in toward an epic and why, with every task it touches finished and ticked in it; each task a test commit then its implementation; interdependent intents stacked ([ADR 0013](../../docs/adr/0013-intent-sized-pull-requests.md), 2026-10-18, which retired the 400-line budget).

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
| `001/NN-T050-obj-import` | T050, T053 (landed as `001/60-T050-T053-obj-import`, #108) |
| `001/NN-T055-validators` | T055, T056, T057 (landed with T054 and T059 as `001/62-T054-T059-validation`, #114; T058 and T048 as `001/63-T058-T048-self-intersection`, #117) |

`NN` is the slice's position in feature 001, assigned when the PR is opened (the next unused number). It is not fixed here because it depends on what lands first. Setup tasks (Phase 1) are repo plumbing, so they are `chore/` branches, not 001 slices.

T022–T027 were planned as one batch and landed as one slice per task, because each was its own intent: `001/11-T022-geom-core` (#37), `001/12-T023-robust-predicates` (#38), `001/13-T024-triangle-soup` (#39), `001/14-T025-halfedge-mesh` (#40), `001/15-T026-debug-validate` (#41), `001/16-T027-invariant-proptests` (#42).

### Split under the old budget — history

Until ADR 0013 (2026-10-18) these tasks were planned to split by capability under the 400-line budget. The budget is retired: an unfinished task lands whole, in the PR whose intent it serves. The table keeps every planned split; a row that names PRs records what landed that way, and the rest now land whole:

| Task | Why it splits |
|---|---|
| T060 weld | Spatial hash, deterministic tie-break, and glTF split-vertex handling are three testable capabilities (landed as a move of validate's grouping, #121, then the remap with tests for all three, #122) |
| T061 degenerates | Dropping repeated-index faces and duplicates (with the sheet flag) is separate from cap stitching (D-036) |
| T062 non-manifold edge pairing | Dihedral sorting, orientation pairing, and the sheet-layer rule (the copies themselves are made by the half-edge build, T181) |
| T066 hole filling | Classification is separate from Liepa triangulation and fairing |
| T073 chart construction | Flood fill, vertex duplication, and provenance mapping |
| T080 SLIM interior | Energy and line search may land before the convergence and iteration policy |
| T085 allowance strips | Strip construction, then corner joins, then the tight-curve union |
| T089 page tiling | Tiling geometry, then registration marks and labels, then the scale square (landed as #175 tiling, #179 rows from the top edge, #178 labels and marks, #181 scale square) |
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
- [ ] T002 [P] **S2** — SLIM plus `faer` on a 50k-triangle chart in WASM: wall time per iteration, native vs. browser. Feeds SC-005's 10 s budget. If the budget is missed, the remeshed-proxy size in the plan's Technical Context is the dial. Box: 2 days. *2026-10-14: there is no remeshed proxy (D-024, S1), so S2 measures charts at input size; a missed budget goes back to the maintainer instead of turning a dial* *2026-10-18: D-041 splits it: the dependency-gate check on `faer` (S2a) comes before T078, the timing (S2b) after T080* *2026-10-18: S2a done in #183: faer 0.24.4 without default features passes determinism (native and wasm32 bit-identical) and the boundary, and fails `cargo deny` advisories on `paste` (RUSTSEC-2024-0436); the maintainer accepted an ignore for it (D-050), and T078 proceeds*
- [x] T003 [P] **S3** — `i_overlay` offset robustness: fuzz 10k random valid outlines with mixed per-edge allowances; catalogue panics (upstream issue 74). Determines how defensive the allowance construction (T085) must be. Box: 1 day *2026-10-15: answered in [ADR 0006](../../docs/adr/0006-allowance-construction.md) (Accepted) and `research/s3_i_overlay_fuzz/` (#81). No panics and no wrong booleans on valid input; invalid input, sub-grid edges and chained booleans are silently wrong, so T085 validates input, merges short edges, makes one `i64` call with fold guards, and checks the result into typed errors; a remaining panic is a crash, not caught ([ADR 0009](../../docs/adr/0009-wasm-panic-is-a-worker-crash.md))*
- [x] T004 [P] **S4** — `jagua-rs` WASM bundle size and discrete rotation-set support. A bundle-size surprise changes the v0.1 layout design even though nesting is v0.2. Box: 0.5 day *2026-10-15: answered in [ADR 0007](../../docs/adr/0007-layout-nesting-engine.md) (Proposed) and `research/s4_jagua_rs_wasm/` (#79). Rotation sets {0°}/{0°,180°} and mirror axes supported; bit-identical native vs wasm32; +364 KB raw / +135 KB gzip, so v0.1 layout design unchanged. 1.1.0 fails `cargo deny` and the boundary check until upstream feature-gates `rayon`/`getrandom`/`web-time`; `sparrow` is not adoptable as a dependency*
- [x] T005 [P] **S5** — `svg2pdf` under WASM vs. writing PDF directly. Decides the approach for T091. Box: 0.5 day *2026-10-15: answered in [ADR 0008](../../docs/adr/0008-pdf-export.md) (Accepted) and `research/s5_svg2pdf_wasm/` (#80). T091 writes the PDF directly with `pdf-writer` 0.15 and labels in standard-14 Helvetica with no embedded font; `svg2pdf` is not adopted (4/3 scale trap at its default dpi, silent text loss, no allowlisted font, +213 KB gzip). T091 returns a typed error for a label outside WinAnsi*
- [x] T006 [P] **S6** — Confirm licenses for `faer`, `i_overlay`, `sparrow`, `svg2pdf`, `threecrate`. Any non-allowlist result needs an ADR or a replacement (Constitution IX). Box: 0.5 day. **Done 2026-10-14: all pass; `sparrow` needs git-source + clarify entries before v0.2 — [research S6 result](./research.md#s6-result--licenses-2026-10-14--closed)**
- [x] T007 **S7** — **Boundary-pinned flattening (D-013).** On every fixture: how often do prescribed boundary lengths fail to close, and does the result stay injective? Measure the interior-distortion penalty against free-boundary SLIM. Parity is a hard contract, so a bad result changes how aggressive the fallback (T082) must be, never the contract. Highest-risk spike in the list: it tests the decision the whole pattern stage rests on. Box: 2 days. **Done 2026-10-15:** 36 runs (the 10 fixture charts, the same 10 jittered, 16 refined): 0 closure failures, 0 injectivity failures. Pinning costs nothing on developable charts and moves curvature into interior compression (hemisphere cap: 0.61 pinned vs 0.87 free, while the free solve drifts seam lengths 41–47%). T082 is a rare-path typed error; T105 thresholds on area-integrated compression — [research S7 result](./research.md#s7-result--boundary-pinned-flattening-2026-10-15--closed), [ADR 0005](../../docs/adr/0005-boundary-pinned-flattening.md). Branches `001/35-T007-s7-fixture-corpus` (#77, loader and cut), `001/38-T007-s7-fixture-run` (#78, run, measures, ADR)
- [ ] T008 [P] human:S8 **S8** — Get the fabric preset value sets in research R7 (fleece, minky and long faux fur first, D-046) reviewed by someone who sews with these fabrics, or replace them from supplier spec sheets. Every number is currently a guess. Box: 0.5 day plus outside review

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
- [x] T176 `scripts/check-js-licenses.sh`, wired into `scripts/check.sh`'s `shell` step, with `scripts/tests/check-js-licenses.test.sh` — the JS half of Constitution IX's license gate: every installed npm package, dev dependencies included, must **declare** (in its own `package.json`) a license expression that `deny.toml`'s allowlist (read with python3 `tomllib`) satisfies: `OR` passes if any alternative is allowed, `AND` only if every part is, and a `WITH` exception is never itself allowed, so it passes only as an `OR` alternative to an allowed license, as under `cargo deny`. pnpm's own license column is only a guess when the field is missing, so it is used to list packages, never to judge them. A missing, `SEE LICENSE IN` or `UNLICENSED` license fails, and so does an expression mixing `AND` and `OR`: stricter than `cargo deny`, deliberately, until a real dependency needs one. Self-test with a stubbed `pnpm` so the engine job needs no Node. Needs T173; blocks T040. *Added 2026-10-15: AGENTS.md names a JS license check that did not exist, and T040 adds the first npm dependencies; ID out of phase order* — *merged in #92 (branch `chore/js-license-gate`). Unlisted lockfile packages, such as other platforms' builds, are judged by their registry license*

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
- [x] T178 `crates/openplush-mesh/src/halfedge/` — split `ManifoldMesh::from_oriented_triangles` (`build.rs`), `ManifoldMesh::debug_validate` (`validate.rs`) and the invariant-3 bowtie test in `tests.rs`, removing their `#[expect(clippy::too_many_lines)]`. Refactor only, one PR per function. The two function PRs open with characterization tests that pass on the unsplit code and pin the order in which violations are reported (two violations, the earlier one wins); the bowtie test, being a test, only gets shorter, in the `validate.rs` PR. Before T060. *Added 2026-10-15; ID out of phase order* — *merged in #86 (`build.rs`) and #91 (`validate.rs` and the bowtie test)*

### Error and warning channels

- [x] T175 The element IDs and proposal types of data-model §13a, **types only** (no algorithms), each in its owning crate so T028–T031 can name them: `ComponentId` in `openplush-mesh`; `SeamKind`, `ChartId`, `SeamChainId`, `ChartFailure`, `Proposal`, `ProposalKind`, `SeamReason`, `ProposalId` (with its FNV-1a derivation) in `openplush-seams`; `PieceId` in `openplush-pattern`; `Format` in `openplush-io`. One PR per crate: `001/21-T175-seams-types` (#49), `001/22-T175-component-id` (#50), `001/23-T175-piece-id` (#51), `001/24-T175-import-format` (#52). Test first: `ProposalId` is stable under unrelated renumbering and matches independently computed FNV-1a vectors, one per kind byte and per seam-kind byte, plus a multi-byte edge index to pin little-endian. *Added 2026-10-14: T028 referenced Phase 3 types (T069's `SeamKind`, charts, pieces), so Phase 2 could not compile without them* — *merged in #49, #50, #51, #52*
  - *2026-10-14 review follow-up, no task ID:* `ProposalId::from_hex` returns `SeamsError`, which `openplush-pipeline` did not re-export, so a shell could not name the error. `001/28-T175-seams-error-reexport` (#66) re-exports it and marks it `#[non_exhaustive]`
- [x] T177 `openplush-mesh` exports `id_newtype!`, generating the shared shape of the eight `u32` IDs (derives, `new`, `index`); `VertexId`, `FaceId`, `HalfEdgeId`, `EdgeId`, `ComponentId`, `ChartId`, `SeamChainId` and `PieceId` use it, `ProposalId` (a `u64`) does not. One PR per crate; `openplush-pattern` gains the direct dependency on `openplush-mesh` that the DAG already allows. Test first: a type defined through the macro round-trips, is `const`-constructible, orders by index and prints `Name(3)` under `Debug`, as `MeshError` messages do today. *Added 2026-10-15: the third copy of the same boilerplate (AGENTS.md, DRY); ID out of phase order* — *merged in #87 (mesh), #89 (seams), #90 (pattern)*
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
- [x] T040 `packages/engine-client` — worker host, typed protocol, and a **mock worker** implementing contracts/worker-protocol.md so shell tests need no WASM build. Needs T176 (its npm dependencies must pass the license gate) and T041 (its protocol types are generated, never hand-written; only the host's `{ crashed: true }` is typed by hand). Includes the crash rules of contracts/worker-protocol.md §4, tested with a mock that crashes on demand: N pending requests get N `EngineCrashed` responses and no second terminal message; a cancel racing the crash is a no-op; the next request runs on a new generation, where an earlier handle gets `StaleHandle`; `restarted` fires once. Writes the protocol's as-built page, `docs/reference/worker-protocol.md` — *merged in #101 and #102 (branches `001/55-T040-engine-client-requests`, `001/56-T040-engine-client-crash`). A success `Outcome` carries the generation that answered it, and the app builds `Issued` handles from that, so a crash between a result and its use cannot restamp a handle; the real `Worker` adapter is T099's*
- [x] T041 `scripts/gen-ts-types.sh` — generate TypeScript types from the Rust protocol definitions with `ts-rs` (dev-dependency only, [ADR 0010](../../docs/adr/0010-protocol-types.md)); CI fails on a diff in either direction, so protocol drift breaks the build rather than the pattern. Also fails on any `bigint` in the output (64-bit integers are banned from the protocol) and on one DTO's optional field not reading `field?:`. Needs T179 — *merged in #98 (branch `001/53-T041-ts-types`), run by `scripts/check.sh types` in the `engine` group; handles get their TypeScript brand from a `ts(type)` override pinned by a compile-time `u32` check*
- [x] T179 The binding's protocol layer, so T041 has Rust definitions to generate from and T040 has types to use. Two PRs, one per crate: (a) `openplush-pipeline` — `EngineError::code()` (the variant name), `EngineError::locations()` and `EngineError::proposals()`, each exhaustive inside the crate, so a new variant without a code, locations or proposals fails to compile rather than reaching the shell unlocated; (b) `bindings/openplush-wasm/src/protocol/` — the DTOs of contracts/worker-protocol.md §2, §4 (handles), §5 and §6, the binding's own code list (`StaleHandle`, and `EngineCrashed`, listed for the generator though only the host raises it), the conversion from `EngineError`, and the panic hook that forwards a panic's message to the worker console; §3 payload types come with the task that adds each operation. (b) needs two contract amendments, each with its ADR, written ahead of T169 (Phase 10) because (b) cannot start without them: how a panic reaches the shell (ADR 0009, #84), and how the DTOs are encoded and mirrored in TypeScript, including the source-vertex and proposal-ID locations §6 cannot carry today (ADR 0010, #85). (a) documents the new methods in `docs/reference/engine-api.md`. Test first: the code list contains `StaleHandle` and `EngineCrashed`; every variant's code; every variant but `Parse` (located by its format, which the message names) and `Cancelled` has locations, counting the source-vertex and proposal-ID categories ADR 0010 adds; each DTO serializes to the contract's JSON shape. *Added 2026-10-15: no task owned the binding's protocol types; ID out of phase order* — *merged in #88 (a), #94, #97 and #100 (b). #100 added the §2 envelope (`Request`, `Op`, `RequestId`, `Response`), which ADR 0010 counts among these DTOs; `WarningDto` is uninhabited until a warning reaches the shell*
- [x] T042 CI determinism lint — ban `HashMap` and `HashSet` iteration and unstable sorts on output-affecting paths (Constitution V), via clippy `disallowed_methods` plus a reviewed allowlist — *merged in #70; follow-up #73*

**Checkpoint**: foundation ready. US1 can start immediately; the other stories build on its stages.

---

## Phase 3: US1 — Seams in, printable pattern out (Priority: P1) 🎯 MVP

**Goal**: A maker imports a model, sets its real-world size, draws seams, and prints a true-scale pattern with seam allowances.

**Independent Test**: Run the cube, capsule, and sphere fixtures through import to print; measure printed pieces against expected dimensions.

This phase is large because US1 *is* the product. Every other story decorates this loop.

### Tests for US1 (write first, observe failing)

- [ ] T043 [P] [US1] `crates/openplush-pipeline/tests/contract_surface.rs` — the stage signatures in contracts/engine-api.md compile and are callable as documented. *2026-10-14: grows one signature at a time, since T031 no longer stubs them: each US1 stage's assertion lands in the PR that adds that stage's `openplush-pipeline` signature, T092 at the latest; other stories' signatures get theirs in those stories*
- [x] T044 [P] [US1] `crates/openplush-flatten/tests/developable.rs` — cylinder strip, cone, and folded flat sheet each flatten with distortion near zero. **The oracle that catches a flattener that is confidently wrong** — *merged in #211: a cylinder strip, a cone frustum strip and sheets folded once and twice, all at mm scale, flatten through `FlattenSession` with every singular value within 1e-9 of 1 (measured within 1e-13), checked by testkit's `injective`, `developable` and `area_conserved`*
- [ ] T045 [P] [US1] `crates/openplush-pattern/tests/parity.rs` — on every fixture, both sides of every seam match within max(1.0 mm, 0.5%). Hard assertion, not a tolerance report (D-013)
- [x] T046 [P] [US1] `crates/openplush-export/tests/scale.rs` — the scale-check square measures exactly 50.00 mm in the output file (SC-003) — *merged in #214: an independent PDF reader in the export tests measures each page's scale square at 50.00 mm and its inch square at 25.40 mm, on A4 and Letter*
- [ ] T047 [P] [US1] `bindings/openplush-cli/tests/end_to_end.rs` — capsule from import to SVG through the CLI path, matching a golden snapshot
- [x] T048 [P] [US1] `crates/openplush-io/tests/defect_corpus.rs` — every model in `fixtures/defects/` is detected and reported (SC-007, 100%) — *merged in #117; it lives in `openplush-io` because reading the fixtures as OBJ from `openplush-mesh` would be a crate cycle. Each defect fixture trips exactly its own detector; each clean fixture trips none*
- [ ] T049 [P] [US1] `crates/openplush-pipeline/tests/determinism.rs` — same input twice gives byte-identical export (SC-008)

### Import and model health (FR-001 … FR-006)

- [x] T050 [US1] `crates/openplush-io/src/obj/` — OBJ geometry and per-corner UVs, read by our own reader (plan D-034: `tobj` loses provenance and fabricates UVs). Source primitives split at `o`, `g` and `usemtl`. *2026-10-16: MTL and textures wait for a consumer (FR-043, v0.2); the primitive split keeps that addition out of `TriangleSoup` and repair* — *merged in #108 (branch `001/60-T050-T053-obj-import`) as `obj::read` in `src/obj/`; its errors are `IoError`, which the pipeline maps to `EngineError` in T092*
- [x] T051 [P] [US1] `crates/openplush-io/src/gltf/` — glTF 2.0 and GLB via `gltf`; skinned and animated data ignored with a warning. *2026-10-16: `.gltf` needs its separate buffers, which `import(bytes, …)` cannot carry; the import input shape is `ImportFiles` (D-038, contract 0.6); the implementing task changes `import`'s signature and the worker message with it* *2026-10-20: reader behaviour fixed by D-066* — *built as `gltf::read(name, bytes, sidecar)` in `src/gltf/`, wired into `import` for `gltf`/`glb` with `EngineError::MissingResource` and the `import-ignored` warning. The refusals D-066 (4) gained on 2026-10-20 are this reader's; accessors are read by its own checks, because the crate's reader trusts the declared component type*
- [ ] T190 [US1] needs:T051 `crates/openplush-io/src/fbx/` — binary FBX 7.x per D-066 (9): the transform stack and geometric transform, axis conversion to Y-up, one source primitive per (model, material) pair, UV layers, deformers and animation stacks listed for `import-ignored`, ASCII FBX refused as `Parse`; with it `Format::Fbx`, `fbx` in the import stage's extension table, the schema's `sourceFormat` `"fbx"`. Tests first, on hand-built binary files: a cube under a rotated, pivoted parent lands where the stack puts it; a Z-up file comes out Y-up; two materials on one mesh are two primitives; an ASCII file is `Parse`. *Added 2026-10-20 at the maintainer's revision of D-064 and D-066; ID out of phase order; needs T051 for the ignored list and the `import-ignored` warning*
- [x] T052 [P] [US1] `crates/openplush-io/src/{stl,ply}.rs` — STL and PLY (the minor default was yes) — *merged in #152 (`stl::read`) and #153 (`ply::read`), each our own reader; the PLY reader refuses polygons over 1000 corners*
- [x] T053 [US1] `crates/openplush-io/src/triangulate.rs` — polygons to triangles, preserving per-corner UVs and material assignment (FR-002). A NaN/∞ coordinate anywhere fails the import with `EngineError::NonFiniteInput` naming every non-finite source vertex (D-021); nothing is dropped — *merged in #108 with T050; material assignment rides on the OBJ primitive split (D-034). A self-crossing polygon is refused as `Parse`; a collinear one is split and left to validation*
- [x] T054 [US1] `crates/openplush-mesh/src/validate/nonmanifold.rs` — non-manifold edges and bowtie vertices, each located — *merged in #114. Topology is judged on the welded soup (weld cells in `coincident.rs`, at the weld distance), with repeated-index faces and later duplicate copies left out*
- [x] T055 [P] [US1] `crates/openplush-mesh/src/validate/degenerate.rs` — zero-area, duplicate, and repeated-index faces — *merged in #114; unwelded coincident vertices are reported too*
- [x] T056 [P] [US1] `crates/openplush-mesh/src/validate/topology.rs` — holes, boundary loops, connected components — *merged in #114; a loop through a repeated vertex is split there*
- [x] T057 [P] [US1] `crates/openplush-mesh/src/validate/orientation.rs` — inconsistent winding, non-orientable detection — *merged in #114, plus closed shells wound inside out, judged per shell*
- [x] T058 [US1] `crates/openplush-mesh/src/validate/selfintersect.rs` — BVH triangle-triangle tests. **Warn only** (data-model section 3 step 8) — *merged in #117. Floating point against the weld distance, not exact predicates; a fold at a shared vertex counts, edge-on and coplanar contact are not reported*
- [x] T059 [US1] `crates/openplush-mesh/src/validate/report.rs` — `ValidationReport` with every defect locatable in 3D (FR-003) — *merged in #114 as `validate(&TriangleSoup) -> ValidationReport`; it never fails: a soup with no triangles or an extent with no tolerances is reported `Unmeasurable`*
- [x] T060 [US1] `crates/openplush-mesh/src/repair/weld.rs` — repair step 1 (step 0 happens at import, D-021). **The weld is the one that matters most in practice**: glTF exporters split vertices at UV and normal discontinuities, so an unwelded import looks full of holes and nothing downstream works — *merged in #121 (branch `001/65-T060-weld-grouping`, validate's grouping moved here) and #122 (`001/66-T060-weld`) as `repair::weld::merge`. Unused and non-representative vertices stay in the soup; `into_manifold` (T067) compacts them*
- [x] T061 [US1] `crates/openplush-mesh/src/repair/degenerate.rs` — step 2, including the double-sided-sheet flag (D-036: caps are stitched by re-triangulating the faces across their longest edges with the caps' vertices; both sheet layers are kept) — *merged in #125 (`degenerate::{classify, remove}`) and #132 (`repair/cap/`)*
- [x] T180 [US1] `crates/openplush-mesh/src/validate/` — topology is judged with both layers of a double-sided sheet, as repair keeps them, reusing repair's degenerate step as validation reuses its weld (D-036). *Added 2026-10-17; ID out of phase order* — *merged in #134*
- [x] T062 [US1] `crates/openplush-mesh/src/repair/nonmanifold_edge.rs` — step 3, pair the faces on each over-shared edge per Guéziec et al. 2001 and return the pairings without rewriting the soup (D-037); the copies the build makes become boundary edges, which become mandatory seams. A double-sided sheet's interior edges pair within each layer (D-036) — *merged in #138 as `pair_edges`: the pairings are returned as data and the soup is not rewritten*
- [x] T063 [US1] `crates/openplush-mesh/src/repair/bowtie.rs` — step 4, split one vertex per fan — *merged in #140 as `bowtie::split`*
- [x] T181 [US1] `crates/openplush-mesh/src/halfedge/` — the build takes the pairings `pair_edges` returns and holds parallel edges: one twin pair per paired copy of an edge, a boundary copy for an unpaired face, `InconsistentOrientation` for a pair that traverses the edge the same way, same-vertex duplicate faces still refused unless paired (D-037). Tests first: a double-sided sheet builds with its rim copies; an odd fan leaves a boundary copy for the unpaired face; a same-way pair is `InconsistentOrientation`; a parallel edge without a pairing is still refused. *Added 2026-10-18; ID out of phase order; T067 needs it* — *merged in #164 as `ManifoldMesh::from_paired_triangles`; a pairing must name every face of an edge with more than two (`PairingMismatch`); an unpaired face needs T184's vertex split first*
- [x] T182 [US1] `crates/openplush-pattern/src/outline.rs` and `src/piece.rs` — `Outline` (validating constructor, typed error for a crossing, repeated, zero-area or clockwise outline) and `PatternPiece` with the fields data-model §7a lists (D-039). Tests first: each invalid outline is refused with its own error; a square passes; a clockwise square is refused, not reversed. *Added 2026-10-18; ID out of phase order; T088 needs it. Builds on Proposed D-039: reworked if the maintainer amends it* — *merged in #161 as `Outline::new` (in `src/outline.rs`) and `PatternPiece::new` (in `src/piece.rs`); both refuse bad input with `PatternError`*
- [x] T186 [US1] `crates/openplush-pattern/src/piece.rs` — `PatternPiece::new` refuses a `cut_quantity` over 100 with a typed error, beside the zero check (D-047). Tests first: 100 builds, 101 is refused. *Added 2026-10-18; ID out of phase order* — *merged in #174: `PatternError::TooManyCuts { cut_quantity }` above 100*
- [x] T187 [US1] `crates/openplush-pattern/src/piece.rs` — replace `cut_quantity` with `as_printed` and `flipped` (D-048); `PatternPiece::new` keeps the 1 to 100 rule on their total, as `TooManyCuts { total }`. Until dedupe (T116) exists, the pattern stage makes every piece 1 as printed, 0 flipped. A separate thin-wiring commit, after the pattern change, makes `openplush-layout`'s `shelf::pack` place `as_printed + flipped` copies so the build stays green; per-copy flipping is T188. Updates `docs/reference/crates.md`. Tests first: 1 + 1 builds, 0 + 0 and 60 + 41 are refused. *Added 2026-10-18; ID out of phase order* — *merged in #187 as `CutCount::new(as_printed, flipped)`, the 1 to 100 rule on the total (`TooManyCuts { total }`); `PatternPiece::new` takes a `CutCount` and `cuts()` replaces `cut_quantity()`*
- [x] T188 [US1] `crates/openplush-layout/src/` — two layouts from one piece list (D-048): print places one template per piece; fabric places `as_printed` copies plus `flipped` copies flipped, per copy, separate from the export-time pile mirroring, which stays `LayoutOptions::mirrored`. Needs T187. *Added 2026-10-18; ID out of phase order* — *merged in #188: `shelf::pack` is the fabric plan (flipped copies mirrored, the whole plan mirrored with the pile print) and `shelf::pack_templates` the print, one template per piece; both share one placer*
- [ ] T189 [US3] needs:T116 `crates/openplush-pipeline/src/warning/` — warning `near-mirror-pieces` from dedupe's near-match records (D-049): a `WarningTarget` variant naming both pieces, `Actionable` with the seam-mirroring proposal, built through `openplush-seams`, when the record says the pair's seams do not yet mirror each other, else `Advisory`; documented in `docs/reference/engine-api.md`. Needs T116 and the engine's seam-mirroring operation (FR-010). Tests first: each record becomes one warning with the right remedy, a pair is one target whatever its order, and an acknowledgement keyed by code and target matches it again. *Added 2026-10-18; ID out of phase order*
- [x] T064 [US1] `crates/openplush-mesh/src/repair/orientation.rs` — step 5, BFS per component from the largest face; signed volume positive when closed. Edges with more than two faces and the copies a pairing makes are not walked (D-037) — *merged in #142 as `orientation::orient`: conflict edges are returned for T067 to make seams*
- [x] T065 [US1] `crates/openplush-mesh/src/repair/components.rs` — step 6; components under 1% of total area become candidate detail parts (FR-005) — *merged in #148 as `components::split`*
- [x] T183 [US1] `crates/openplush-mesh/src/repair/plan.rs` — `RepairPlan` and `RepairReport` as data-model §3a defines them (the plan's fields are the schema's), with the report's accessors; no repair logic. Tests first: the plan's defaults and loop order, a report lists every field empty by default. *Added 2026-10-18 (D-043); ID out of phase order; T066 and T067 need it* — *merged in #162; the plan has plain `pub` fields (no combination is invalid), and the round-trip test belongs to `openplush-project`, which serializes it*
- [x] T066 [US1] `crates/openplush-mesh/src/repair/holes.rs` — step 7, classify each boundary loop by `RepairPlan.fill_holes` (an opening unless listed, D-043); fill the listed ones with Liepa 2003 triangulation (lowest maximum dihedral angle, then area), refinement and fairing (D-065). Needs T183. Two PRs, as the split table says: classification, then the fill. *2026-10-20: classification built as `repair::holes::classify(mesh, plan)`: every boundary loop named by its lowest source corner, with its half-edges and whether the plan lists it (accidental); a listed loop the mesh lacks is `UnknownHoleLoop`, lowest first; an accidental loop was refused with `HoleFillUnavailable` until the fill landed; D-065 fixes the fill: Liepa's weight, refinement, bilaplacian fairing with `faer`, the patch appended without provenance, and `HoleNotFillable`, `HoleOnSplit` and `HoleTooLarge` in place of `HoleFillUnavailable`* — *fill built in `repair/fill/` (triangulate, refine, fair) and `halfedge/append.rs` as D-065 fixes it; the fairing's sparse Cholesky is a second small `faer` wrapper beside `openplush-flatten`'s solver, which one shared home could later replace*
- [x] T184 [US1] `crates/openplush-mesh/src/repair/bowtie.rs` — `split(soup, pairings)` takes the pairings `pair_edges` returned (as `&[PairedEdge]`, in the split's own soup's face IDs) and joins fans across a pair, not across the whole over-shared edge, so a face left unpaired, or a pair whose ends are not rejoined round the vertex, gets its own vertex copies before the build (D-037; found in T181). Tests first: a face left unpaired on a three-face edge is split off at both ends; the two pairs of a four-face edge split off at both ends when nothing rejoins them; a double-sided sheet's rim vertices stay single and its interior vertices split per layer; with no pairings, behaviour is unchanged. *Added 2026-10-18; ID out of phase order; T067 needs it* — *merged in #168 as `bowtie::split(soup, pairings)`; its pairings are stale once it copies a vertex, so T067 pairs the split soup again*
- [x] T185 [US1] `crates/openplush-mesh/src/halfedge/build.rs`, `repair/pipeline.rs` — a non-orientable conflict edge becomes a forced seam (data-model step 5, D-044) instead of `InconsistentOrientation`: the build takes the conflict edges and allows a deliberate cut of a two-face edge (today `PairingMismatch` refuses a pairing on an edge with two or fewer faces), and `into_manifold` fills `RepairReport::orientation_conflicts` and adds the edges to `mandatory_seams`. Each edge it or a step-3 split copies is recorded as a copy group, the `EdgeId`s of its copies in the repaired mesh (D-060), so `SewingMap` can join the chains of a group into one bundle instead of refusing them. Tests first: a twisted ring builds as an open strip whose cut edge is a boundary and a seam; a cut on an ordinary manifold edge without a conflict is still refused. *Added 2026-10-18; ID out of phase order; found in T067* — *2026-10-19: built per D-044 and D-060: `orient` runs before the bowtie split and returns each conflict with its two faces; the split gives each face of a conflict edge its own fan; the build takes `conflicts` and cuts a two-face edge the split left shared into two boundary copies (`cut_copies`), refusing anything else with `PairingMismatch`; the pipeline tracks every split edge by one face per copy and resolves them to `RepairReport::edge_copies` after the build, lists conflicts in `orientation_conflicts` and adds both copies to `mandatory_seams`. The twisted ring builds as a 7-vertex strip with one boundary loop*
- [x] T067 [US1] `crates/openplush-mesh/src/repair/pipeline.rs` — `into_manifold()` returning `(ManifoldMesh, RepairReport)`. **Typestate gate**: `ManifoldMesh` gets no public constructor, so unrepaired data cannot reach the flattener by construction (FR-004, Constitution VII). Moves the soup's `SourceMaterials` and `extra_uv_sets` onto the mesh unchanged (003 S-011). Needs T181 and T184 (D-037) and T183 (D-043); pairs again after the split (its pairings go stale once it copies a vertex), and passes the build the double-sided-sheet face pairs, found by `degenerate::classify` on the stitched soup: passes `pair_edges`' pairings, mapped through `Removed::origin` and `Stitched::origin`, to the build — *merged in #170 as `openplush_mesh::into_manifold`; a non-orientable conflict edge was refused as `InconsistentOrientation` until T185 cut it (superseded there), and a `fill_holes` entry was refused until T066 built the fill*
- [x] T068 [US1] `crates/openplush-pattern/src/scale.rs` — real-world size from a dimension or measured reference distance; warn, never block, outside 75 mm to 1000 mm (FR-006, D-015) *2026-10-18: D-040 accepted: input is the longest finished dimension or one measured reference distance; returns `Scale` and `SizeRange`; the warning is raised in pipeline* — *merged in #176 as `scale::size(model_longest, target)` returning `(Scale, SizeRange)`; `SizeTarget` is `LongestDimension(Mm)` or `Reference { model_length, finished }`; refuses `BadModelLength` and `BadScale`*

### Seams and charts (FR-010 … FR-014)

- [x] T069 [US1] `crates/openplush-seams/src/set.rs` — `SeamSet` as non-destructive `BTreeMap<EdgeId, SeamKind>` (`SeamKind` itself comes from T175); the mesh never changes when seams change — *merged in #149 (`SeamSet`: a `Mandatory` seam is never removed or changed and is placed only on an edge with no seam; `SeamsError::MandatorySeam`)*
- [x] T070 [US1] `crates/openplush-seams/src/graph.rs` — seam graph node classification and maximal seam chains (the unit of notching, labelling, and parity) — *merged in #225 as `SeamGraph::new(mesh, seams)` per D-052: nodes (`Junction`, `BoundaryEnd`, `DartTip`, `KindChange`, first that fits), maximal chains in ID order with D-052's walking order, closed rings, and `SeamsError::UnknownEdge`; checked on generated sheets and tubes by an independent well-formedness oracle; D-052 (6) resolved by D-058, built in T074*
- [x] T071 [US1] `crates/openplush-seams/src/path.rs` — shortest surface path between two picked points, for seam drawing (FR-010) — *2026-10-19: built as `shortest_path(mesh, from, to)`: Dijkstra over mesh edges by 3D length between two picked vertices, edges in walking order, deterministic among equal walks; `UnknownVertex` and `NoPath` refusals. Checked against a Floyd–Warshall oracle on every pair of a sheet*
- [x] T072 [US1] `crates/openplush-seams/src/dart.rs` — darts as seams ending inside a piece; dart tips stay single vertices, a slit rather than a separation (FR-011) — *2026-10-19: built as `mark_dart(mesh, seams, edges)`: a walk from an anchored base (a seam or the mesh boundary) to a tip that touches neither becomes `Dart` edges, either direction of drawing; a walk anchored at neither end, reaching a seam or boundary past its base, not joined, or looping is refused with a typed error and nothing changes. The slit itself (tip one vertex, piece still one disk) is T070's `DartTip` and T073's cut, checked here end to end*
- [x] T073 [US1] `crates/openplush-seams/src/chart.rs` — flood fill across non-seam edges, duplicating vertices along interior seam edges; provenance maps back to global IDs *2026-10-19: the flood fill merged in #228 as `charts(mesh, seams)`, charts as face sets numbered by lowest face; the cut-open chart built per D-054 as `ChartMesh::new(mesh, seams, chart)`: flat arrays shaped like flatten's input (`xyz`, `faces` with local corners, one provenance map `vtx_to_global`), one local vertex per fan between cuts, numbered by first appearance*
- [x] T074 [US1] `crates/openplush-seams/src/sewing_map.rs` — `SewingMap` with `k >= 2` sides per chain. v0.1 exposes `k = 2`; the model supports more from day one so FR-016 is not a migration later. A chain on mesh-boundary edges joins a bundle only through repair's recorded copy groups (D-058, D-060, recorded by T185); until then, and for a boundary seam with no partner, a typed refusal — *2026-10-19: built as `SewingMap::new(mesh, seams, graph)`: one `SeamBundle` per chain in chain order, each with `sides: Vec<ChainSide>` (a chart and its half-edges along the chain in walking order; the side running with the walk first, its twins second), so a third or fourth side is a push, not a migration; a chain along mesh-boundary edges is refused with `SeamsError::UnpairedBoundarySeam`. The join of repair's recorded copy groups (D-060) is finished after T185: `SewingMap::new(mesh, seams, graph, groups)` with `SeamGraph::with_copies` (D-061) joins each copy's chain into the lead's bundle (fin k = 3, sandwich k = 4, cut k = 2) or refuses a mismatch*
- [x] T075 [US1] `crates/openplush-seams/src/topology.rs` — Euler characteristic check; flattenable iff genus 0 with exactly one boundary loop — *merged in #229 as `check_disk(mesh, seams, chart)`: counts the chart cut open along its seams without building it; `PositiveGenus` whenever it has handles, else `Closed` or `MultipleBoundaries`; checked on sheets, tubes, a torus and an octahedron against hand-worked topology*
- [x] T174 [US1] `crates/openplush-seams/src/fold.rs` — refuse a **folded chart** before flattening: adjacent faces whose normals are within a new named angle tolerance (`FOLD_ANGLE`, added to data-model §11 and `openplush_geom::epsilon` by this task) of antiparallel across a shared edge, i.e. a surface lying back over itself. A typed error naming the chart and the edge, upstream of T079–T080; never left for the flattener to discover as an injectivity failure. Needs an `EngineError` entry (contracts/engine-api.md §5) agreed in this task's PR. Test first with S7's folded synthetic caps (`research/s7_boundary_pinned/`). *Added 2026-10-14 from S7 finding 3* — *2026-10-19: built as `check_fold(mesh, seams, chart)` per D-059: `FOLD_ANGLE = 5°` in `openplush_geom::epsilon`; an uncut edge whose two chart faces have normals within it of antiparallel is a fold, refused with `SeamsError::FoldedChart { chart, edge }` naming the lowest such edge; `EngineError::FoldedChart` added. Checked against S7-style caps rebuilt in Rust (175° passes, 190° refused where an independent dihedral oracle finds the fold) and a two-triangle book at 2° and 10°*
- [x] T076 [US1] `crates/openplush-seams/src/proposals.rs` — minimal extra cuts for each failure mode in data-model section 4: closed region, multiple boundaries, positive genus. Returned **inside** `EngineError::ChartNotDisk` so the error is the suggestion (FR-013) *2026-10-19: merged in #236 as `close_chart(mesh, seams, chart) -> Result<Option<Proposal>, SeamsError>` per D-055: one `CloseChart` per failing chart covering every failure it has, checked to leave a disk or refused with `SeamsError::ChartNotClosed`; the `EngineError::ChartNotDisk` mapping comes with T092*
- [x] T077 [US1] `crates/openplush-seams/src/opening.rs` — mark any chain as the stuffing opening (FR-012) — *2026-10-19: built as `mark_opening(seams, chain)`: every edge of the chain becomes `SeamKind::Opening`, or nothing changes and a mandatory edge is refused with `MandatorySeam`, an edge with no seam with the new `SeamsError::NotASeam`*

### Flattening (FR-020, FR-023, FR-026)

- [x] T078 [US1] `crates/openplush-flatten/src/solver.rs` — `faer` integration with `default-features = false`, since the default features pull in `rayon` and break the WASM target *2026-10-18: S2a (#183) passed every gate but `cargo deny` advisories (`paste`, RUSTSEC-2024-0436); D-050 accepts an ignore for it, added to `deny.toml` by this task; features `sparse-linalg` only (no `std`)* — *merged in #193 as the crate-private `solver` module (`solver/mod.rs`): `analyse` once per sparsity pattern, `Pattern::factor` per set of values, `Factor::solve`; refusals are `SolveError`, mapped to `FlattenError` by the first caller*
- [x] T079 [US1] `crates/openplush-flatten/src/boundary.rs` — **prescribe boundary edge lengths to their 3D lengths** (D-013, BFF-style). This is what makes seam parity exact without any cross-chart coupling: both sides of a seam come from the same 3D edge, so pinning each chart independently makes them agree — *merged as the crate-private `boundary` module in three slices: #194 (`closure`: close the polygon by turning angles alone, lengths fixed), #195 (`topology`: the one boundary loop, refusing non-disks), #197 (`curvature` and `prescribe`: BFF turning with boundary scale factors at zero, solved with T078's solver); refusals are `BoundaryError`, mapped to `FlattenError` by T080*
- [x] T080 [US1] `crates/openplush-flatten/src/interior.rs` — SLIM symmetric-Dirichlet minimisation with the boundary held fixed and a flip-preventing line search *2026-10-18: D-051 fixes the start (cotangent fill, uniform fallback, `Flip` if both flip), the stopping rule (relative energy change 1e-9 or 200 iterations) and the new `FlattenError` variants; the energy and step bound landed in #199* — *merged in #199 (energy and step bound) and #201 (`interior::Solve`: harmonic start, SLIM iterations, line search, stop rule, `FlattenError` mapping); crate-private until `FlattenSession` (T083) calls it. Oracles: half-cylinder 4.000, S7 `cap-90` hemisphere 5.2001*
- [x] T081 [US1] `crates/openplush-flatten/src/injectivity.rs` — verify det J > 0 on every face; return `EngineError::Flip` with the offending faces. Constitution III post-flatten invariant — *merged in #154 as `injectivity::verify` over plain `(FaceId, [DVec2; 3])` triangles, returning `FlattenError::Flip`; the `EngineError` mapping comes with T092*
- [x] T082 [US1] `crates/openplush-flatten/src/fallback.rs` — when the prescribed boundary cannot close, return `BoundaryNotRealizable` plus a dart proposal. **Never relax parity to escape this** — that is the one thing the fallback must not do, and S7 (T007) sizes how often it fires — *merged in #204 as the crate-private `fallback::not_realizable`: `BoundaryNotRealizable { chart, residual, dart }`, the residual being the closing gap at the interior's own turning and the dart the shortest edge path from the rim to the interior vertex with the most angle defect (`None` without an interior vertex); called by `interior` when closure fails. Parity is never relaxed*
- [x] T083 [US1] `crates/openplush-flatten/src/session.rs` — `FlattenSession::step()` so the single-threaded WASM worker can yield, report progress, and honour cancellation without ever exposing a half-flattened result — *merged in #205 as the public `FlattenSession` (`new`, `step() -> Progress { Working { iteration, energy } | Done }`, `finish() -> Option<layout>`, `None` until `Done`); cancelling is dropping the session, so there is no cancel method*

### Pattern, layout, export (FR-030, FR-031, FR-060, FR-061)

- [ ] T084 [US1] needs:T008 `crates/openplush-pattern/src/fabric.rs` — the `Fabric` entity from data-model section 6a and the presets from research R7, shipped as data. Derived values (rotation set, mirrored printing, pile-scaled allowance) are computed, never stored *2026-10-18: D-042: two PRs, the type first and the presets after review (S8)* *2026-10-18: D-042 accepted; the presets PR carries fleece, minky and long faux fur with measured values, the other four follow as measured (D-046)* *2026-10-18: the type merged in #177 (`Fabric::new(FabricSpec)`, `rotations_deg()`, `asks_mirroring()`); the presets PR waits for measured values*
- [x] T085 [US1] `crates/openplush-pattern/src/allowance.rs` — per-edge allowance as a union of per-segment strips via `i_overlay`, not a single-distance offset, because seam, opening, and hem allowances differ along one outline (FR-030). Defensiveness level set by S3 (T003): [ADR 0006](../../docs/adr/0006-allowance-construction.md) *2026-10-18: ADR 0006 accepted* — *merged in #213 (strips, fold guards, the short-edge merge, one `i64` boolean, the typed refusals `AllowanceCount`, `FoldConflict`, `AllowanceSplitsPiece`, `AllowanceEnclosesGap`, `AllowanceNotSimple`), #215 (miter or bevel joins at the 4× limit) and #217 (S3's oracle and 1000 seeded cases, every accepted cut line matching the reference) as the public `allowance::cut_line(piece, sewing, &[Mm])`. FR-057's fraying rule for zero allowance is T152's*
- [x] T086 [US1] `crates/openplush-pattern/src/lines.rs` — sewing line and cut line drawn distinctly (FR-031) — *done by the exporters, not a pattern module: `svg::write` (#182) and `pdf::write` (#214) draw each copy's cut line solid at 0.5 mm and its sewing line dashed `3 2` at 0.3 mm, proved by `tests/svg.rs` and `tests/pdf.rs`*
- [x] T087 [US1] `crates/openplush-pattern/src/parity.rs` — assert parity within max(1.0 mm, 0.5%) and return `EngineError::SeamParity` otherwise. Should be unreachable; it exists because a silently mismatched seam is the failure Constitution III calls worse than a crash — *merged in #155 as `parity::check` over two `Mm` side lengths, returning `PatternError::SeamParity`; the `EngineError` mapping comes with T092*
- [x] T088 [US1] `crates/openplush-layout/src/shelf.rs` — simple shelf packing for v0.1 (optimised nesting is v0.2, FR-062) *2026-10-18: needs T182; defines `Sheet` and `Placement` in `openplush-layout` (D-039)* *2026-10-18: D-039 accepted as amended: the maker's export-time mirroring answer reaches layout as `LayoutOptions`' `mirrored`, which sets `Placement::mirrored`; no change to the merged code* — *merged in #166 as `shelf::pack`, with `Sheet`, `Placement` and `LayoutOptions`; a rotation that is not a multiple of 90 is refused (`UnsupportedRotation`) until a deterministic rotation exists*
- [x] T089 [US1] `crates/openplush-layout/src/pages.rs` — tile for Letter and A4 with overlap, registration marks, and a scale-check square on every page, giving export what the cover's page map needs (FR-061) *2026-10-18: D-045: overlap without trimming, 15 mm band on right and bottom inside a 10 mm margin, matching crosshairs in the band, row-letter column-number labels, page map on page 1 (moved to a cover page by D-053), 50 mm square with a 1 in square inside* — *merged in #175 (tiling), #179 (rows run down from the sheet's top edge, since sheets are y-up), #178 (labels and registration marks) and #181 (scale square) as `pages::tile(sheet, paper)`; `TooManyPages` over 26 pages either way; the page map needs no layout output beyond `PageTiling`, so export draws it*
- [x] T090 [US1] `crates/openplush-export/src/svg.rs` — SVG at true scale, document dimensions in millimetres, 1 user unit = 1 mm (FR-060) *2026-10-18: needs T088 and an accepted ADR 0012 (export reads layout)* *2026-10-18: ADR 0012 accepted; this task adds the edge to `scripts/check-boundary.sh`* — *merged in #180 (`Placement::apply`, the one placement transform) and #182 as `svg::write(sheet, pieces)`, one sheet per SVG with cut and sewing lines; `ExportError`: `UnknownPiece`, `DuplicatePiece`; no golden snapshot (T033 is the maintainer's)*
- [x] T091 [US1] `crates/openplush-export/src/pdf.rs` — tiled PDF, approach chosen by S5 (T005): direct `pdf-writer`, [ADR 0008](../../docs/adr/0008-pdf-export.md). Browsers print SVG at unreliable scale, which is why PDF is in v0.1 at all *2026-10-18: ADR 0008 accepted* — *2026-10-19: pages, outlines, scale squares (#214), crosshairs and labels (#216) merged as the public `pdf::write(sheet, pieces, tiling)`; the cover page (D-053, accepted 2026-10-19) with the scale squares and the page map merged in #226, tiles from page 2; and the cover's warning summary, rows of text per D-056, built with T159 as `pdf::write(sheet, pieces, tiling, summary)`*
- [ ] T092 [US1] `crates/openplush-pipeline/src/stages.rs` — wire every stage; memoise by content hash so a seam edit re-runs chart and flatten only for affected charts (FR-014), and a scale or shrink change re-runs pattern onward only; maps each stage error (`FlattenError`, `PatternError`, …) to `EngineError` *2026-10-18: D-039 review: names the stage-error to `EngineError` mapping that T081 and T087 defer here* *2026-10-20: D-062 fixes the `chart` stage: `ManifoldMesh::edge_copies()` and `EngineError::Seams`; the `chart` stage is built in #268, with T043's first assertion; D-063 fixes `validate` and `repair`: `EngineError::Mesh`, `NotManifold` removed; `validate` and `repair` are built in #270; D-064 fixes `import`: the reader by extension, `UnknownFormat`, no options; `import` is built for OBJ, STL and PLY (#303), and the worker's `import` message no longer carries `ImportOptions`; the memoisation and the other stages are not built; D-068 fixes the `flatten` stage; its slice also rewrites contract §2's `flatten(charts, opts)` and §5's `residual_mm` to match, with every other use of `residual_mm` and its message (the pipeline's error, its tests, the wasm protocol test, `docs/reference/engine-api.md`); the `flatten` stage is built, with D-068's mapping and contract 0.7; D-069 fixes the `Pipeline` session: its slice adds `crates/openplush-pipeline/src/session.rs` with the memo and stepping, makes `openplush-flatten`'s `FlattenSession` own its chart's arrays, rewrites `contracts/engine-api.md` §3 and §4's session and stepping and `contracts/worker-protocol.md`'s `flattenStart` row (no `FlattenOptions`) to match, with the contract's version bump and changelog line, and tests that a layout reused from the memo is byte-identical to one recomputed; the proposal methods are T107's*
- [ ] T093 [US1] `bindings/openplush-cli/src/commands/` — `validate`, `repair`, and `pattern` subcommands exactly as quickstart.md documents (Constitution II) *2026-10-20: `validate` and `repair`'s inputs, outputs and flags fixed by D-070; that slice needs T092's import slice (`ImportFiles` and the `import` stage) on `main`, and parses `--fill`'s loop-ref text as D-067 writes it; `validate` and `repair` are built as D-070 says (#344), and `pattern` waits on its stage*

### Web shell for US1

- [ ] T094 [US1] `apps/web/src/viewer/` — Three.js scene, model load, orbit controls
- [ ] T095 [US1] `apps/web/src/scale/` — size entry, unit selection, human-silhouette reference (FR-006)
- [ ] T096 [US1] `apps/web/src/health/` — defect list with 3D highlight; nothing changes until the maker confirms (FR-004)
- [ ] T097 [US1] `apps/web/src/seams/` — seam drawing with `three-mesh-bvh` edge picking, plus the optional symmetry-mirroring mode
- [ ] T098 [US1] `apps/web/src/export/` — page size choice, export, download
- [ ] T099 [US1] `packages/engine-client/src/worker.ts` — the real worker host with transferable buffers, replacing the mock for integration; wraps every export call so any exception is a crash (contracts/worker-protocol.md §4) — *first slice merged in #105 (branch `001/58-T099-worker-host`): `fromWorker` and the worker script's `serve`. Open: the wasm entry file and default spawn, waiting for the binding's first exported op, and transfer buffers, waiting for an op that returns geometry*

**Checkpoint**: US1 is independently functional. A maker can import, seam, and print a true-scale pattern. This is the MVP.

---

## Phase 4: US2 — See where the pattern will fail (Priority: P1)

**Goal**: Before cutting, the maker sees where fabric would stretch, pucker, or not fit, and gets concrete suggestions.

**Independent Test**: Sphere fixture with one seam shows high distortion near the poles and suggests darts there.

### Tests for US2

- [x] T100 [P] [US2] `crates/openplush-flatten/tests/metrics.rs` — singular-value metrics on a known-stretched fixture match hand-computed values — *merged in #207*
- [ ] T101 [P] [US2] `crates/openplush-seams/tests/dart_suggestion.rs` — sphere with one seam: suggestions cluster at the poles, and applying one measurably lowers distortion
- [ ] T102 [P] [US2] `apps/web/tests/heatmap.spec.ts` — the heatmap palette is color-blind-safe and identical between the 3D and 2D views (FR-083)

### Implementation

- [x] T103 [US2] `crates/openplush-flatten/src/metrics.rs` — per-triangle Jacobian singular values: max stretch, max compression, area ratio, conformal ratio. Interpreted against the active fabric's tolerances, not fixed constants — *merged in #207 as the public `metrics::distortion(chart, xyz, faces, uv) -> Vec<(FaceId, Distortion)>` (stretch σ₁, compression σ₂, area ratio, conformal ratio), `uv` dense and indexed like `xyz`; refusals are `EmptyChart`, `Degenerate`, and `Flip` naming every det J ≤ 0 face. Raw values only: tolerances stay the fabric's*
- [x] T104 [US2] `crates/openplush-flatten/src/summary.rs` — numeric per-piece summary (FR-021) — *merged in #209 as the public `summary::summarize(chart, xyz, faces, uv) -> Summary`: the most stretched and most compressed face with its value, the σ₁ and σ₂ the worst 5% of the 3D area reaches, the area-weighted stretch excess and compression deficit, and the total area ratio; refusals are `metrics::distortion`'s. Raw values only*
- [ ] T105 [US2] `crates/openplush-seams/src/suggest_dart.rs` — the four-step MVP algorithm from data-model section 5: cluster over-tolerance faces, find the max angle-defect vertex, propose a dart from the nearest boundary toward that cone point, accept for display only if distortion drops by the threshold
- [ ] T106 [US2] `crates/openplush-seams/src/concavity.rs` — warn when a concave region has no seam through it, with an `Actionable` remedy proposing a seam along the crease (FR-025). **This is the warning that matters most for a plush**: stuffing pushes an unsupported concavity open and the shape is simply lost
- [ ] T107 [US2] `crates/openplush-pipeline/src/proposals.rs` — every proposal is preview-then-accept, applied only on acceptance (FR-022, Constitution VII) *2026-10-20: offered, preview and apply fixed by D-069; this task adds `proposals`, `preview` and `apply_proposal` to the session and rewrites contract §4's proposal methods and the worker's `proposals` and `applyProposal` rows to match*
- [ ] T108 [US2] `apps/web/src/quality/` — heatmap on both the 3D model and the flat pieces, with a viridis or cividis palette and a per-piece summary panel
- [ ] T109 [US2] `apps/web/src/quality/suggestions.tsx` — suggestion list, preview on hover, apply on accept

**Checkpoint**: US1 and US2 both work. The maker can now see and fix problems before cutting fabric.

---

## Phase 5: US3 — Sewing marks that make assembly unambiguous (Priority: P2)

**Goal**: Pieces carry match notches, partner labels, cut counts as printed and flipped, and a marked stuffing opening.

**Independent Test**: For every seam in the capsule fixture, both sides carry matching notch codes at equal arc-length positions.

### Tests for US3

- [ ] T110 [P] [US3] `crates/openplush-pattern/tests/notches.rs` — paired notches sit at the same fractional arc length on both sides of every chain
- [ ] T111 [P] [US3] `crates/openplush-pattern/tests/dedupe.rs` — a symmetric model yields one template labelled "cut 2 (1 + 1 flipped)", not two identical pieces; a near match, a pair whose markings or grain differ, or a mirror pair in two fabric entries with identical properties, stays two templates; the near match is reported as a near-match record and the fabric pair as nothing
- [ ] T112 [P] [US3] `crates/openplush-pattern/tests/clip_marks.rs` — tight concave curves get clip marks, tight convex curves get notch-out marks, and zero-allowance edges get neither

### Implementation

- [ ] T113 [US3] `crates/openplush-pattern/src/notches.rs` — notches at chain ends plus every 50 mm (provisional); side B placed at the same fractional arc length as side A
- [ ] T114 [US3] `crates/openplush-pattern/src/pairing.rs` — unambiguous pairing codes. Notch multiplicity differs between adjacent chains on the same piece, so a sewist cannot rotate a piece and match the wrong edge. **The whole point is that a wrong match must be impossible, not merely unlikely**
- [ ] T115 [US3] `crates/openplush-pattern/src/labels.rs` — piece label block: ID, name, cut counts ("Cut 2 (1 + 1 flipped)", D-048), fabric, grain and nap arrows, seam-partner labels (FR-033)
- [ ] T116 [US3] `crates/openplush-pattern/src/dedupe.rs` — identical and mirror-image pieces collapse to one template (FR-034) *2026-10-18: D-048: fills `as_printed` and `flipped`; merges only when fabric, nap, seam partners, markings and grain arrows match through the mirror and every seam is within seam-parity tolerance; a near match is never merged: the maker sees its largest difference in mm and is offered seam mirroring (FR-010), else the pieces stay separate; the question's type and where the answer is saved were open in D-048 and are resolved by D-049* *2026-10-18: D-049: returns a near-match record (both `PieceId`s, the difference in mm, whether their seams already mirror each other) for a pair above parity tolerance and at most 5 mm apart; the pipeline raises the warning (T189); "same fabric" means the same fabric entry, and different entries never merge or produce a record*
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
- [x] T122 [P] [US7] `crates/openplush-project/tests/provenance_stability.rs` — **change repair settings and confirm seams still land on the same edges.** *2026-10-20, D-067: every step toggle but the weld and `drop_degenerates` keeps every seam on its edges; those two may change the faces that carry a key, and the test then expects the key to resolve to nothing, never to a wrong edge; un-listing a loop in `fill_holes` drops its patch, so a key on a patch edge resolves to nothing, as D-067 (5) says, which T191 tests; the `seam-unresolved` warning is the pipeline's and is checked with T124.* This is the test that protects the provenance-key design; if it fails, reopening a project silently moves the maker's seams
- [ ] T123 [P] [US7] `crates/openplush-project/tests/migration.rs` — a v1 document opens in a later engine unchanged

### Implementation

- [ ] T124 [US7] `crates/openplush-project/src/document.rs` — the document type matching the schema; re-export the public document API from `openplush-pipeline` as `openplush_pipeline::project` (D-020); bring project load into the pipeline, which raises `seam-unresolved` and `marking-unresolved` for each key `openplush-project` returns unresolved, adding both codes to the warning enum (D-067 (4))
- [x] T125 [US7] `crates/openplush-project/src/provenance.rs` — stable keys from source provenance: sorted source-vertex pairs for edges, sorted source-corner triples for faces. Never `EdgeId`, which moves when repair settings change *2026-10-20: key form and resolution fixed by D-067; this task also edits data-model §12 and the schema's `provenanceEdge`, `provenanceFace` and `boundaryLoopRef` descriptions to match* — *built as `EdgeKey`, `FaceKey` and `Provenance` (`edge_key`, `resolve_edge`, `face_key`, `resolve_face`); a face key resolves to every face with those corners, both layers of a double-sided sheet included. A hole patch's `<loop>+<n>` keys are T191, after T066's fill makes the first corners without a source; `LoopRef`'s text with the document's serde (T126)*
- [x] T191 [US7] needs:T066 `crates/openplush-project/src/provenance.rs` — hole-patch keys per D-067 (5): patch vertex `n`, in the patch's append order, is `<loop>+<n>`: a filled loop's patch vertices are the vertices of its `HoleReport::added_faces` that are not on its rim, appended after every vertex before them, so their `VertexId` order is the append order; edge and face keys over patch corners use that form, and the schema's `provenanceEdge`, `provenanceFace` and `boundaryLoopRef` descriptions gain it. Tests first: a filled hole's patch edge and face keys round-trip through their text and resolve to the same elements when the same plan is repaired again; a key whose loop is not filled under the new plan resolves to nothing. *Added 2026-10-20 at the maintainer's word: D-067 (5) had no task* — *built as `VertexKey`'s `Patch` and `Provenance::new(mesh, holes)`; `EdgeKey` and `FaceKey` now hold `VertexKey`s, and a patch corner takes its vertex's key*
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
- [ ] T149 [US6] `crates/openplush-pattern/src/pile.rs` — pile-scaled default allowance, and the small-piece and tight-curve warnings at the 2× multiple (FR-055, D-046). At 30 mm pile this warns on any piece or detail under 60 mm; the multiple stays a project setting
- [ ] T150 [US6] `crates/openplush-pattern/src/mirror_print.rs` — for pile fabrics export asks every time, with no default, whether to print mirrored with a "trace on wrong side" label or as-is with a "lay face down on wrong side" label (FR-054, D-046)
- [ ] T151 [US6] `crates/openplush-pattern/src/stretch_mismatch.rs` — compare per-direction stretch across each shared seam **after** grain angles are known, warn past the 20-point threshold, worded as millimetres over a 100 mm span (FR-058, D-046). Evaluated post-orient because two pieces of the same knit cut at 90 degrees to each other mismatch as badly as two different fabrics
- [ ] T152 [US6] `crates/openplush-pattern/src/zero_allowance.rs` — zero allowance for edge-stitched and appliqué pieces on non-fraying fabric; such pieces are excluded from clip marking since there is no allowance to clip (FR-057)
- [ ] T153 [US6] `crates/openplush-layout/src/rotations.rs` — honour the per-fabric rotation set and mirrored-copy reflection axis (FR-053)
- [x] T154 [US6] `crates/openplush-layout/src/roll.rs` — layout within a given fabric roll width, reporting the fabric length needed (FR-062) — *done by T088's shelf packing (#166), not a separate module: `shelf::pack` places pieces within `LayoutOptions`' roll width, refuses a piece wider than the roll, and `Sheet::length_mm` is the fabric length needed, proved by `tests/shelf.rs`*
- [ ] T155 [US6] `apps/web/src/fabric/` — fabric picker with the shipped presets, profile editor, nap-direction strokes, grain and nap arrow display

**Checkpoint**: all v0.1 user stories are independently functional.

---

## Phase 9: Warning plumbing and export gating (cross-cutting)

**Purpose**: FR-064 and FR-065 span every stage, so they land once the warnings they carry exist. Doing this earlier means plumbing with nothing to plumb; later means retrofitting every stage.

- [ ] T156 [P] `crates/openplush-pipeline/tests/warning_flow.rs` — a warning raised at any stage reaches export intact, with its remedy
- [ ] T157 [P] `crates/openplush-export/tests/warning_page.rs` — unresolved warnings appear in the first-page summary, and the summary is part of the golden snapshot so a warning silently vanishing from the printed page is a test failure
- [ ] T158 `crates/openplush-pipeline/src/warning_flow.rs` — accumulate `WarningSet` across stages, deterministically ordered
- [x] T159 `crates/openplush-export/src/warning_summary.rs` — render the first-page block from rows of text, code, location, explanation, remedy, that the pipeline renders from its `WarningSet` (FR-064, D-056) — *2026-10-19: built as `SummaryRow` and `pdf::write`'s `summary`: Courier lines under a shrunk page map that keeps half its room, overflow on pages after the last tile per D-056 (4); text outside WinAnsi refused with `ExportError::Unprintable`. The pipeline's rows from its `WarningSet` come with T092*
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
- [ ] T171 human:recruiting SC-004 usability test: five or more first-time makers, prepared model, printable pattern within 15 minutes. Needs recruiting and a written protocol; gates the release claim, not the build

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
- Phase 2: T023, T027, T034–T037, T039; T175's four per-crate PRs in parallel, before T028; T177 and T178 alongside anything. T179, then T041, then T040; T176 (Phase 1) before T040
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
