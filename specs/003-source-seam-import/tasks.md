---
description: "Task list for 003 source seam import"
---

# Tasks: Import Seams and Fabric Regions from the Source Model

**Input**: Design documents in `/specs/003-source-seam-import/`
**Prerequisites**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/source-import.md](./contracts/source-import.md), [quickstart.md](./quickstart.md)
**Generated**: 2026-10-16

**Tests are mandatory**, as in 001 and 002: Constitution IV requires test-first with independent oracles, and `AGENTS.md` requires the failing test commit before the implementation commit.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: which user story it serves (US1 to US4 in spec.md)
- `needs:ID[,ID]`: unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:S-0NN`: a proposed decision this one waits on; only the maintainer clears it
- `human:XX`: needs a person (outside review, measured values); no agent picks it up
- `needs:ID[,ID]`: unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:S-0NN`: a proposed decision this one waits on; only the maintainer clears it
- `human:XX`: needs a person (outside review, measured values); no agent picks it up
- Task IDs are `U0NN` (for UV), distinct from 001's `T0NN` tasks and `S1`–`S8` spikes and from 002's `H0NN`. This feature's decisions are `S-0NN` and its spikes `Q1`–`Q4`, as in plan.md and research.md.

## PR packaging

Same policy as 001 and 002: one intent per PR, each task a test commit then the implementation that makes it pass, and interdependent intents stacked ([ADR 0013](../../docs/adr/0013-intent-sized-pull-requests.md)). See [`AGENTS.md`](../../AGENTS.md#commits-and-prs) and [docs/how-to/pull-requests.md](../../docs/how-to/pull-requests.md). Branches are `003/NN-slug`, the slug naming the intent; `NN` is the next unused position in feature 003 when the PR opens (01 was the spec and plan, 02 is this task list).

### Batch these

| PR | Tasks |
|---|---|
| `003/NN-U013-source-materials` | U012, U013 |
| `003/NN-U015-io-materials` | U014, U015 |
| `003/NN-U019-uv-cuts` | U016, U017, U019 (stacked on U018's one-constant slice) |
| `003/NN-U071-projection-hint` | U070, U071 (stacked on U019's slice, which creates `source.rs`) |
| `003/NN-U021-source-import-stage` | U020, U021 |
| `003/NN-U032-closing` | U031, U032 |
| `003/NN-U034-accept-all` | U033, U034 |
| `003/NN-U040-material-boundaries` | U039, U040 |
| `003/NN-U044-source-regions-stage` | U043, U044 |
| `003/NN-U052-dead-ends` | U051, U052 |
| `003/NN-U054-source-darts` | U053, U054 |
| `003/NN-U060-reimport-diff` | U059, U060 |

Each CLI, WASM, engine-client and web pair (a test task and its implementation) is one PR.

### Isolate these

| Task | Why |
|---|---|
| **U018** `UV_CUT` | The one value that decides what counts as a seam. It lands alone, with Q1's numbers in the PR body, so the tolerance gets a reviewer's full attention rather than a skim inside detection code |
| **U042** name matching and `Fabric::with_defaults` | S-007's default is also 001's "add fabric" default. It touches 001's fabric module, so both features' reviewers need to see it |
| **U048** `UnreviewedFabric` | A new warning code, and it is printed on the pattern. Its text is maker-facing and gets a prose review |

### Fixtures go alone

U001 to U007 are each their own PR in the fixture lane, as in 002. U001 adds the generator module and the license row; the rest stack on it, because they extend the same module.

---

## Blocking dependency on 001

| 001 tasks | Needed by |
|---|---|
| T024, T025 (landed) | U012, U013: the soup and `ManifoldMesh` being extended |
| T050, T051, T053 | U014, U015: the importers that fill `SourceMaterials` |
| T060, T067 | Every fixture-based engine test: glTF fixtures arrive with split vertices and need the weld before they are a `ManifoldMesh`. Synthetic welded meshes cover unit tests until then |
| T066 | U041, U042: repair-filled faces (FR-024) |
| T069, T070, T073, T075, T076 | U019 (`SeamSet`), U051 (seam graph), U031 and U032 (charts, topology, `propose_cuts`) |
| T084 | U042: `Fabric` and the minky preset |
| T093 | U022 onward: the CLI |
| T099 | U029, U030 and every later web task: the real worker host (U027 runs against the mock worker) |
| T107, T128 | U020, U029, U030 (accepting proposals), U033, U034 ("accept all" as one undo step) |
| T124, T125, T126 | U046, U057, U059: the project document, provenance keys, and the stored mesh chunk |
| T137 | U042: painted regions |

As with 002, work can start partway through 001's Phase 3. Fixtures and spikes (Phase 0) can start now.

## Spec gaps to close before the tasks they block

Each needs a spec-only PR (specs and implementation never share a PR):

1. **001 has no task for its minimum-piece-size check.** 001 data-model §5 defines the value (provisional: 20 mm in each principal direction at target scale), but no 001 task implements it, and 002's H049 depends on the same check. S-013 (Proposed) puts FR-014's mark there rather than in `SourceImport`, so 003's data model needs no new field. U024 is blocked until 001 gains that task, plus a worker operation that exposes the check to `apps/`.
2. **Re-import (FR-050 to FR-052) has no user story**, so its phase below has no story label. Either add a user story (US5, P2) with an independent test, or leave the phase as written. It doesn't block the tasks.

---

## Phase 0: Fixtures and spikes

**Purpose**: Q1 sets `UV_CUT`, which decides every UV cut, so detection (U018 onward) waits for it.

**How fixtures are made**: `research/fixtures_gen/` writes them, using the standard library only, under CC0, byte-reproducible, like the rest of `fixtures/`. It imitates what Blender's exporters write: glTF splits a vertex wherever its corner UVs or normals differ and writes one primitive per material; OBJ writes per-corner `vt` and switches materials with `usemtl`. The generator places every cut itself, so its seam set in `fixtures/seams/` (the existing seam-set format, indexed by the OBJ twin's `v` lines) is the oracle. It comes from the generator, never from OpenPlush (Constitution IV). Whether real Blender exports follow those conventions is for spikes Q1 to Q4, which run Blender as an outside tool under U069's ADR. A spike that finds a difference fixes the generator.

- [x] U069 `docs/adr/0011-blender-research-tool.md`: Blender's Python module (`bpy`, GPL-3.0) is allowed as a research-only tool. Scripts that import it are GPL-3.0-or-later, live only in their spike's directory under `research/` with that spike's own `LICENSE`, and are never imported by `crates/`, `bindings/`, `apps/` or `packages/`. Their exports are not committed as fixtures. Constitution IX is unaffected, because nothing GPL reaches the shipped artifact. Blocks U008, U009, U011
- [x] U001 [P] `research/fixtures_gen/source_seams.py` (new, called from `generate.py`): `fixtures/meshes/source-seams/capsule-uv.glb` and `capsule-uv.obj`, plus the oracle `fixtures/seams/capsule-uv.json`. The capsule has a known seam layout: one meridian from pole to pole plus the tube's upper rim (the capsule has no equator ring), so two UV islands. Pole corners in each island share one UV, so a pole fan carries no false cut. The glTF splits vertices where corner UVs differ, as Blender does. `fixtures/LICENSES.md` gets a `meshes/source-seams/` row. `research/fixtures_gen/test_source_seams.py` checks, sharing no code with the generator: the cut edges recomputed from the OBJ's `vt` by the per-endpoint rule equal the seam set (SC-001); the glTF, once welded by position, has the same triangles and corner UVs as the OBJ; every chart under the seam set is a disk; and the files regenerate byte for byte
- [x] U002 [P] `fixtures/meshes/source-seams/capsule-hardshade.glb`: U001's layout plus hard-shaded edges, so vertices also split for normals alone (SC-002). Its oracle is `fixtures/seams/capsule-uv.json`, and the test checks that its normal splits add no UV cut
- [x] U003 [P] `fixtures/meshes/source-seams/sphere-band.glb` and `.obj`: two latitude-ring UV cuts, like a neck and a waist, so the band between them is an annulus. Its UV island is a ring, which can't lie flat without one more cut (US2). The two caps are disks. The test checks that under the seam set exactly one chart, the band, is not a disk, with two boundary loops
- [x] U004 [P] `fixtures/meshes/source-seams/three-materials.glb` and `three-materials.obj`: materials "minky / cuddle" (lower case), "Polar fleece " (trailing space) and "Felt Nose" (US3). After trimming and lowercasing, the first two equal 001 research R7's preset names "Minky / cuddle" and "Polar fleece"; "Felt Nose" equals none ("Craft felt" is not a match). The glTF has one primitive per material. The OBJ switches `usemtl` twice inside one `o` group, which must still give one primitive per run (001 D-034)
- [x] U005 [P] `fixtures/meshes/source-seams/dead-end.glb`, `.obj` and the oracles `fixtures/seams/non-disk/dead-end.json` (the cuts) and `dead-end.tips.json` (under `non-disk/` because an interior slit leaves a chart that is not a disk): two deliberate dead-ended cuts, plus one cut that ends on a material boundary and one that ends on an opening (US4, FR-013, US4 scenario 3). The tips oracle lists exactly the two tips as OBJ vertex indices
- [x] U006 [P] `fixtures/meshes/source-seams/no-uv.obj`: no `vt` lines, one material (FR-005)
- [x] U007 [P] `fixtures/meshes/source-seams/reshaped-v1.glb` and `reshaped-v2.glb`: U001's capsule with seams and two materials, before and after stretching one end cap. Every vertex outside that cap is bit-identical between the two, so R5's positional re-placement has a known answer (FR-050)
- [x] U008 **Q1** `research/q1_blender_uv_cuts/`: in Blender (U069), mark a known seam layout on a capsule and on a quadruped, add one dead-ended seam, unwrap, and export glTF and OBJ. Measure whether every marked seam and only those read as a UV cut at `UV_CUT = 1e-5`, and whether the dead end reads as a slit. Also measure whether a pole fan's corners share one UV under Blender's usual unwraps, or would read as false cuts under the per-endpoint rule [verify], and whether the exports follow the conventions the generator imitates. Record the measured UV differences (largest across an uncut edge, smallest across a cut) and the chosen value in research.md R2 and R6, and change the R1 dead-end row from `[likely]` to `[solid]` or `[verify]`. Depends on U069. Box: 0.5 day. Delivered on UV spheres, not on a capsule or quadruped; the results are in research.md R1 to R3 and R6
- [x] U009 [P] **Q2** `research/q2_blender_texcoord/`: which Blender UV map becomes `TEXCOORD_0`, the active one or the first listed? Record it in research.md R1; it sets the FR-004 report wording in U021. Depends on U069. Box: 1 h. Answered: the first listed; OBJ carries the active map (research R1, S-016)
- [x] U010 **Q3**: answered without a spike by 001 D-034. 001 reads OBJ with its own reader, not `tobj`, and starts a new source primitive at every `o`, `g` and `usemtl` line, so each primitive has at most one material (S-002) with no 003 work
- [x] U011 [P] **Q4** `research/q4_blender_reshape/`: in Blender (U069), build a quadruped with a marked seam layout (Q1 built none), reshape one leg, and re-export. Measure the share of seam edges away from the edited leg whose endpoints match within the weld tolerance, and update research.md R5's tag. Depends on U069. Box: 1 day

**Checkpoint**: `UV_CUT` has a value measured on UV spheres; the S-002 OBJ question is answered.

---

## Phase 1: Setup

None. No new crate and no new dependency (S-001).

---

## Phase 2: Foundational: materials on the soup and the mesh

**Purpose**: no story can read a material until the soup and `ManifoldMesh` carry one (S-002, S-011). Blocks US3 and US4 directly, and US1 through the importers' `extra_uv_sets`.

### Mesh (`openplush-mesh`)

- [x] U012 [P] `crates/openplush-mesh/tests/soup.rs` and `crates/openplush-mesh/src/halfedge/tests.rs`: failing tests for `SourceMaterials` (data-model §1). In `tests/soup.rs`: `TriangleSoup::with_materials` (S-019) refuses, with a new typed `MeshError` variant naming the lowest offending primitive, (a) a `by_primitive` "shorter than the highest primitive index used" and (b) a `MaterialIndex` ≥ `names.len()`. A valid soup returns its materials and `extra_uv_sets` unchanged. In `src/halfedge/tests.rs`, because `from_oriented_triangles` is `pub(crate)` and stays so: building a `ManifoldMesh` from such a soup carries both fields unchanged, and a face's material is its first corner's primitive's material
- [x] U013 `crates/openplush-mesh/src/soup.rs`, `src/ids.rs`, `src/halfedge/mod.rs`, `src/halfedge/build.rs`: `MaterialIndex` via `id_newtype!`; `SourceMaterials { names, by_primitive }` and `extra_uv_sets: u32` on `TriangleSoup` and `ManifoldMesh`, with read accessors only. Doc comments state the invariant "every source primitive has at most one material". Makes U012 pass — *merged in #137 (`with_materials` does the refusing, S-019)*

### Importers (`openplush-io`, alongside 001 T053)

Lands after T050 and T051 merge, as its own 003 slice. Developed alongside T053 so triangulation keeps one primitive's corners under that primitive's number.

- [x] U014 [P] `crates/openplush-io/tests/source_materials.rs`: failing tests on U004 and U006. glTF: names in file order, one material per primitive, a primitive with no material (glTF's default material) maps to `None`, and `extra_uv_sets` counts `TEXCOORD_1` and up. OBJ: names from `usemtl` exactly as written (no trimming; matching trims later), faces before any `usemtl` map to `None`, and each `usemtl` change starts a new primitive even inside one group. `no-uv.obj` imports with no UVs and one material — *OBJ half merged in #139; glTF half built in memory where no fixture has a material-less primitive or a `TEXCOORD_1`; it passed on arrival, since T051 (#308) already filled the table*
- [x] U015 needs:T051 `crates/openplush-io/src/obj/mod.rs`, `src/obj/parse.rs`, `src/gltf/mod.rs`: fill `SourceMaterials` and `extra_uv_sets`. OBJ: name each primitive after the `usemtl` line that opened it (the reader already splits there, 001 D-034); no MTL file is read, because only the name is used. glTF: one material per primitive, after T051. Makes U014 pass — *OBJ half merged in #139; the glTF half came with T051's reader (#308) in `src/gltf/mod.rs`, as built in [openplush-io](../../docs/reference/crates/openplush-io.md)*

**Checkpoint**: an imported fixture's `ManifoldMesh` names each face's material.

---

## Phase 3: User Story 1: bring my UV seams in as sewing seams (P1) 🎯 MVP

**Goal**: UV cuts become proposed `Cut` seams that the maker accepts or declines.

**Independent test**: import `capsule-uv.glb`; the proposed seams equal `capsule-uv.layout.json` edge for edge. Accept them, generate the pattern, and get one piece per UV island.

### Tests first

- [x] U016 [P] [US1] `crates/openplush-seams/tests/source_uv_cuts.rs`: failing tests on hand-built welded meshes from `openplush-testkit`. An interior edge is a cut when, at either endpoint, the corner UVs on the two sides differ by more than `UV_CUT`: one endpoint agreeing and the other differing is still a cut. A difference at or below `UV_CUT` is not a cut. An edge with a UV-less side is never a cut, and those faces are counted. A boundary edge is never a cut (FR-003). Normals are never read (FR-001). Once T060 and T067 land, also: `capsule-uv.glb` and `.obj` give exactly the U001 layout by position (SC-001), and `capsule-hardshade.glb` gives the same set (SC-002) — *as built, the fixture half (SC-001, SC-002) is `crates/openplush-pipeline/tests/source_uv_fixtures.rs`, because reading the files needs `openplush-io`, a sibling of `openplush-seams`; the meshes are generated in the test, as `openplush-testkit` has no mesh builders. Each UV coordinate is compared on its own, as U001's oracle does, and a NaN difference reads as a cut*
- [x] U017 [P] [US1] `crates/openplush-seams/tests/source_props.rs`: `proptest` over random UV assignments on testkit meshes. The proposed seams contain no `Dart` and no `Opening` kind (FR-010), never an edge already in the given `SeamSet`, and come out in ascending `EdgeId` order, identical on repeated runs (FR-061) — *also checks the cuts against an oracle recomputed from the triangles by vertex index, and `faces_without_uv`*

### Implementation

- [x] U018 [US1] `crates/openplush-geom/src/epsilon.rs`: `UV_CUT` with Q1's value. Its doc comment says why it is absolute and dimensionless, without citing a spec section (AGENTS.md). One-constant slice, stacked under U019. Depends on U008 — *merged in #144*
- [x] U019 [US1] `crates/openplush-seams/src/source.rs` (new) and `src/proposal.rs`: UV-cut detection over `&ManifoldMesh` and `&SeamSet`, returning the cut edges and the count of faces without UVs. Adds `SeamReason::SourceModel`, with a doc comment, and builds the seam `Proposal` (`ProposalKind::Seam { reason: SeamReason::SourceModel }`, `SeamKind::Cut`) through `Proposal::new`, or nothing when there are no edges. A `debug_assert!` checks that no `Dart` or `Opening` kind is produced. Makes U016 and U017 pass — *`uv_cuts(mesh, seams) -> Result<UvCuts, SeamsError>` (`UnknownEdge` for a seam the mesh lacks), with `UvCuts::edges`, `faces_without_uv` and `proposal() -> Result<Option<Proposal>, SeamsError>`; documented in `docs/reference/crates/openplush-seams.md`*
- [x] U070 [P] [US1] `crates/openplush-seams/tests/source_projection.rs`: failing tests for projection points (spec FR-006, data-model §2). A vertex with 12 incident faces whose pole corners all differ by more than `UV_CUT` is one. A vertex with 4 such faces is not, and neither is one with 5 faces where two corners agree, or one where any incident face has no UVs. A pole fan whose corners share one UV gives none. A vertex where five hand-marked seams meet is also one: a known false positive, which the hint's "may" wording allows for. The points come out ascending, and the cut edges are the same with or without them (the hint changes no proposal). Once T060 and T067 land, also: `capsule-uv.glb` has none — *as built, the fixture half is in `crates/openplush-pipeline/tests/source_uv_fixtures.rs`, as U016's is; the meshes are generated bipyramids and a five-sector disc, and a pole on the surface boundary is also none*
- [x] U071 [US1] `crates/openplush-seams/src/source.rs`: projection points and the private `PROJECTION_MIN_FACES = 5`, whose comment gives research R2's reason for five. Makes U070 pass — *`UvCuts::projection_points() -> &[VertexId]`, computed by `uv_cuts` beside the cuts; documented in `docs/reference/crates/openplush-seams.md`*
- [ ] U020 [US1] `crates/openplush-pipeline/tests/source_import.rs`: failing tests for `source_import(mesh, existing, fabrics)` (contract §1). On a UV-cut mesh, `seams` is `Some`, with the cuts. With no UVs, `seams` is `None`, the report's `uv_set_read` is `None`, and the report says no UV cuts were found (US1 scenario 4). Edges already in `existing` are excluded. Applying `seams` through T107's path gives the same `SeamSet` as drawing those edges by hand (US1 scenario 2). Report counts (`uv_set_read`, `extra_uv_sets`, `uv_cut_edges`, `faces_without_uv`, `projection_points`) as an `insta` snapshot. A mesh with `extra_uv_sets = 2` carries FR-004's warning naming 2 ignored sets, and one with 0 carries none. Repeated runs give byte-identical serialised output (SC-007)
- [ ] U021 [US1] `crates/openplush-pipeline/src/source.rs` (new) and `src/lib.rs`: the `source_import` stage and the public `SourceImport { seams, report }` / `SourceReport` with only US1's fields; later stories add theirs. The FR-004 warning (S-016) is part of the report whenever `extra_uv_sets > 0`, and its text goes in `docs/reference/` with the stage. The FR-006 projection hint text goes in `docs/reference/` with the stage. Includes rustdoc for every public item and the `source_import` entry in `docs/reference/engine-api.md`. Makes U020 pass. Its spec follow-up is U068
- [ ] U022 [P] [US1] `bindings/openplush-cli/tests/import_seams.rs`: failing tests for quickstart scenarios 1, 2 and 6. `import-seams <model> --json <out>` writes the proposal and report; the OBJ twin gives the same edge set by position; two runs give byte-identical files
- [ ] U023 [US1] `bindings/openplush-cli/src/commands/import_seams.rs`: the `import-seams` subcommand (contract §3) and its entry in `docs/reference/cli.md`. Makes U022 pass
- [ ] U024 [US1] human:spec-gap-1 FR-014 (S-013): a closed cut that cuts off a piece below 001's minimum piece size is still proposed, and the preview marks that piece before acceptance using 001's piece-size check on the flattened, scaled proposal. `apps/` reaches that check only through `@openplush/engine-client`, so this depends on 001's check and its worker operation, both still to be tasked (spec gap 1). Playwright test in `apps/web/tests/import-preview.spec.ts` (created by U029), then the implementation in `apps/web/src/import/`. No 003 engine change. **Blocked** on spec gap 1
- [ ] U025 [P] [US1] `bindings/openplush-wasm/tests/source_import.rs`: failing DTO tests. `sourceImport` request and response; each `ProposalId` crosses as its 16-hex-digit string; floats are finite; no field is a 64-bit integer (ADR 0010). The response for `capsule-uv.glb` is byte-identical to the native CLI's JSON (SC-007)
- [ ] U026 [US1] `bindings/openplush-wasm/src/`: the `sourceImport` operation and its DTOs, regenerated TS types via `scripts/check.sh types`, and the operation in `docs/reference/worker-protocol.md`. Makes U025 pass
- [ ] U027 [P] [US1] `packages/engine-client/src/source-import.test.ts`: failing Vitest tests for the client's `sourceImport` call against the mock worker. A crash in the worker surfaces as a crash (worker-protocol §4), not an empty proposal
- [ ] U028 [US1] `packages/engine-client/src/client.ts`: the `sourceImport` call on the public client. Makes U027 pass
- [ ] U029 [P] [US1] `apps/web/tests/import-preview.spec.ts`: failing Playwright tests on `capsule-uv.glb`. Proposed seams show on the model with a count. Accepting makes them seams that can be edited, erased, and undone like drawn seams (FR-043). Declining leaves the project equal to a plain import (FR-042). Accept and decline work from the keyboard alone (FR-044). No network request is made (SC-008)
- [ ] U030 [US1] `apps/web/src/import/`: the import preview panel and 3D overlay for imported seams, using a colour-blind-safe palette with room for the extra-seam and region layers (FR-040); accept and decline go through T107. Makes U029 pass

**Checkpoint**: a cut capsule goes from import to pattern without drawing a seam. Dead-ended cuts still import as plain cuts until US4 makes them dart proposals, so this state isn't releasable on its own.

---

## Phase 4: User Story 2: pieces that can't lie flat are fixed in the same step (P1)

**Goal**: the preview already contains the fewest extra seams that make every chart a disk, highlighted differently, with one "accept all" action.

**Independent test**: import `sphere-band.glb`; the preview holds the cut plus extra seams; after "accept all", every piece flattens.

- [ ] U031 [US2] `crates/openplush-pipeline/tests/source_closing.rs`: failing tests on `sphere-band.glb` and testkit meshes. `closing` is non-empty and holds only `CloseChart` proposals, each with a non-empty plain-language explanation (FR-031). After applying `seams` and all of `closing`, every chart passes T075's Euler check (SC-003); that check is the oracle, independent of `propose_cuts`. A closed surface with no cuts still gets at least one closing cut. When every chart is already a disk, `closing` is empty and the report says so (US2 scenario 4). `closing` is computed against `existing` ∪ `seams`
- [ ] U032 [US2] `crates/openplush-pipeline/src/source.rs`: adds `closing: Vec<Proposal>` to `SourceImport`, built from T076's `propose_cuts` over the charts under `existing` ∪ `seams`, in chart order, with rustdoc. Makes U031 pass
- [ ] U033 [US2] `crates/openplush-pipeline/tests/source_accept.rs`: failing tests. "Accept all" (seams, then each closing proposal) is one T128 undo entry, and undoing restores the prior `SeamSet` exactly (FR-041). Accepting `seams` without `closing` leaves 001's validation raising `ChartNotDisk` for the unflattenable chart (US2 scenario 3)
- [ ] U034 [US2] `crates/openplush-pipeline/src/proposals.rs`: apply several offered proposals as one undo step, through T107 and T128 and adding no other apply path. Makes U033 pass
- [ ] U035 [P] [US2] `bindings/openplush-cli/tests/pattern_seams_from_model.rs`: failing tests for quickstart scenario 3. `pattern --seams-from-model --accept-closing` produces pieces. Without `--accept-closing`, it exits with the `EngineError` code and the `ChartNotDisk` JSON with proposals (S-010)
- [ ] U036 [US2] `bindings/openplush-cli/src/commands/pattern.rs`: the `--seams-from-model` and `--accept-closing` flags, plus `docs/reference/cli.md`. Makes U035 pass
- [ ] U037 [P] [US2] `bindings/openplush-wasm/tests/source_import.rs` and `packages/engine-client/src/source-import.test.ts`: extend the DTO and client tests to `closing`. Implementation: `bindings/openplush-wasm/src/` DTO field, regenerated types, `docs/reference/worker-protocol.md`. One PR per package: the WASM test and implementation, then the client
- [ ] U038 [P] [US2] `apps/web/tests/import-preview.spec.ts`: failing Playwright tests on `sphere-band.glb`. Extra seams look different from imported ones, and each shows its explanation. "Accept all" is one undo step. "Accept without extra seams" applies only the imported seams. Both work from the keyboard. Implementation in `apps/web/src/import/` follows in the same PR as a second commit

**Checkpoint**: an imperfect UV layout reaches a valid pattern in three actions (SC-006).

---

## Phase 5: User Story 3: my materials become my fabrics (P1)

**Goal**: each material becomes a fabric region, material boundaries become seams, and an unmatched name gets a minky-valued profile flagged until edited.

**Independent test**: import `three-materials.glb`. There are three regions. "minky / cuddle" and "Polar fleece " match the "Minky / cuddle" and "Polar fleece" presets, and "Felt Nose" is created with minky's values and an `UnreviewedFabric` warning. Every material boundary is a seam.

### Seams (`openplush-seams`)

- [x] U039 [P] [US3] `crates/openplush-seams/tests/source_boundaries.rs`: failing tests. An interior edge is a material boundary when both faces come from the source and differ in `Option<MaterialIndex>`; a source face with no material next to a material face counts. An edge touching a repair-filled face is never a boundary (FR-024). An edge that is both a UV cut and a boundary appears once, as `FabricBoundary` (S-005), and the shared count includes it. Two materials that later map to one fabric still give a boundary — *as built, also: faces of one material from two primitives give none, a boundary already in the `SeamSet` is left out, and the proposal's explanation counts the `Cut` and `FabricBoundary` edges*
- [x] U040 [US3] `crates/openplush-seams/src/source.rs`: material-boundary detection merged with the UV cuts into the one seam proposal, with `FabricBoundary` winning on shared edges. Makes U039 pass — *`UvCuts::material_boundaries()` and `shared_edges()`, computed by `uv_cuts`, whose `proposal()` now carries both kinds; the UV-cut tests pass unchanged. Documented in `docs/reference/crates/openplush-seams.md`*

### Regions (`openplush-pattern`)

- [ ] U041 [P] [US3] `crates/openplush-pattern/tests/source_regions.rs`: failing tests. Matching is `trim` plus `str::to_lowercase` against each profile name treated the same way, first match in project order wins, and nothing fuzzy ("minky_fabric" matches nothing) (S-006). Case folding doesn't depend on the locale (a name with "İ" folds the same everywhere). An unmatched name gives `Fabric::with_defaults(name)`, equal to the minky preset except for the name (S-007). Source faces with no material form one "No material" region, matched like any other name (FR-020). With no material on any face there are no regions (FR-005). With one material there is one region. A `proptest` checks that every face is in exactly one region, regions are ordered by `MaterialIndex` with "No material" last, and faces in each region ascend (SC-005). A repair-filled area joins the region bordering the largest share of its perimeter by length; ties go to the lowest `MaterialIndex`, then "No material" (data-model §3)
- [ ] U042 [US3] `crates/openplush-pattern/src/regions.rs` and `src/fabric.rs`: regions from materials beside T137's painted regions, name matching, and `Fabric::with_defaults(name)`. Makes U041 pass

### Stage (`openplush-pipeline`)

- [ ] U043 [US3] `crates/openplush-pipeline/tests/source_import.rs`: failing tests on `three-materials.glb` (quickstart scenario 4). Three regions; `matched` lists the "Minky / cuddle" and "Polar fleece" presets, `created` lists Felt Nose; `new_fabrics` holds one profile; `material_boundary_edges`, `shared_edges` and `filled_areas` are reported. Snapshot the report with `insta`
- [ ] U044 [US3] `crates/openplush-pipeline/src/source.rs`: adds `regions`, `new_fabrics`, and the report's material fields, with rustdoc. "Accept all" now also applies `set_fabrics` inside U034's single undo step (FR-041). Makes U043 pass

### Unreviewed-fabric warning (FR-025)

- [ ] U045 [P] [US3] `crates/openplush-project/tests/from_material.rs`: failing tests. Optional `fabrics[].fromMaterial` defaults to `false`; a v1 file without it loads unchanged; a profile created by import has it set; the first edit to that profile clears it
- [ ] U046 [US3] needs:T124 `crates/openplush-project/src/document.rs` and the project schema page in `docs/reference/`: the field (already in `specs/001-pattern-pipeline/contracts/project.schema.json`) and the clear-on-edit rule. Makes U045 pass. Depends on T124
- [ ] U047 [US3] `crates/openplush-pipeline/src/warning/tests.rs`: failing tests. `WarningCode::UnreviewedFabric` (target `Project`, `Remedy::Advisory`, text "Pick a preset for, or edit, the fabric '<name>'") is raised once per fabric with `fromMaterial`, gone after an edit, and present in the warnings printed on the pattern (FR-025)
- [ ] U048 [US3] `crates/openplush-pipeline/src/warning/mod.rs`: the warning code and its string in `docs/reference/engine-api.md`. Makes U047 pass

### Shell

- [ ] U049 [P] [US3] `bindings/openplush-cli/src/commands/import_seams.rs`, `bindings/openplush-wasm/src/` and `packages/engine-client/src/client.ts`: extend `bindings/openplush-cli/tests/import_seams.rs` (scenario 4 JSON), `bindings/openplush-wasm/tests/source_import.rs` and `packages/engine-client/src/source-import.test.ts` to regions, new fabrics and the material report. Then the DTO fields, regenerated types and `docs/reference/worker-protocol.md`. One PR per package, each test first
- [ ] U050 [P] [US3] `apps/web/tests/import-preview.spec.ts`: failing Playwright tests on `three-materials.glb`. A region overlay distinct from both seam layers (FR-040). Created profiles are listed with an edit action. Repainting an accepted region behaves like hand painting (US3 scenario 5). Implementation in `apps/web/src/import/` as the PR's second commit

**Checkpoint**: one import gives seams, regions and fabrics; an unreviewed guess is printed on the pattern.

---

## Phase 6: User Story 4: a cut that stops partway is shown as a dart, not guessed at (P2)

**Goal**: each dead-ended cut becomes its own located, explained dart proposal, apart from the imported seams; accepting adds a dart, and declining leaves that chain uncut (S-014).

**Independent test**: import `dead-end.glb`. Exactly two dart proposals, one per tip in `dead-end.tips.json`; none for cut A; the imported seams don't contain the dart chains.

- [x] U051 [P] [US4] `crates/openplush-seams/tests/source_dead_ends.rs`: failing tests. *As built, the meshes are sheets and a closed globe whose corner UVs are cut along chosen paths by the fixture generator's wedge rule (`wedge_uvs`, `Grid::cut_sheet` in `tests/common`). The fixture half, `dead-end.glb` and `.obj` against `dead-end.tips.json`, is `crates/openplush-pipeline/tests/source_dead_end_fixture.rs`, because reading the files needs `openplush-io`. FR-012 changed `source_props.rs`' property: the seam proposal and the darts together hold every cut exactly once*
  - Tips are interior vertices of seam-valence 1 in T070's graph over UV cuts ∪ material boundaries ∪ mandatory seams, with surface-boundary vertices as endpoints (research R3). A cut ending on another seam, a material boundary, an opening or a mandatory seam is not a tip (FR-013, US4 scenario 3). A seam already in the project doesn't rescue a tip (S-012).
  - Each tip's chain runs from the tip to the first vertex of seam-valence 3 or more, or to the surface boundary. There is one chain per tip, tips ascend, and a slit with two free ends is one chain, owned by its lower tip.
  - A pole-to-pole meridian on a closed sphere gives two tips, one per pole. Its chains meet at the equator junction if there is one; otherwise it is a single two-tip chain.
  - Once T060 and T067 land, also `dead-end.glb` against `dead-end.tips.json`.
- [x] U052 [US4] `crates/openplush-seams/src/source.rs` and `src/proposal.rs`: *As built, `UvCuts::darts() -> Result<Vec<Proposal>, SeamsError>`, computed by `uv_cuts`. Tips are `SeamGraph`'s `DartTip` nodes over the cuts and boundaries (one kind) and the `Mandatory` seams in `seams` (their own kind, so a cut meeting one ends there). A mandatory seam's own free end is never a dart. `proposal()` leaves dart edges out. Documented in `docs/reference/crates/openplush-seams.md`*
  - The dead-end chains, reusing T070's free-endpoint classification with no second definition of an endpoint.
  - `SeamReason::SourceDart` and one `Proposal` per chain, every edge `SeamKind::Dart`. Its explanation names the tip and says the cut stops partway, so it will be sewn as a dart, and if not meant the maker can decline it and fix the cut in the modelling tool.
  - The chains are removed from the UV-cut seam proposal (FR-012).
  - Makes U051 pass.
- [ ] U053 [US4] `crates/openplush-pipeline/tests/source_import.rs`: failing tests.
  - `SourceImport::darts` holds one proposal per tip, in tip order. `seams` and `darts` share no edge.
  - Accepting `seams` without a dart leaves its chain uncut (FR-012), and "accept all" applies the darts in the same undo step (FR-041).
  - `closing` is computed against existing ∪ `seams` ∪ `darts`.
  - The explanation is the documented string.
- [ ] U054 [US4] `crates/openplush-pipeline/src/source.rs`: the `darts` field, with rustdoc; the explanation string in `docs/reference/engine-api.md`; U034's accept-all extended to darts. Makes U053 pass
- [ ] U055 [P] [US4] `bindings/openplush-cli/tests/import_seams.rs`, `bindings/openplush-wasm/src/` and `packages/engine-client/src/client.ts`:
  - The CLI test, quickstart scenario 5: two dart proposals in the JSON.
  - `pattern --seams-from-model` declines darts unless `--accept-darts` is given, and the report lists them as declined.
  - Then the `darts` DTO field in WASM and the client.
  - One PR per package, each test first, with `docs/reference/cli.md` and `worker-protocol.md`.
- [ ] U056 [P] [US4] `apps/web/tests/import-preview.spec.ts`: failing Playwright tests on `dead-end.glb`.
  - Dart proposals are drawn distinctly from imported and extra seams, with their count (FR-015).
  - Selecting one highlights its chain and shows its explanation.
  - "Accept all" includes the darts. Declining one leaves its chain uncut and the rest of the import intact (US4 scenario 2).
  - Everything is keyboard-operable.
  - Implementation in `apps/web/src/import/` as the second commit.

**Checkpoint**: the feature's engine half is releasable.

---

## Phase 7: Re-import (FR-050 to FR-052)

**Goal**: importing into a project that already has seams or regions asks Replace or Merge, shows the difference first, and never drops a seam silently. No story label: see spec gap 2.

- [ ] U057 [P] `crates/openplush-project/tests/seam_origin.rs`: failing tests. Optional `seams[].origin` (`"drawn" | "imported" | "proposed"`) defaults to `"drawn"`; v1 files load unchanged; any other value is a typed parse error (S-009)
- [ ] U058 `crates/openplush-project/src/document.rs` and the schema page in `docs/reference/`: the field (already in `project.schema.json`). Accepting a source import marks its seams `imported`, and closing seams `proposed`. Makes U057 pass
- [ ] U059 `crates/openplush-pipeline/tests/reimport.rs`: failing tests for `reimport_diff(old, new_mesh, import, on_existing)`. Old seams and region faces re-place by position within the weld tolerance, never by provenance key or nearest edge (S-008). Under Merge, an unplaceable seam or face is listed in `unplaceable_*` and located on the old mesh, and an imported seam on an existing seam isn't counted twice. Under Replace, `removed_seams` counts all existing seams and `removed_hand_drawn` only `origin = drawn`. The stage is pure. A seam edge split once by a new vertex on it re-places as its two halves, a non-axis-aligned edge split at its `f32`-rounded midpoint also re-places, and one whose new vertex lies off it beyond the tolerance doesn't (S-017). A re-placed split edge counts as one placed seam, and when two paths qualify the documented tie-break picks the same one on every run. With every vertex moved, `placed_seams` is 0 and every old seam is unplaceable, under Replace and Merge alike (S-018). U007's v1 and v2 re-place every seam away from the stretched cap, as Q4 measured
- [ ] U060 `crates/openplush-pipeline/src/source.rs` (split into `src/source/` if it exceeds 400 lines): `OnExisting`, `ReimportDiff`, and `reimport_diff`, with rustdoc and `docs/reference/engine-api.md`, including FR-051's nothing-placed hint text. Makes U059 pass
- [ ] U061 `crates/openplush-pipeline/tests/source_accept.rs`: failing tests. Replace and Merge each apply as one undo step; declining leaves the project byte-identical (FR-052). Implementation in `src/proposals.rs` as the second commit
- [ ] U062 [P] `bindings/openplush-cli/tests/import_seams.rs` and `src/commands/import_seams.rs`, `bindings/openplush-wasm/src/` and `packages/engine-client/src/client.ts`: `import-seams --project <old> --on-existing <replace|merge>` (quickstart scenario 7), then the `reimportDiff` operation. Test first in each package, one PR per package, with `docs/reference/cli.md` and `worker-protocol.md`
- [ ] U063 [P] `apps/web/tests/reimport.spec.ts`: failing Playwright tests. Replace and Merge are offered, neither preselected. The diff shows before commit, with hand-drawn removals counted. Unplaceable seams are located on the previous model. Keyboard-operable. Implementation in `apps/web/src/import/` as the second commit

---

## Phase 8: Polish and cross-cutting

- [ ] U064 [P] `crates/openplush-pipeline/benches/source_import.rs`: UV-cut and boundary reading on a 50,000-triangle testkit mesh with UVs and three materials. Record the time in the PR against SC-009's 1 s budget; a miss is a finding, not a tuning knob
- [ ] U065 [P] `.github/workflows/determinism.yml` (T017): add quickstart scenario 8's native-vs-WASM byte comparison for `three-materials.glb` (SC-007)
- [ ] U066 [P] `docs/how-to/import-seams-from-a-model.md`: what counts as a cut, why a texture-only cut becomes a seam (keep texture cuts on a separate UV map), dead ends and how to fix them, and how names match fabrics
- [ ] U067 Run quickstart.md scenarios 1 to 9 end to end, and point each implemented spec section to its as-built docs (ADR 0003)
- [ ] U068 needs:U021 `specs/001-pattern-pipeline/contracts/engine-api.md` and `specs/001-pattern-pipeline/plan.md`: spec-only PR once U021 merges. Bump 001's engine-api version, add the Decision Ledger entry, and add the one-line pointer to this feature's contract (contract preamble). Separate from U021 because spec and implementation never share a PR
- [ ] U072 [P] `crates/openplush-seams/src/source.rs`, `src/lib.rs`, the `source_*` tests in `crates/openplush-seams/tests/` and `crates/openplush-pipeline/tests/`, and `docs/reference/crates/openplush-seams.md`: rename `uv_cuts`/`UvCuts` to `source_seams`/`SourceSeams`, since the result now holds UV cuts, material boundaries, darts and projection points. A refactor with no behaviour change, so no failing-test commit: every test passes before and after, renamed with it. `tests/source_uv_cuts.rs` keeps its name because it tests the UV cuts. Requested by the maintainer on 2026-10-21

---

## Dependencies and execution order

```text
Phase 0 fixtures U001–U007; ADR U069 ─▶ spikes U008, U009, U011 (U010 answered by 001 D-034)
                                   │ Q1
Phase 2  U012→U013 (mesh)          │
         U014→U015 (io: OBJ and glTF)
                                   ▼
US1  U016,U017 → U018 (after U008) → U019 → U070→U071 → U020→U021 (after U009) → U022→U023, U025→U026 → U027→U028 → U029→U030 → U024 (after spec gap 1)
US2  (after U021) U031→U032 → U033→U034 → U035→U036, U037, U038
US3  (after U013, U015, U019) U039→U040, U041→U042 → U043→U044; U045→U046 → U047→U048; U049, U050
US4  (after U019, U040) U051→U052 → U053→U054 → U055, U056
Re-import (after U044, T124–T126) U057→U058 → U059→U060 → U061 → U062, U063
Polish after US4; U068 after U021 merges
```

- Within a story: test task, then implementation, then the next layer up (seams, then pattern, then pipeline, CLI, WASM, client, web).
- US2 and US3 both extend `pipeline/src/source.rs` and can run in parallel only if their slices merge in order. Stack the later one on the earlier.
- US4 needs US3's material boundaries (U040), because FR-013 judges dead ends on cuts ∪ boundaries.

### Parallel examples

- **Phase 0**: U001 first (it adds the glTF writer), then U002 to U007 and U069 together; U009 beside U008.
- **Phase 2**: U012 and U014 (different crates) together.
- **US1**: U016 and U017 together; U022, U025 and U027 together once U021 merges.
- **US3**: U039 (seams), U041 (pattern) and U045 (project) together; they share no file.
- **US4**: U051 beside US3's shell tasks.

## Implementation strategy

- **MVP**: Phase 0, Phase 2, US1, US3's material boundaries (U039, U040), and US4's engine tasks (U051 to U054). The boundaries come along because FR-013 judges dead ends on cuts ∪ boundaries. US1 alone would import dead-ended cuts as plain seams instead of dart proposals (FR-011), so US1 isn't shipped to makers without U054.
- **Then** US2 (most real UV layouts need closing seams), US3, the rest of US4's shell, and re-import.
- **Release** (v0.2, plan Phasing) needs every phase, and the U064 timing within SC-009.

## Notes

- U024 stays blocked until spec gap 1 closes.
- Never update an `insta` snapshot to pass a test (AGENTS.md). New snapshots in U020 and U043 are reviewed as part of their PRs.
