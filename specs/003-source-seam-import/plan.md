# Implementation Plan: Import Seams and Fabric Regions from the Source Model

**Branch**: `003-source-seam-import` | **Date**: 2026-10-16 | **Status**: Draft
**Spec**: [spec.md](./spec.md) · **Research**: [research.md](./research.md) · **Data model**: [data-model.md](./data-model.md) · **Contract**: [contracts/source-import.md](./contracts/source-import.md)

**Input**: Feature specification from `/specs/003-source-seam-import/spec.md`

## Summary

When a maker imports a model they already cut and organised in a modelling tool, read its UV cuts as seams and its materials as fabric regions. Offer both as one proposal, with any extra seams needed to make every piece lie flat already highlighted and one "accept all" action.

**Technically this is two new modules, one new stage, and one small extension to 001's soup.** No new crate and no new dependency. The signals are already in 001's data: per-corner UVs survive the weld, and source provenance names the primitive each face came from. 001 just never read them. The one gap is that 001's `TriangleSoup` and `ManifoldMesh` have nowhere to put material names, even though 001 FR-002 promises to keep material assignment (this feature's data-model §1).

## Technical Context

| Field | Value |
|---|---|
| Language/Version | Rust stable (same MSRV as 001); TypeScript 5.x `strict` |
| Primary dependencies | **None new.** `openplush-mesh`, `openplush-io`, `openplush-seams`, `openplush-pattern`, `openplush-pipeline` from 001; `three` for the preview overlay |
| Storage | Accepted results are ordinary `seams[]`, `fabrics[]` and `fabricRegions[]` entries. One additive, optional field (`seams[].origin`), so no schema version bump |
| Testing | `cargo test`, `proptest` (no imported dart; every face in one region), `insta` (import-report goldens), Vitest, Playwright |
| Target platform | Same as 001: evergreen desktop browsers (WASM, Web Worker) and the native CLI |
| Project type | Additive feature in the existing Rust + pnpm monorepo |
| Performance goals | SC-009: under 1 s added to a 50k-triangle import. The work is one pass over edges plus 001's `propose_cuts` |
| Constraints | Deterministic output; never mutates the mesh or the `SeamSet`; offline; no telemetry |
| Scale/scope | Same mesh bounds as 001 |

### Hard dependency on 001

From 001's `tasks.md`:
- **T050, T051, T053**: importers that record materials. This feature extends their output (data-model §1); it doesn't replace them.
- **T060, T066**: weld and hole fill.
- **T069–T076**: the seam set, seam graph, charts, topology and `propose_cuts`.
- **T084**: `Fabric`.
- **T107**: proposal preview-then-accept.
- **T124–T125**: the project document and provenance keys.
- **T128**: undo.
- **T137**: regions.

Work can start partway through 001's Phase 3, like 002. The material extension (data-model §1) is two tasks in two crates, because one PR never spans two: first `openplush-mesh` (`SourceMaterials` on the soup and `ManifoldMesh`, carried by repair), then `openplush-io` (importers fill it, alongside T053).

## Constitution Check

*GATE: must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Status |
|---|---|
| I Engine/Shell separation | ✅ Detection and matching are in `crates/`; the preview is `apps/web` only. The shell reaches the stage through `openplush-pipeline` |
| II CLI parity | ✅ `import-seams` subcommand, plus `--seams-from-model` and `--accept-closing` on `pattern` (contract §3). Headless never accepts a proposal without a flag naming it |
| III Invariants as contracts | ✅ A dead-ended cut is a typed error (`DeadEndCut`), never dropped or turned into a dart. Imported seams then pass 001's chart validation like any other |
| IV Test-first oracles | ✅ Independent oracles: fixtures with a recorded cut layout (SC-001); a hard-shaded twin that must give the identical edge set (SC-002); a property test that no `Dart` or `Opening` kind is ever produced |
| V Determinism | ✅ Edges and faces in index order, regions by `MaterialIndex`, name matching locale-free, ties broken by index (data-model §3) |
| VI Units | ✅ No physical quantity is introduced. `UV_CUT` is dimensionless and documented as such beside the other tolerances |
| VII Explicit repair / never silent mutation | ✅ The stage is a pure read; imported and extra seams are `Proposal`s; "accept all" is the maker's action (spec Q3). This is the principle the maintainer's original "fix it on its own" request ran into, resolved without amending it |
| VIII Local-first | ✅ Runs in the worker; no network |
| IX License | ✅ No new dependencies |
| X Baseline before cleverness | ✅ Exact reading of the file; no inference beyond the R2 tolerance |

