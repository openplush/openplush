# Implementation Plan: Seam-Placement Hints

**Branch**: `002-seam-placement-hints` | **Date**: 2026-10-13 | **Status**: Draft
**Spec**: [spec.md](./spec.md) · **Research**: [research.md](./research.md) · **Data model**: [data-model.md](./data-model.md)

**Input**: Feature specification from `/specs/002-seam-placement-hints/spec.md`

## Summary

Inspect the model and show the maker where a seam, dart, or edge would help, as four toggleable categories with a sensitivity control: sharp edges, concave creases, dart candidates, and the left/right symmetry plane. Hints appear as a 3D overlay and an inspection panel, explain themselves in plain language, convert to a seam or dart in one action with a flat-pattern preview, and are never applied on their own.

**Technically this is one new engine crate and one new UI panel.** No new dependencies, no new architectural layer — every signal it needs (dihedral angles, Gaussian curvature, symmetry, the proposal mechanism, seam creation, flattening) was already specified for 001. See [research R4](./research.md).

The feature exists because drawing good seams is the hardest skill a new plush maker faces, and it is the groundwork for the automatic seam layout deferred to v0.2 (001 FR-015, D-015).

## Technical Context

| Field | Value |
|---|---|
| Language/Version | Rust stable (same MSRV as 001); TypeScript 5.x `strict` |
| Primary dependencies | **None new.** Reuses `openplush-fields`, `openplush-mesh`, `openplush-seams`, `openplush-pipeline` from 001; `three` for the overlay |
| Storage | Hint decisions and settings in the existing project document — already present in `project.schema.json` v1, so no migration |
| Testing | `cargo test`, `proptest` (monotonicity), `insta` (hint-set goldens), Vitest, Playwright |
| Target platform | Same as 001: evergreen desktop browsers (WASM, Web Worker) plus native CLI |
| Project type | Additive feature in the existing Rust + pnpm monorepo |
| Performance goals | SC-003: hints within 5 s at 50k triangles; sensitivity change reflected within 1 s |
| Constraints | Deterministic hint sets; never mutates geometry; offline; no telemetry |
| Scale/scope | Same mesh bounds as 001; hint count capped per category for panel readability (SC-010) |

### Hard dependency on 001

**This feature cannot ship before 001's flattening stage works** (spec Clarifications, Q3). FR-031's preview is a flat-pattern preview, which is 001's `flatten` plus `pattern`. There is deliberately no 3D-only fallback — seeing the pattern consequence is the part that teaches, and a hints feature that cannot show it would be the decorative half of the idea.

Concretely, in 001's `tasks.md`: this feature needs **T069–T077** (seams and charts), **001 T078–T083** (flatten), **001 T084–T087** (pattern), **001 T107** (proposal preview-then-accept), and **001 T144** (curvature). It can begin once those are green, which is partway through 001's Phase 3 rather than after all of 001.

## Constitution Check

*GATE: must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Status |
|---|---|
| I Engine/Shell separation | ✅ Detection is pure geometry in `crates/openplush-hints`; the overlay and panel are `apps/web` only. No browser API reaches the crate |
| II CLI parity | ✅ `openplush-cli hints <model> --json` emits the full hint report (FR-050) |
| III Invariants as contracts | ✅ Detection reads the surface and cannot fail an invariant; accepting a hint goes through 001's existing seam validation, so an accepted hint that would break a chart surfaces as 001's typed error |
| IV Test-first oracles | ✅ Independent oracles available: a smooth sphere must yield zero crease hints (SC-004), a cube must yield exactly its 12 edges, symmetry must be found on symmetric fixtures and refused on asymmetric ones (SC-005) |
| V Determinism | ✅ Identical hint report byte-for-byte across runs and platforms (SC-008); hints sorted by (category, provenance key) |
| VI Units | ✅ Sensitivity thresholds are physical — millimetres and degrees (FR-007), so the same control means the same thing on a keychain and a large plush |
| VII Explicit repair / never silent mutation | ✅ FR-010 forbids any geometry change; FR-032 makes every hint a proposal until accepted |
| VIII Local-first | ✅ FR-051; detection runs in the same worker as the rest of the engine |
| IX License | ✅ No new dependencies, so no new license surface and no ADR required |
| X Baseline before cleverness | ✅ This **is** the deterministic geometric baseline. The spec's out-of-scope list explicitly excludes learned models, and a future `FeatureDetector` trait can sit behind the same hint API |

