---
description: "Task list for 003 source seam import"
---

# Tasks: Import Seams and Fabric Regions from the Source Model

**Input**: Design documents in `/specs/003-source-seam-import/`
**Prerequisites**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/source-import.md](./contracts/source-import.md), [quickstart.md](./quickstart.md)
**Generated**: 2026-10-16

**Tests are mandatory**, as in 001 and 002: Constitution IV requires test-first with independent oracles, and `AGENTS.md` requires the failing test before the implementation.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: which user story it serves
- Task IDs are `S0NN`, distinct from 001's `T0NN` and 002's `H0NN` in commit messages and PR titles. Decision IDs keep their hyphen (`S-007`)

## PR packaging

Same policy as 001 and 002: 400 lines of hand-written source, one test task plus the implementation that satisfies it, one story and one crate per PR. See [`AGENTS.md`](../../AGENTS.md#commits-and-prs) and [docs/how-to/pull-requests.md](../../docs/how-to/pull-requests.md).

### Batch these

| PR | Tasks |
|---|---|
| `003/NN-S006-source-materials` | S005, S006 (`openplush-mesh`) |
| `003/NN-S008-io-materials` | S007, S008 (`openplush-io`), opened alongside 001's T053 |
| `003/NN-S010-source-fixtures` | S009, S010 |
| `003/NN-S015-uv-cuts` | S011, S012, S014 (thin `openplush-geom` wiring commit), S015 |
| `003/NN-S017-dead-ends` | S013, S016, S017 |
| `003/NN-S022-source-stage` | S018, S019, S021 (thin `openplush-seams` wiring commit), S022 |
| `003/NN-S036-regions` | S033, S034, S036 (`openplush-pattern`) |
| `003/NN-S038-source-materials-stage` | S035, S038 (`openplush-pipeline`) |

`NN` is assigned when the PR is opened: the next unused position in feature 003 (`01` is the spec and plan).

### Isolate these

| Task | Why |
|---|---|
| Each spike (S001–S004) | A spike's output is a decision. Bundled with code, the decision hides inside a diff |
| **S006** `SourceMaterials` on `ManifoldMesh` | Changes 001's core mesh type (S-011). It needs reviewers from both features |
| **S037** `Fabric::with_defaults` | Blocked on the maintainer's S-007 answer, and 001's own "add fabric" will use it |
| **S017** dead-end check | Where FR-012's "nothing is proposed" is either structural or quietly isn't |

### Fixtures go alone

The Blender exports from S001 and S004 are fixture PRs of their own with their `fixtures/LICENSES.md` entries, as are S009–S010.

---

## Blocking dependency on 001

This feature **cannot start its engine work** until these 001 tasks are green (plan, "Hard dependency on 001"):

| 001 tasks | Why |
|---|---|
| T050, T051, T053 | The importers S008 extends |
| T060, T066 | Weld (UV cuts survive it) and hole fill (the repair-filled faces of FR-024) |
| T069–T076 | Seam set, seam graph, charts, topology, `propose_cuts` |
| T084 | `Fabric` and the presets |
| T107 | Proposal preview-then-accept |
| T124, T125 | Project document and provenance keys |
| T128 | Undo |
| T137 | Painted regions |

S001–S004 (spikes) and S005–S006 (the mesh extension) can start now: `TriangleSoup` and `ManifoldMesh` already exist.

---

## Phase 0: Spikes

**Purpose**: settle the four `[verify]` items in [research R6](./research.md#r6-spikes) before code depends on them. Each spike's result goes in `research/003_qN_<slug>/results.md`, its row in research R6, and the Decision Ledger entry it confirms or changes.

- [ ] S001 [P] **Q1**: UV-cut tolerance, in `research/003_q1_uv_cut/`. On Blender 4.x exports (glTF and OBJ) of the capsule and a quadruped with marked seams, confirm that every marked seam, and only those, reads as a UV cut at `UV_CUT = 1e-5` under research R2's per-endpoint test, and that a dead-ended seam reads as a slit. Records the confirmed value for S014 and flips S-003 from provisional. The exports land first as a fixture PR: `fixtures/meshes/source-seams/blender-capsule.{glb,obj}`, `blender-quadruped.glb`, each with a `fixtures/LICENSES.md` entry. Box: 0.5 day
- [ ] S002 [P] **Q2**: which Blender UV map becomes `TEXCOORD_0` (active or first listed), in `research/003_q2_texcoord0/`. Result sets the import report's wording for `uv_set_read` (FR-004) used by S021. Box: 1 h
- [ ] S003 [P] **Q3**: does `tobj` split a model on a `usemtl` change inside one `o`/`g` group, in `research/003_q3_tobj_usemtl/`. A "no" means S008's OBJ importer splits primitives itself to keep S-002's invariant (a primitive has at most one material). Box: 1 h
- [ ] S004 [P] **Q4**: positional re-placement, in `research/003_q4_reimport/`. Edit one leg of the Blender quadruped, re-export, and measure what share of untouched seams re-place by position within the weld tolerance (research R5). The pair lands as a fixture PR: `fixtures/meshes/source-seams/quadruped-v1.glb`, `quadruped-v2.glb`, with `fixtures/LICENSES.md` entries. Changes R5's `[likely]` tag either way; a low share reopens S-008. Box: 0.5 day

**Checkpoint**: `UV_CUT` has a measured value, the OBJ split rule is known, and re-import has a measured re-placement rate.

---

## Phase 1: Foundational — materials on the soup and the mesh

**Purpose**: 001 FR-002 promises to keep material assignment, but `TriangleSoup` and `ManifoldMesh` have nowhere to put it (data-model §1, S-002, S-011). Two crates, so two PRs: the mesh first, then the importers.

- [ ] S005 [P] `crates/openplush-mesh/tests/source_materials.rs`: (a) `TriangleSoup::new` refuses a `by_primitive` shorter than the highest `SourceCorner::primitive` used, with a typed `MeshError` naming both lengths; (b) a face's material is its first corner's primitive's material; (c) building a `ManifoldMesh` from the soup carries `SourceMaterials` and `extra_uv_sets` unchanged; (d) a face's material state has three distinct values: a material, no material in the file (`by_primitive` is `None`), and repair-filled (its corner provenance is `None`). (d) is the test that keeps FR-024's filled holes from ever reading as "No material"
- [ ] S006 `crates/openplush-mesh/src/materials.rs` (new), `ids.rs`, `soup.rs`, `halfedge/mod.rs`, `halfedge/build.rs`: `MaterialIndex` via `id_newtype!`; `SourceMaterials { names: Vec<String>, by_primitive: Vec<Option<MaterialIndex>> }` with names as written in the file (no trimming: that is matching's job, S-006); `extra_uv_sets: u32` on the soup; the same two fields and read accessors on `ManifoldMesh`, moved unchanged by construction from the soup. The three-state face material is an enum read from `corner_src: Vec<Option<SourceCorner>>`, where `None` is the repair-filled case. Rustdoc states the invariant "every source primitive has at most one material". Keep `soup.rs` under 400 lines by putting the new types in `materials.rs`
- [ ] S007 `crates/openplush-io/tests/materials.rs`: OBJ faces before any `usemtl` have no material; a `usemtl` change inside one group starts a new primitive (S003's answer decides whether `tobj` or our code does it, but the test is the same); glTF gives one material per primitive and a primitive with no material maps to `None`, not to a "default" name; a glTF with `TEXCOORD_0` and `TEXCOORD_1` reads the first and reports `extra_uv_sets == 1`; triangulating a polygon keeps every triangle on its polygon's primitive
- [ ] S008 `crates/openplush-io/src/obj.rs`, `gltf.rs`, `triangulate.rs`: fill `SourceMaterials` and `extra_uv_sets`; the OBJ importer splits primitives at each material change if S003 says `tobj` doesn't. Open alongside 001's T053 (same `openplush-io` files): either in T053's PR or the next `openplush-io` PR after it, never ahead of T050/T051. Depends on S006

**Checkpoint**: a repaired mesh knows each face's material state. Nothing reads it yet.

---

## Phase 2: Fixtures

- [ ] S009 [P] `research/fixtures_gen/generate.py`, `test_fixtures.py`: a GLB writer (stdlib only, no new dependency) that splits vertices at every UV and normal discontinuity, as Blender's exporter does (research R1), with multiple primitives and materials. Test: a written file round-trips through the generator's own reader with the same split count and material per primitive
- [ ] S010 `fixtures/meshes/source-seams/`, `fixtures/seams/source-seams/`, `fixtures/LICENSES.md`, `fixtures/README.md`: generate quickstart's synthetic fixtures (`capsule-uv.glb` and `.obj`, `capsule-hardshade.glb`, `sphere-one-cut.glb`, `three-materials.glb` with materials "Minky", "fleece " and "Felt Nose", `dead-end.glb` with two dead ends and one cut ending on a material boundary, `no-uv.obj` with one material). Each gets a recorded layout in the existing seam-set format, against an OBJ twin of the welded mesh so edges stay OBJ vertex pairs. Depends on S009

---

## Phase 3: Foundational — UV cuts and the dead-end check

**Purpose**: every story's proposal passes through the same cut reading and the same dead-end gate. The gate lands here, not in US4, so no build ever proposes a dead-ended slit (which 001's graph would classify as a dart, against FR-010).

### Tests first

- [ ] S011 [P] `crates/openplush-seams/tests/source_uv.rs`: on `capsule-uv.glb` and `.obj` the UV cuts equal the recorded layout exactly (SC-001); `capsule-hardshade.glb` gives the identical edge set (SC-002); a cut whose UVs agree at one endpoint and differ at the other is still a cut (research R2's per-endpoint rule); a UV difference at or below `UV_CUT` is not a cut; an edge with a UV-less side is never a cut, so a filled hole gets no ring of false cuts; a boundary edge is never a cut (FR-003)
- [ ] S012 [P] `crates/openplush-seams/tests/source_props.rs`: `proptest` over random soups: the detected edges are ascending and identical on a second run (FR-061), and no edge ever carries `SeamKind::Dart` or `SeamKind::Opening` (FR-010)
- [ ] S013 [P] `crates/openplush-seams/tests/dead_end.rs`: `dead-end.glb` reports exactly its two tips, both in one error (spec edge case "all at once"); the cut ending on a material boundary is not a tip (FR-013); a cut ending on a mandatory repair seam or on a surface boundary is not a tip; a cut ending on a seam already in the project's `SeamSet` **is** a tip (S-012); each chain is walked from its tip. Also pins `material_boundaries` on `three-materials.glb` against its recorded boundary edges, and asserts that a material face next to a repair-filled face is not a boundary

### Implementation

- [ ] S014 `crates/openplush-geom/src/epsilon.rs`: `UV_CUT` with S001's value (provisional `1e-5`). Its doc comment says why it is absolute and dimensionless, unlike the model-relative and physical tolerances beside it
- [ ] S015 `crates/openplush-seams/src/source.rs` (new): `uv_cuts(&ManifoldMesh) -> Vec<EdgeId>` in ascending order, per research R2: interior edges only, UVs on both sides, endpoint UVs differ by more than `UV_CUT` at either end. Normals are never read (FR-001)
- [ ] S016 `crates/openplush-seams/src/source.rs`: `material_boundaries(&ManifoldMesh) -> Vec<EdgeId>`: interior edges whose two faces both come from the source file and differ in `Option<MaterialIndex>`. A repair-filled face is never on a boundary (data-model §2). Needed here because the dead-end gate takes the union; US3 consumes the same function
- [ ] S017 `crates/openplush-seams/src/source.rs`: the dead-end check over UV cuts ∪ material boundaries ∪ mandatory seams, reusing T070's node classification (a free endpoint is a tip), with surface-boundary vertices as endpoints and the project's existing seams excluded (S-004, S-012). Returns every tip ascending and each tip's chain, for `EngineError::DeadEndCut { tips, chains }`

**Checkpoint**: the seams crate can read the file's cuts and refuse a bad file. Nothing is proposed yet.

---

## Phase 4: US1 — Bring my UV seams in as sewing seams (Priority: P1) 🎯 MVP

**Goal**: a maker who unwrapped in Blender imports, accepts, and reaches the flat pattern without drawing a seam.

**Independent Test**: `capsule-uv.glb` gives proposed seams equal to the recorded layout; accepting and generating gives one piece per UV island.

### Tests for US1

- [ ] S018 [P] [US1] `crates/openplush-pipeline/tests/source_import.rs`: `source_import` on `capsule-uv.glb` returns one `Seam` proposal with reason `SourceModel` whose edges equal the layout; declining leaves the `SeamSet` untouched (US1 scenario 3); `no-uv.obj` gives `seams: None`, `uv_set_read: None` in the report (FR-005, US1 scenario 4); an edge already in `existing` is not in the proposal; `dead-end.glb` returns `DeadEndCut` and no `SourceImport` (FR-012)
- [ ] S019 [P] [US1] `crates/openplush-pipeline/tests/source_import_report.rs`: `insta` snapshot of the `SourceReport` for `capsule-uv.glb` and `no-uv.obj`; byte-identical across two runs (SC-007)
- [ ] S020 [P] [US1] `bindings/openplush-cli/tests/import_seams.rs`: `import-seams capsule-uv.glb --json` emits the JSON quickstart scenarios 1, 2 and 6 assert against; `DeadEndCut` exits with the CLI's existing `EngineError` code and lists every tip

### Implementation

- [ ] S021 [US1] `crates/openplush-seams/src/proposal.rs`: append `SeamReason::SourceModel` with its explanation (the file's UV cut, in plain language). Appended last; `SeamKind`'s hash bytes are unaffected
- [ ] S022 [US1] `crates/openplush-pipeline/src/source.rs` (new), `error.rs`, `lib.rs`: `source_import(&ManifoldMesh, &SeamSet, &[Fabric]) -> StageResult<SourceImport>` per contract §1, returning `EngineError::DeadEndCut` from S017 before anything is built. This slice fills `seams` (UV cuts as `Cut`) and the report's `uv_set_read`, `extra_uv_sets`, `uv_cut_edges`, `faces_without_uv`; `regions`, `new_fabrics` and `closing` stay empty until US3 and US2. Rustdoc on every public item; `DeadEndCut`'s remedy text from contract §2 in `docs/reference/engine-api.md` with the engine-api version bump there
- [ ] S023 [US1] `bindings/openplush-cli/src/commands/import_seams.rs`: `import-seams <model> --json <out>` (contract §3, Constitution II); `docs/reference/cli.md`
- [ ] S024 [US1] `bindings/openplush-wasm/`, `packages/engine-client/src/ops/source-import.ts`: the `sourceImport` request and `SourceImport` DTO under ADR 0010 (finite floats, `ProposalId` as a 16-hex-digit string), with the mock-worker implementation so shell tests need no WASM
- [ ] S025 [US1] `apps/web/src/import/preview.tsx`, `apps/web/src/import/overlay.ts`: the import preview showing imported seams on the model with their count, colour-blind-safe (FR-040); accept applies the seam proposal through T107 as one undo step (T128); decline leaves the project as a plain import would (FR-042). Accepted seams are ordinary seams from then on (FR-043)
- [ ] S026 [P] [US1] `apps/web/tests/import-preview.spec.ts`: Playwright on `capsule-uv.glb`: preview shows the seams, accept then undo restores the plain import, decline changes nothing

**Checkpoint**: US1 complete. UV seams come in, and a bad file is refused with every tip listed.

---

## Phase 5: US2 — Pieces that can't lie flat are fixed in the same step (Priority: P1)

**Goal**: when the file's cuts leave a piece that can't lie flat, the preview already holds the fewest extra seams, and one action accepts everything.

**Independent Test**: `sphere-one-cut.glb` shows the cut plus highlighted extra seams; after "accept all" every piece flattens.

### Tests for US2

- [ ] S027 [P] [US2] `crates/openplush-pipeline/tests/source_closing.rs`: `sphere-one-cut.glb` gives a non-empty `closing`, each with a non-empty explanation (FR-031); applying `seams` then every `closing` proposal leaves every chart a disk (SC-003); `capsule-uv.glb` gives an empty `closing` (US2 scenario 4); a closed surface with no imported cut still gets at least one closing cut (spec edge case)
- [ ] S028 [P] [US2] `bindings/openplush-cli/tests/pattern_seams_from_model.rs`: `pattern sphere-one-cut.glb --seams-from-model --accept-closing` produces pieces; without `--accept-closing` it exits with `ChartNotDisk` carrying the proposals (quickstart scenario 3, S-010)

### Implementation

- [ ] S029 [US2] `crates/openplush-pipeline/src/source.rs`: fill `closing` with 001's `propose_cuts` (T076) over `existing ∪ seams`; invariant: `closing` is empty exactly when every chart is a disk
- [ ] S030 [US2] `bindings/openplush-cli/src/commands/pattern.rs`: `--seams-from-model` and `--accept-closing`; headless never accepts a proposal without the flag naming it (S-010); `docs/reference/cli.md`
- [ ] S031 [US2] `apps/web/src/import/preview.tsx`, `overlay.ts`: extra seams highlighted distinctly from imported ones with their explanations (FR-030, FR-031); "accept all" applies seams then each closing proposal as one undo step (FR-041); "accept without extra seams" applies the imported seams only (US2 scenario 3); "no extra seams needed" message when `closing` is empty
- [ ] S032 [P] [US2] `apps/web/tests/import-accept-all.spec.ts`: Playwright on `sphere-one-cut.glb`: two distinct highlight treatments, "accept all" is a single undo step, partial acceptance then generate shows 001's `ChartNotDisk` validation

**Checkpoint**: SC-006's three actions (import, accept all, generate) work for a cut-only model.

---

## Phase 6: US3 — My materials become my fabrics (Priority: P1)

**Goal**: each material becomes a fabric region, boundaries become seams, and unmatched names get an editable new profile.

**Independent Test**: `three-materials.glb` gives three regions, a seam on every material boundary, "Minky" and "fleece " matched to presets, and one created profile "Felt Nose".

### Tests for US3

- [ ] S033 [P] [US3] `crates/openplush-pattern/tests/source_regions.rs`: every face in exactly one region (SC-005, proptest over random material assignments); "fleece " and "FLEECE" match the Fleece preset (S-006: Unicode-lowercase, trimmed, first in project order); no fuzzy match ("minky_fabric" creates a profile); a model with one material gives one region (US3 scenario 4); a model with no material on any face gives no regions (FR-005); source faces with no material among materialled ones form "No material", matched like any other name (FR-020)
- [ ] S034 [P] [US3] `crates/openplush-pattern/tests/source_regions_filled.rs`: a repair-filled area joins the region bordering the largest share of its perimeter by length; ties go to the lowest `MaterialIndex`, "No material" last (data-model §3); a filled area is never its own region and never "No material"
- [ ] S035 [P] [US3] `crates/openplush-pipeline/tests/source_materials.rs`: on `three-materials.glb`, every material boundary is in `seams` as `FabricBoundary`; an edge that is both a UV cut and a boundary appears once, as `FabricBoundary` (S-005, FR-021); `shared_edges` counts it once; two materials matching the same profile still get a seam between them (spec edge case)

### Implementation

- [ ] S036 [US3] `crates/openplush-pattern/src/regions.rs`: regions from `ManifoldMesh`'s material states, name matching per S-006, the FR-024 filled-area rule and tie-break. Extends T137's module; split into `regions/source.rs` if it nears 400 lines
- [ ] S037 [US3] `crates/openplush-pattern/src/fabric.rs`: `Fabric::with_defaults(name)` with the S-007 defaults. **Blocked on the maintainer's S-007 decision** (proposed: quilting-cotton values). 001's "add fabric" uses the same constructor, so the decision lands in 001's plan as well when T084 has merged. Test in the same PR: the created profile carries the given name verbatim and the confirmed values, and `frays` is the confirmed value (FR-057's non-zero allowance)
- [ ] S038 [US3] `crates/openplush-pipeline/src/source.rs`: add material boundaries to `seams` as `FabricBoundary` (UV-cut-only edges stay `Cut`); fill `regions`, `new_fabrics`, and the report's `material_boundary_edges`, `shared_edges`, `filled_areas`, `matched`, `created`. Extend S019's snapshot with `three-materials.glb`
- [ ] S039 [US3] `apps/web/src/import/preview.tsx`, `overlay.ts`: imported regions shown in a third colour-blind-safe treatment with counts (FR-040); created profiles listed as new and editable (FR-023); "accept all" also calls `set_fabrics` in the same undo step (plan, "Accepting")
- [ ] S040 [P] [US3] `apps/web/tests/import-regions.spec.ts`: Playwright on `three-materials.glb`: three regions, "Felt Nose" listed as created and editable, repainting an accepted region behaves like a hand-painted one (US3 scenario 5)

**Checkpoint**: SC-006 holds for a cut and material-assigned model.

---

## Phase 7: US4 — A broken cut is caught, not guessed at (Priority: P2)

**Goal**: the maker sees every dead-ended cut on the model with the remedy. The gate itself landed in Phase 3; this phase makes it usable.

**Independent Test**: `dead-end.glb`: the error names both cuts, the 3D view locates them, nothing is proposed, the model still imports.

- [ ] S041 [P] [US4] `apps/web/tests/import-dead-end.spec.ts`: Playwright on `dead-end.glb`: both chains highlighted, the remedy text shown, no proposal offered, seams can still be drawn by hand (US4 scenario 2)
- [ ] S042 [US4] `apps/web/src/import/dead-end.tsx`: highlight each `chains[i]` from its tip, list the tips, show the engine-supplied remedy (FR-011); camera moves to a tip when it is selected
- [ ] S043 [US4] `crates/openplush-pipeline/tests/source_import.rs`: the cut in `dead-end.glb` that ends on a material boundary is not reported once materials are wired (US4 scenario 3, end to end through the stage rather than S013's seams-level check)

**Checkpoint**: all four stories work.

---

## Phase 8: Re-import (FR-050 to FR-052)

**Purpose**: no story owns re-import, but every story's result lands in a project that may already have seams. Needs 001's T124–T125 and S004's measured rate.

- [ ] S044 [P] `crates/openplush-project/tests/origin.rs`: a v1 project file with no `origin` loads with every seam `drawn`; `origin` round-trips; the schema version is unchanged (S-009)
- [ ] S045 `specs/001-pattern-pipeline/contracts/project.schema.json`, `crates/openplush-project/`: optional `seams[].origin: "drawn" | "imported" | "proposed"`, default `drawn`; accepted imports write `imported` and accepted closing seams `proposed`; `docs/reference/` project schema entry
- [ ] S046 [P] `crates/openplush-pipeline/tests/reimport.rs`: on the quadruped pair, Merge re-places seams away from the edited leg by position; seams that can't be placed are in `unplaceable_seams`, located on the old mesh, never dropped or snapped (FR-051); Replace counts `removed_seams` and `removed_hand_drawn`; an imported seam on an existing seam edge is not duplicated (FR-050)
- [ ] S047 `crates/openplush-pipeline/src/reimport.rs`: `OnExisting`, `ReimportDiff`, `reimport_diff` per contract §1 and research R5, mapping edges and faces by position within the weld tolerance
- [ ] S048 `bindings/openplush-cli/src/commands/import_seams.rs`: `--project` and `--on-existing replace|merge` (quickstart scenario 7); `docs/reference/cli.md`
- [ ] S049 `bindings/openplush-wasm/`, `packages/engine-client/src/ops/source-import.ts`: `reimportDiff` request and DTO
- [ ] S050 `apps/web/src/import/reimport.tsx`: Replace or Merge with neither preselected (FR-050); the difference before commit, hand-drawn removals called out by count, unplaceable seams located on the previous model (FR-051); each choice one undo step, declining changes nothing (FR-052)
- [ ] S051 [P] `apps/web/tests/import-reimport.spec.ts`: Playwright: neither option preselected, the diff shows before commit, undo restores the prior project

---

## Phase 9: Polish

- [ ] S052 [P] `crates/openplush-pipeline/benches/source_import.rs`: SC-009, under 1 s added to a 50k-triangle import
- [ ] S053 [P] `apps/web/tests/import-no-network.spec.ts`: zero network requests during import, preview and acceptance (SC-008, FR-062)
- [ ] S054 [P] `apps/web/src/import/keyboard.ts`: keyboard alone can accept all, accept partially, decline, choose Replace or Merge (FR-044); verify all three palettes in a colour-blindness simulator (FR-040)
- [ ] S055 [P] Native vs WASM determinism: quickstart scenario 8, byte-identical JSON for `three-materials.glb` (FR-061, SC-007)
- [ ] S056 `docs/explanation/`: how UV cuts and materials become seams and regions, and why texture-only cuts should live on a second UV map (plan, "Trade-offs accepted")
- [ ] S057 `specs/001-pattern-pipeline/contracts/engine-api.md`, `specs/001-pattern-pipeline/plan.md`, `specs/001-pattern-pipeline/data-model.md`, this feature's `data-model.md` §1: once S006 and S022 have merged, the as-built pointers: 001's contract gains a one-line pointer to the `source_import` rustdoc, 001's Decision Ledger records the engine-api bump, and both data models' soup and mesh sections point to `openplush-mesh` rustdoc. Spec-only PR; no signature edits ([ADR 0003](../../docs/adr/0003-as-built-docs.md))
- [ ] S058 Run the full [quickstart.md](./quickstart.md) walkthrough, scenarios 1–9

---

## Dependencies and Execution Order

### Phase dependencies

- **Phase 0**: start now. S001 and S004 each need their Blender exports in a fixture PR first
- **Phase 1**: S005–S006 start now; S007–S008 need S006, S003, and 001's T050, T051, T053
- **Phase 2**: any time; S010 needs S009
- **Phase 3**: needs Phase 1, S010, S001 (for S014's value), and 001's T060, T066, T069–T070. **Blocks every story**
- **Phase 4 (US1)**: needs Phase 3 and 001's T107, T128
- **Phase 5 (US2)**: needs US1's stage (S022) and 001's T073–T076
- **Phase 6 (US3)**: needs Phase 3 and 001's T084, T137; S037 needs the S-007 decision. S038 needs S022
- **Phase 7 (US4)**: needs S022 and S038 (for S043)
- **Phase 8**: needs US1, S004, and 001's T124, T125
- **Phase 9**: needs everything; S057 needs only S006 and S022 merged

### Story independence

US1 ships alone. US2 and US3 each need only US1's stage and can run in parallel. US4's gate is foundational, so its phase is shell work plus one end-to-end test.

### Parallel opportunities

- All of Phase 0
- Phase 1's mesh PR alongside Phase 2's fixtures
- All of Phase 3's tests (S011, S012, S013)
- Phases 5 and 6 together after S022
- All of Phase 9 except S057 and S058

### Within each story

1. Tests written and observed failing
2. Seams and pattern crates before the pipeline stage, the stage before CLI and WASM
3. Engine before shell, always

---

## Traceability

| Spec requirement | Tasks |
|---|---|
| FR-001 UV cut, normals ignored | S001, S014, S011, S015 |
| FR-002 material boundary | S016, S035 |
| FR-003 boundary edges never seams | S011, S015 |
| FR-004 first UV set, report says so | S002, S007, S022 |
| FR-005 no UVs or no materials | S018, S033 |
| FR-010 never a dart or opening | S012 |
| FR-011 dead end is an error | S013, S017, S042 |
| FR-012 nothing proposed on failure | S017, S018 |
| FR-013 dead ends on the combined set, project excluded | S013, S017, S043 |
| FR-014 small closed piece marked | **gap**: see note below |
| FR-020 one region per material, "No material" | S033, S036 |
| FR-021 boundary seams, shared edge once | S035, S038 |
| FR-022 name matching | S033, S036 |
| FR-023 new default profile | S037, S039 |
| FR-024 repair-filled faces join a region | S005, S034, S036 |
| FR-030 to FR-032 extra seams | S027, S029, S031 |
| FR-040 to FR-043 preview and acceptance | S025, S031, S039 |
| FR-044 keyboard | S054 |
| FR-050 to FR-052 re-import | S044–S051 |
| FR-060 headless | S023, S030, S048 |
| FR-061 determinism | S012, S019, S055 |
| FR-062 local only | S053 |
| SC-001 / SC-002 | S011 |
| SC-003 | S027 |
| SC-004 | S013 |
| SC-005 | S033 |
| SC-006 | S032, S040 |
| SC-007 | S019, S055 |
| SC-008 | S053 |
| SC-009 | S052 |

**FR-014 gap**: the spec requires the preview to mark a piece below the minimum sewable size, but data-model §3's `SourceImport` has no field for it. Add one in a spec-lane change to data-model §3 before S029; a test and implementation task follow from it.