No violations. Complexity Tracking omitted.

**Re-check after Phase 1**: still clean. The design added one public error variant, one stage pair, and an optional schema field, none of which touches a principle. The positional re-placement in research R5 exists *because* of Principle III: re-using stale provenance keys would put seams on the wrong edges without telling anyone.

## Project Structure

### Documentation (this feature)

```text
specs/003-source-seam-import/
├── spec.md
├── plan.md                    # this file
├── research.md                # what the files carry, UV-cut test, dead ends, name matching, re-import
├── data-model.md              # soup extension, SourceImport, DeadEndCut, ReimportDiff
├── quickstart.md
├── contracts/source-import.md # stage signatures, error, CLI, worker op
├── checklists/requirements.md
└── tasks.md                   # task list (U001–U067)
```

### Source code

```text
crates/openplush-geom/src/epsilon.rs        # + UV_CUT
crates/openplush-mesh/src/soup.rs           # + SourceMaterials, extra_uv_sets, MaterialIndex
crates/openplush-io/src/{obj,gltf}.rs       # fill them; OBJ splits primitives per material (spike Q3)
crates/openplush-seams/src/source.rs        # NEW: UV cuts, material boundaries, dead-end check
crates/openplush-seams/src/proposal.rs      # + SeamReason::SourceModel
crates/openplush-pattern/src/regions.rs     # + regions from materials, name matching, default profile
crates/openplush-pattern/src/fabric.rs      # + Fabric::with_defaults(name) (research R4)
crates/openplush-pipeline/src/source.rs     # NEW: source_import and reimport_diff stages
crates/openplush-pipeline/src/error.rs      # + EngineError::DeadEndCut
crates/openplush-project/                   # + seams[].origin (optional)
bindings/openplush-cli/src/commands/        # + import-seams; pattern --seams-from-model --accept-closing
bindings/openplush-wasm/                    # + sourceImport, reimportDiff DTOs
apps/web/src/import/                        # NEW: import preview, accept all / partial / decline
fixtures/meshes/source-seams/               # NEW: quickstart's fixture table
```

**Structure decision**: no new crate. Unlike 002's detectors, this feature has no signal of its own to compute and cache. It reads two attributes and hands them to machinery 001 already has. Each piece goes in the crate that owns the concept it touches:
- the soup in `openplush-mesh`;
- seams and the seam graph in `openplush-seams`;
- fabrics and regions in `openplush-pattern`;
- composition in `openplush-pipeline`.

A new crate would need to depend on both `seams` and `pattern` only to glue them, which is the pipeline's job. No new dependency edge.

## Architecture

### The stage

```text
ManifoldMesh ─┬─▶ seams::source ─▶ UV cuts, material boundaries ─▶ seam graph ─▶ dead end? ─▶ Err(DeadEndCut)
              │                                                                       │ no
              │                                                                       ▼
              │                                              Proposal (SourceModel), propose_cuts ─▶ closing
              └─▶ pattern::regions ─▶ regions by material, name match / Fabric::with_defaults
                                                     │
                                         pipeline::source_import ─▶ SourceImport { seams, closing, regions, new_fabrics, report }
```

The dead-end check runs before anything is proposed, so FR-012's "nothing from it is proposed" is structural: no `SourceImport` exists when the error fires.

### Accepting

"Accept all" is three edits in one undo step: apply `seams`, apply each `closing` proposal, then `set_fabrics` with the regions and new profiles. It reuses T107's apply path, so imported seams become indistinguishable from drawn ones (FR-043) except for the optional `origin` field.

### Re-import

`reimport_diff` re-places the old project's seams and region faces on the new mesh **by position** (research R5), because provenance keys don't survive re-export. Anything that can't be placed is listed and located on the old mesh, never dropped silently. The spec's FR-051 gains one sentence saying so, in the same change as this plan.

## Phasing