No violations. Complexity Tracking omitted — nothing to justify.

## Project Structure

### Documentation (this feature)

```text
specs/002-seam-placement-hints/
├── spec.md
├── plan.md                  # this file
├── research.md              # the four detectors, monotonicity, identity
├── data-model.md            # Hint, HintCategory, SymmetryPlane, HintDecision
├── quickstart.md
├── checklists/requirements.md
└── tasks.md
```

No `contracts/` directory. This feature extends two existing contracts rather than adding a surface of its own:
- `specs/001-pattern-pipeline/contracts/engine-api.md` gains the `hints` stage.
- `specs/001-pattern-pipeline/contracts/worker-protocol.md` already lists the `hints` op.
- `project.schema.json` v1 already carries `dismissedHints` and `hintSettings`, so **no schema migration is needed** — that was deliberate when the schema was written.

### Source code

```text
crates/
└── openplush-hints/              # NEW — the only new crate
    ├── src/
    │   ├── lib.rs                # detect() entry point, HintReport
    │   ├── hint.rs               # Hint, HintCategory, HintId, HintKind
    │   ├── sensitivity.rs        # s -> threshold mapping; monotonicity contract
    │   ├── crease.rs             # sharp edges + concave creases (shared signal)
    │   ├── chain.rs              # crease edges -> polylines, junction handling
    │   ├── dart.rs               # integrated Gaussian curvature, clustering
    │   ├── symmetry.rs           # PCA candidates + reflection-ICP + score
    │   ├── identity.rs           # provenance keys, re-match after model change
    │   └── explain.rs            # plain-language text per hint (FR-022)
    └── tests/
        ├── oracles.rs            # sphere/cube/symmetry oracles
        ├── monotonicity.rs       # proptest: s1 < s2 implies hints(s1) subset of hints(s2)
        └── determinism.rs

crates/openplush-pipeline/        # extended: hints stage, HintSession
crates/openplush-project/         # extended: hint decisions + settings persistence
bindings/openplush-cli/           # extended: `hints` subcommand
apps/web/src/hints/               # NEW — overlay + inspection panel
packages/engine-client/           # extended: hints op in the typed protocol
fixtures/meshes/                  # extended: quadruped, asymmetric, faceted fixtures
```

**Structure Decision**: a separate `openplush-hints` crate rather than folding detection into `openplush-fields` or `openplush-seams`. Three reasons: it keeps the crate DAG acyclic (hints depends on fields and mesh, and nothing depends on hints except the pipeline); it keeps the feature independently testable; and it gives v0.2's automatic seam layout an obvious place to consume these signals rather than recomputing them.

### Crate DAG addition

```text
openplush-geom
   └─ openplush-mesh ─┬─ openplush-io
                      ├─ openplush-seams
                      ├─ openplush-fields ─ openplush-hints   ← new leaf
                      └─ ...
openplush-pipeline (depends on all, including hints)
```

`openplush-hints` is a leaf. Nothing in the engine depends on it, which means it can be built, tested, and reverted without touching the pattern pipeline.

## Architecture

### The hints stage

```rust
pub fn hints(mesh: &ManifoldMesh, scale: Scale, opts: &HintOptions) -> StageResult<HintReport>;
```

Fits 001's existing stage shape (`StageResult<T>` carrying a `WarningSet`). It takes `scale` because thresholds are physical (FR-007) — this is why FR-040 requires recomputation when the model's size changes, and why the signature cannot omit it.

**Detection is a pure read.** The signature takes `&ManifoldMesh` and there is no mutable path, so FR-010's "never alters geometry" is enforced by the borrow checker rather than by discipline.

### Separation of signal from threshold

The architectural core, and what makes FR-006's monotonicity guarantee cheap:

```text
mesh ─▶ signals (computed once, cached by mesh content hash)
            │  dihedral angle per edge
            │  integrated Gaussian curvature per vertex
            │  symmetry candidates + scores
            ▼
      threshold filter (sensitivity per category)  ─▶ hints
            │
      decisions (accept / dismiss, from the project)  ─▶ HintReport
```

Changing sensitivity re-runs only the filter, which is what makes SC-003's 1-second response achievable. Changing the model invalidates the signals. Changing scale re-runs the filter only, since the signals are scale-independent and only the physical thresholds move.

### Hints are not warnings

Worth stating plainly, because the two look similar and 001 just grew a warning channel. A hint is an **opportunity** surfaced before the maker acts; a warning is a **problem** found after. Hints never gate anything, never appear in the export summary, and are not part of FR-064's acknowledgement flow. `HintReport` is its own type, deliberately not a `WarningSet`.

The one place they must agree: the concave-crease detector and 001's FR-025 unsupported-concavity warning read the same signal. Shared implementation, not two thresholds that drift apart — otherwise the tool shows no hint and then complains about the same region, which reads as a bug and erodes trust in both.

### Accepting a hint

```text
Hint ─▶ Proposal (001's existing type) ─▶ preview ─▶ accept ─▶ SeamSet edit
```

Hints reuse 001's `Proposal` rather than defining a parallel mechanism. Preview-then-accept, undo, and the "nothing changes until the maker confirms" guarantee all come for free, and an accepted hint becomes an ordinary seam with no trace of its origin — which is what FR-030 requires and what keeps editing uniform afterwards.

## Phasing

| Release | Scope |
|---|---|
| **v0.2** | All of this feature: four detectors, overlay, panel, accept, dismiss, sensitivity. Ships alongside 001's v0.2 work |
| Later | A `FeatureDetector` trait with an optional learned implementation behind it (Constitution X), and automatic layout consuming these signals (001 FR-015) |

Nothing here is deferred within the feature. The spec's out-of-scope list is the boundary.

## Decision Ledger