| Release | Scope |
|---|---|
| v0.2 | The whole feature. It needs 001's flatten and propose_cuts, so it can't precede them |
| Later (stretch goal) | **A Blender add-on** that exports OpenPlush seams and fabric assignments directly: Mark Seam flags, darts and openings marked as such, and each material mapped to a fabric preset by the maker in Blender. It would replace this feature's inferences (UV cuts as seams, the `UV_CUT` tolerance, name matching, default profiles, the `tobj` question) with explicit data, and could express darts, which a UV export cannot. It builds on 001's v0.3 `openplush-py` plus add-on plan and needs its own feature spec; this feature stays useful for models from other tools |

## Decision Ledger

| Date | ID | Decision | Status |
|---|---|---|---|
| 2026-10-16 | S-001 | **No new crate.** Detection in `openplush-seams`, regions in `openplush-pattern`, composition in `openplush-pipeline`. No new dependency edge | Accepted |
| 2026-10-16 | S-002 | **Materials are keyed by source primitive**, with the new invariant that a primitive has at most one material. No per-face array. The OBJ importer splits primitives at material changes if `tobj` doesn't (spike Q3) | Accepted. Changes 001 data-model §2; a mesh task, then an io task alongside T053 |
| 2026-10-16 | S-003 | **UV cut = endpoint UV mismatch beyond an absolute `UV_CUT`, with UVs on both sides.** Provisional `1e-5`; spike Q1 confirms | Accepted, research R2 |
| 2026-10-16 | S-004 | **Dead ends are judged on the seam graph over UV cuts ∪ material boundaries ∪ mandatory seams**, reusing 001's node classification. All tips are reported in one `DeadEndCut` | Accepted, research R3, spec FR-011 and FR-013 |
| 2026-10-16 | S-005 | **An edge that is both a UV cut and a material boundary becomes one `FabricBoundary` seam** | Accepted, spec FR-021 |
| 2026-10-16 | S-006 | **Name match = Unicode-lowercase, trimmed, first in project order.** No fuzzy matching | Accepted, research R4 |
| 2026-10-16 | S-007 | **New profiles copy the minky preset** under the material's name, via one `Fabric::with_defaults(name)` that 001's "add fabric" can also use, and carry an `UnreviewedFabric` advisory warning until edited (spec FR-025) | Accepted. Maintainer, 2026-10-16 (research R4) |
| 2026-10-16 | S-008 | **Re-import re-places old seams and regions by position, not provenance key.** Unplaceable ones are listed and located; spec FR-051 amended to say so | Accepted, research R5; spike Q4 measures it |
| 2026-10-16 | S-009 | **Optional `seams[].origin`** (`drawn`/`imported`/`proposed`, default `drawn`), so FR-051 can count hand-drawn seams. Additive; v1 files unchanged | Accepted |
| 2026-10-16 | S-010 | **Headless never accepts a proposal implicitly.** `--accept-closing` names the act | Accepted, Constitution II and VII |
| 2026-10-16 | S-011 | **`ManifoldMesh` carries `SourceMaterials` and `extra_uv_sets`**, moved unchanged from the soup by repair, so stages that take the mesh can read them. A repair-filled face is distinguishable from a source face with no material, so filled holes never make material-boundary seams | Accepted. Changes 001's `ManifoldMesh`; review of #99 |
| 2026-10-16 | S-012 | **Dead ends ignore seams already in the project.** The file must be consistent on its own, because Replace would discard those seams | Accepted, spec FR-013; review of #99 |
| 2026-10-16 | S-013 | **FR-014's undersized-piece mark comes from 001's piece-size check, run by the preview on the flattened, scaled proposal, not from `SourceImport`.** The minimum (001 data-model §5: provisional 20 mm in each principal direction at target scale) is measured on flat pieces in mm, and `source_import` runs before flattening and takes no scale. Same route as 002 FR-036 (H049). 001 has no task for that check yet; both features need one | Proposed, found while writing tasks.md (#107) |

### Trade-offs accepted

- **A cut placed for texturing alone becomes a sewing seam.** The file can't distinguish the two. The preview is the safeguard, and the maker can decline or erase. Advising makers to keep texture cuts and sewing cuts on separate UV maps would be a docs task, not code.
- **Positional re-placement fails on edited regions by design.** A seam on a limb the maker reshaped won't re-place. It is listed, not guessed.
- **One more tolerance kind** (dimensionless) in `epsilon`. Accepted rather than pretending UV space is model space.

### Open questions

- Spikes Q1–Q4 ([research R6](./research.md)).
- 001's own "add a fabric profile by hand" has no default defined either. S-007 fills that gap for both features; 001's plan should record it when T084 lands.