| Date | ID | Decision | Status |
|---|---|---|---|
| 2026-10-13 | D-001 | New leaf crate `openplush-hints` rather than extending `openplush-fields` or `openplush-seams`: keeps the DAG acyclic, keeps the feature independently testable and revertible, and gives v0.2's auto-layout a place to consume the signals | Accepted |
| 2026-10-13 | D-002 | Signals computed once and cached; sensitivity is a pure threshold filter over them. Makes FR-006's monotonicity guarantee structural and SC-003's 1 s re-filter achievable | Accepted |
| 2026-10-13 | D-003 | **Fixed absolute thresholds, never percentile or adaptive ones.** Adaptive thresholds break monotonicity: raising sensitivity could reshuffle which features qualify and remove a hint the maker was looking at | Accepted — research R2 |
| 2026-10-13 | D-004 | Sharp-edge and concave-crease detectors share one dihedral computation with separate signed thresholds, and the concave one shares its implementation with 001's FR-025 warning | Accepted — research R1 |
| 2026-10-13 | D-005 | Hint identity is provenance key plus category, reusing 001's persistence keying. Re-match on exact key, then on more-than-half provenance overlap, then surface as orphaned (FR-041) | Accepted — research R3; overlap fraction tunable, spike H3 |
| 2026-10-13 | D-006 | Symmetry reports at most one plane with its match score, and reports **none** when candidates are close or no candidate scores well. Ambiguous beats arbitrary | Accepted — research R1, FR-005; the margin is D-010 |
| 2026-10-13 | D-007 | `HintReport` is its own type, not a `WarningSet`. Hints are opportunities before acting; warnings are problems after. Hints gate nothing and never reach the export summary | Accepted |
| 2026-10-13 | D-008 | Hard dependency on 001's flatten and pattern stages; no 3D-only preview fallback | Accepted — spec Q3 |
| 2026-10-13 | D-009 | `project.schema.json` v1 carries `dismissedHints` and `hintSettings` from the start, so this feature needs no schema migration and a 002-aware file still opens in a 001-only build | Accepted |
| 2026-10-19 | D-010 | D-006's ambiguity margin is the acceptance threshold itself (whose value T005 sets): symmetry reports none ("two planes fit") whenever a second distinct candidate plane (normals more than 10° apart, a spike parameter rather than a measurement) also scores within the threshold, with no separate margin constant. Spike T007 found that noise moves every fitting plane's score alike (about 1.4 σ / diagonal on three synthetic heads [likely]), so a gap between two fitting planes says which mirrors more exactly, not which the maker sews by; a tie and a near-tie must be treated the same. Only bodies with two fitting planes are affected; the report still lists every fitting candidate. Fallback if a fixed margin is wanted: 0.003 of the diagonal [likely] | Accepted 2026-10-19 — research/symmetry_margin |
| 2026-10-20 | D-011 | Default thresholds, provisional: sharp edge 30° mean fold, concave crease 25°, minimum chain length 20 mm, dart candidates at 15 mm integration radius and 60° summed angle defect. Dart curvature is the **signed** sum of angle defects over the disk, taken in absolute value afterwards, not the sum of absolute defects (R1's wording): a grid mesh alternates saddle and peak vertices, so the absolute sum measures tessellation and merged the quadruped into one cluster where the signed sum gives 20 at 30°. Spike T005 on five procedural fixtures (no real plush model): the coarse sphere's chains fold 20 to 25°, so sharp edges sit above 25°; concave creases separate cleanly from 25°; a 60° disk at 15 mm is the curvature of a cap of radius about 26 mm [likely]. Re-tune on T004's hand-made patterns when they exist. Open for T031: a cluster is located at its peak, which put the quadruped's head-dome hint on an ear. Open for the flood path: `faceted.glb` yields 10 hints at 20° or lower and none at the 30° default, so it exercises SC-010's cap at no threshold | Accepted 2026-10-20, provisional, by the agent under the maintainer's delegation of that day; the maintainer may reverse it — research/default_thresholds |

### Trade-offs accepted

- **Dart candidates locate, they do not design.** The detector says "a dart is needed around here"; direction and size come from 001's dart machinery on acceptance. Promising a specific dart would mean duplicating seam optimisation inside a hints feature.
- **Fixed thresholds need per-corpus tuning** (spike T005) and will be wrong for unusual models. Accepted in exchange for the monotonicity guarantee, which is more valuable to a first-time maker than optimal defaults.
- **SC-001 is unmeasurable until fixtures exist.** The feature's headline claim — that hints land where experienced makers put seams — cannot be evaluated without models that ship with known-good hand-made patterns. That is fixture-lane work (001 T035, spike T008), not code, and it is the single biggest hole in this plan.
- **The concavity signal is shared with 001's warning**, which couples two features. Accepted deliberately: the alternative is two thresholds that disagree, and a tool that contradicts itself about the same region.

### Open questions

- Spikes T005–T008 ([research R6](./research.md)). T008 is blocked on fixtures, and the re-match threshold spike, which has no task, is deferred until there is real editing-session data.
- Does the symmetry plane this feature detects fully satisfy 001's FR-010 mirroring mode, or does that mode need a user-declared override as well? Assume it does; revisit if the mirroring UI needs more.
- Hint count cap (SC-010, 50 per category): is the right behaviour to truncate the list, or to show a count and require the maker to lower sensitivity? Current assumption is the latter, since truncation hides hints silently.
