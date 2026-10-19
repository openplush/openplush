---
description: "Task list for 002 seam-placement hints"
---

# Tasks: Seam-Placement Hints

**Input**: Design documents in `/specs/002-seam-placement-hints/`
**Prerequisites**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [data-model.md](./data-model.md)
**Generated**: 2026-10-13

**Tests are mandatory**, as in 001: Constitution IV requires test-first with independent oracles, and `AGENTS.md` requires the failing test before the implementation.

## Format: `[ID] [P?] [Story] Description`

- **[P]** — can run in parallel (different files, no dependency)
- **[Story]** — which user story it serves
- `needs:ID[,ID]` — unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:D-0NN` — a proposed decision this one waits on; only the maintainer clears it
- `human:XX` — needs a person (outside review, measured values); no agent picks it up
- `needs:ID[,ID]` — unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:D-0NN` — a proposed decision this one waits on; only the maintainer clears it
- `human:XX` — needs a person (outside review, measured values); no agent picks it up
- Task IDs are `H0NN` to keep them distinct from 001's `T0NN` in commit messages and PR titles

## PR packaging

Same policy as 001: one intent per PR, each task a test commit then its implementation, interdependent intents stacked ([ADR 0013](../../docs/adr/0013-intent-sized-pull-requests.md)). See [`AGENTS.md`](../../AGENTS.md#commits-and-prs) and [docs/how-to/pull-requests.md](../../docs/how-to/pull-requests.md).

This feature packages more cleanly than 001, because each detector is independently testable.

### Batch these

| PR | Tasks |
|---|---|
| `002/NN-H009-hints-crate` | H009, H010, H011 |
| `002/NN-H015-signals` | H012, H013, H014, H015, H016, H017 — the foundation and its three contract tests belong together; the tests define what the split *is* |
| `002/NN-H026-crease-detector` | H018, H019, H020, H026 |
| `002/NN-H027-chaining` | H027 (plus H006's spike finding) |
| `002/NN-H029-dart-detector` | H021, H029 |
| `002/NN-H030-symmetry` | H022, H030 |
| `002/NN-H061-identity` | H057, H059, H060, H061, H062 |
| `002/NN-H071-lifecycle` | H071, H072, H073 |

`NN` is assigned when the PR is opened: the next unused position in feature 002.

One PR per detector. Each is a self-contained signal with its own oracle, which is exactly the review unit you want — a sign error in the concave threshold is obvious in a 150-line detector PR and invisible in a 600-line "all detectors" PR.

### Isolate these

| Task | Why |
|---|---|
| **H012** monotonicity property test | The single most important test in the feature. If it lands bundled, nobody checks whether it actually holds — and FR-006's guarantee becomes a comment |
| **H016** sensitivity mapping | Where the monotonicity guarantee is either kept or quietly broken by someone adding a helpful percentile threshold |
| **H031** explanation text | The teaching payload of the whole feature. It wants a careful prose review, not a skim inside a code PR. Review it like documentation, because it is |
| **H028** shared concavity signal with 001's FR-025 | Touches 001's warning path, so it crosses a feature boundary and needs both sides checked |

### Fixtures go alone

H001–H004 are each their own PR with their `LICENSES.md` entry. They are the `lane:fixtures` contribution path, they need no code review, and bundling them with code would force a geometry reviewer onto a model they cannot evaluate.

---

## Blocking dependency on 001

This feature **cannot start** until these 001 tasks are green (plan H-008):

| 001 tasks | Why |
|---|---|
| T069–T077 | Accepting a hint creates a seam |
| T078–T083 | FR-031's preview is a flat-pattern preview |
| T084–T087 | Same |
| T107 | Hints reuse the proposal mechanism rather than duplicating it |
| T144 | The dart-candidate curvature signal |
| T035, T037 | Nothing here is measurable without fixtures |

That is partway through 001's Phase 3, not after all of 001. H001–H004 (fixtures and spikes) can start earlier.

---

## Phase 0: Fixtures and spikes

**Purpose**: Hints are tuned against models. Without the fixtures, every threshold below is a guess, and SC-001 is unmeasurable.

- [ ] H001 [P] `fixtures/meshes/quadruped.glb` — symmetric animal with a muzzle fold, neck crease, and head dome. **The primary tuning fixture**; every default threshold is judged against it. Plus `LICENSES.md` entry
- [ ] H002 [P] `fixtures/meshes/asymmetric.glb` and `near-symmetric.glb` — one with no mirror plane at all, one with a single tilted ear. These prove refusal and the match score respectively; without them SC-005 cannot distinguish "works" from "always says yes"
- [ ] H003 [P] `fixtures/meshes/faceted.glb` — icosphere or low-poly model for the flood cap (SC-010)
- [ ] H004 **Fixture call: models with known-good hand-made patterns.** Fixture-lane work, no code. **SC-001 cannot be evaluated at all until this lands**, so the feature's headline claim stays unverified however green the test suite is. Flagged as a release risk, not a task to rush
- [ ] H005 needs:H001 **H1** — Tune default thresholds (dihedral, concave, curvature, minimum chain length, integration radius) against the corpus so a typical plush yields useful hints with no adjustment. Depends on H001. Box: 1 day
- [ ] H006 [P] **H2** — Confirm the monotonicity guarantee survives chaining and clustering. This is H012's property test, run as a spike first to find out whether the chain-once design actually holds. Box: 0.5 day
- [ ] H007 [P] needs:H002 **H4** — Symmetry on near-symmetric models: how close must two candidate planes score before reporting none beats reporting the best? Sets the ambiguity margin for H-006. Depends on H002. Box: 0.5 day
- [ ] H008 [P] needs:H004 **H5** — SC-001 measurement harness: compare a hint set against a hand-made pattern by seam-length coverage within 5 mm. **Blocked on H004.** Write the harness anyway so the measurement is ready the day fixtures arrive. Box: 1 day

**Checkpoint**: thresholds have defensible defaults. H004 and H008 may still be outstanding — they gate the release claim, not the build.

---

## Phase 1: Crate setup

- [ ] H009 `crates/openplush-hints/Cargo.toml` — new leaf crate (H-001), depending only on `openplush-geom`, `openplush-mesh`, `openplush-fields`. **Nothing in the engine may depend on it except `openplush-pipeline`**; the boundary check (001 T016) enforces the DAG
- [ ] H010 [P] `crates/openplush-hints/src/hint.rs` — `Hint`, `HintCategory`, `HintKind`, `HintId`, `Decision`, `HintReport` per data-model section 2, including the `absent` and `flooded` lists
- [ ] H011 [P] `crates/openplush-hints/src/explain.rs` — `Explanation` as a **struct with required `why` and `effect`**, never `Option<String>`. FR-022 is then enforced by the type: an unexplained hint cannot be constructed

**Checkpoint**: the crate compiles and the types exist. No detection yet.

---

## Phase 2: Foundational — signals and the threshold contract

**Purpose**: The signal/threshold split (H-002) is what makes monotonicity structural and SC-003's 1-second re-filter achievable. Every detector depends on it, so it lands first.

### Tests first

- [ ] H012 [P] `crates/openplush-hints/tests/monotonicity.rs` — `proptest` over random sensitivity pairs: the hint set at the lower sensitivity is a **subset** of the set at the higher one, for every category. **The single most important test in this feature.** Without it, FR-006's guarantee is a comment in a file
- [ ] H013 [P] `crates/openplush-hints/tests/determinism.rs` — identical input gives a byte-identical `HintReport`, and ordering is independent of insertion order (SC-008)
- [ ] H014 [P] `crates/openplush-hints/tests/scale.rs` — the same model at 80 mm and 600 mm produces different hint sets, because the length and radius thresholds are physical (FR-007). A test that passes when they are identical would be a test that permits FR-007 to go unimplemented

### Implementation

- [ ] H015 `crates/openplush-hints/src/signals.rs` — `HintSignals`: signed dihedral per edge, angle defect per vertex, integrated curvature over a geodesic disk of physical radius, symmetry candidates. Cached by mesh content hash
- [ ] H016 `crates/openplush-hints/src/sensitivity.rs` — sensitivity to threshold mapping, monotonically decreasing, **fixed absolute thresholds only** (H-003). A doc comment must state why percentile thresholds are forbidden, so the next person does not helpfully add them
- [ ] H017 `crates/openplush-hints/src/lib.rs` — `detect(&ManifoldMesh, Scale, &HintOptions) -> StageResult<HintReport>`. Takes `&ManifoldMesh` with no mutable path, so FR-010 is enforced by the borrow checker

**Checkpoint**: signals compute and filter monotonically. Detectors can now be added one at a time.

---

## Phase 3: US1 — See where seams belong (Priority: P1) 🎯 MVP

**Goal**: Instead of a blank surface, the maker sees the model's natural seam lines, listed by category, each explaining itself.

**Independent Test**: On the quadruped fixture, each category produces hints along the muzzle fold, neck crease, and head dome; a smooth sphere produces zero crease hints at default sensitivity.

### Tests for US1

- [ ] H018 [P] [US1] `crates/openplush-hints/tests/oracles.rs` — smooth sphere yields **zero** sharp-edge and concave-crease hints at default sensitivity (SC-004), and the categories appear in `absent` rather than simply being empty
- [ ] H019 [P] [US1] `crates/openplush-hints/tests/oracles.rs` — a cube yields exactly 12 sharp-edge chains, no more and no fewer. Catches both over-detection and chain fragmentation in one assertion
- [ ] H020 [P] [US1] `crates/openplush-hints/tests/concave.rs` — on the quadruped, concave hints appear at the neck and between the legs, and the convex muzzle ridge is **not** classified as concave. Sign errors are the obvious bug here and are invisible without this test
- [ ] H021 [P] [US1] `crates/openplush-hints/tests/dart.rs` — dart candidates cluster at the head dome and nose tip; a cylinder (developable) yields none
- [ ] H022 [P] [US1] `crates/openplush-hints/tests/symmetry.rs` — plane found on the quadruped with a low score; **none reported** on `asymmetric.glb`; score reported on `near-symmetric.glb` (SC-005)
- [ ] H023 [P] [US1] `crates/openplush-hints/tests/flood.rs` — the faceted fixture reports its over-cap categories in `flooded` and does **not** silently truncate the list (SC-010)
- [ ] H024 [P] [US1] `crates/openplush-hints/tests/explanations.rs` — every hint from every fixture has a non-empty `why` and `effect` (SC-007)
- [ ] H025 [P] [US1] `bindings/openplush-cli/tests/hints.rs` — the `hints` subcommand emits the JSON the quickstart asserts against

### Detectors

- [ ] H026 [US1] `crates/openplush-hints/src/crease.rs` — signed dihedral thresholding, with separate thresholds for convex and concave and the **concave threshold lower** (research R1: a shallow valley still vanishes when stuffed)
- [ ] H027 [US1] `crates/openplush-hints/src/chain.rs` — chain crease edges into polylines, preferring the smallest turning angle, ties broken by lowest provenance key. Stop at junctions and emit junctions as their own short hints rather than resolving a T-junction arbitrarily. **Chain once at the most permissive threshold, then filter by length** — re-chaining per sensitivity breaks monotonicity (data-model section 3)
- [ ] H028 [US1] `crates/openplush-hints/src/crease.rs` — **share the concave signal with 001's FR-025 unsupported-concavity warning** (H-004). One implementation, not two thresholds that drift. A tool that shows no hint and then warns about the same region reads as broken and erodes trust in both halves
- [ ] H029 [US1] `crates/openplush-hints/src/dart.rs` — integrated absolute Gaussian curvature over a physical-radius geodesic disk, clustered, one hint per cluster located at the peak vertex. Cluster once at the most permissive threshold, same reason as H027
- [ ] H030 [US1] `crates/openplush-hints/src/symmetry.rs` — area-weighted PCA candidates, reflection-ICP refinement, match score relative to bbox diagonal. **Report none when no candidate scores well, or when two score within the ambiguity margin** (H-006, margin from H007). Fixed iteration count and candidate ordering, no randomness
- [ ] H031 [US1] `crates/openplush-hints/src/explain.rs` — write the static `why` and `effect` text for all four categories, and the `detail` formatter. **This text is the teaching payload of the entire feature** and deserves more care than the code around it; it must say what a seam there *would do*, never that the maker *should* cut there
- [ ] H032 [US1] `crates/openplush-hints/src/suppress.rs` — suppress hints on detail parts (001 FR-005) and hints overlapping an existing seam within tolerance. A hint telling the maker to do what they already did is noise that makes the whole panel ignorable

### Pipeline, CLI, and shell

- [ ] H033 [US1] `crates/openplush-pipeline/src/hints.rs` — expose the `hints` stage; cache signals by mesh hash, re-filter on sensitivity change only
- [ ] H034 [US1] needs:H033 `specs/001-pattern-pipeline/contracts/engine-api.md` — once H033 has merged, give the contract's `hints` entry an "As built" pointer to rustdoc and `docs/reference/engine-api.md`; do not edit the signature. H033's PR documents the built signature itself; this task is the later, spec-only edit, since one PR never mixes spec and implementation ([ADR 0003](../../docs/adr/0003-as-built-docs.md)). *2026-10-14: replaces "reconcile the signature with what was built"*
- [ ] H035 [US1] `bindings/openplush-cli/src/commands/hints.rs` — `hints` subcommand with `--only`, `--sensitivity`, `--json` (FR-050, Constitution II)
- [ ] H036 [US1] `packages/engine-client/src/ops/hints.ts` — the `hints` op in the typed protocol, with the mock-worker implementation so shell tests need no WASM
- [ ] H037 [US1] `apps/web/src/hints/overlay.tsx` — 3D overlay, one visually distinct treatment per category, **colour-blind-safe palettes** (FR-020, Constitution accessibility constraint)
- [ ] H038 [US1] `apps/web/src/hints/panel.tsx` — inspection panel grouped by category with counts; selecting an entry moves the camera and highlights the overlay element (FR-021)
- [ ] H039 [US1] `apps/web/src/hints/panel.tsx` — render the explanation for the selected hint (FR-022); show the `absent` message for empty categories (FR-024) and the `flooded` message with a pointer to the sensitivity control (FR-025)
- [ ] H040 [US1] `apps/web/src/hints/keyboard.ts` — full keyboard operation: navigate between hints, accept, dismiss (FR-023)
- [ ] H041 [P] [US1] `apps/web/tests/hints-panel.spec.ts` — Playwright: categories toggle independently, selection moves the camera, empty categories say so

**Checkpoint**: US1 complete. The maker can see and understand where seams belong. Nothing can be acted on yet, but the teaching half of the feature works.

---

## Phase 4: US2 — Turn a hint into a seam (Priority: P1)

**Goal**: Accept a hint in one action, see the flat-pattern effect, keep or discard. Nothing applies itself.

**Independent Test**: Every hint on the capsule and quadruped fixtures accepts into a valid seam or dart, editable afterwards like a hand-drawn one, with a preview first.

### Tests for US2

- [ ] H042 [P] [US2] `crates/openplush-hints/tests/accept.rs` — accepting any hint on any fixture yields a valid seam or dart: never a malformed or self-crossing path (SC-006)
- [ ] H043 [P] [US2] `crates/openplush-hints/tests/accept.rs` — an accepted hint becomes an **ordinary seam**, indistinguishable from a hand-drawn one in the resulting `SeamSet`. FR-030's real requirement, and the thing that keeps later editing uniform
- [ ] H044 [P] [US2] `crates/openplush-hints/tests/accept.rs` — declining a preview leaves the model byte-identical (FR-031); taking no action changes nothing (FR-032)
- [ ] H045 [P] [US2] `crates/openplush-hints/tests/unsewable.rs` — a hint whose acceptance would produce a sub-minimum piece warns first and still permits acceptance; one that would stop a chart being a disk is refused with proposals attached (FR-036, data-model section 6)
- [ ] H046 [P] [US2] `apps/web/tests/hints-accept.spec.ts` — Playwright: accept shows a flat-pattern preview, undo restores the hint to its un-acted state

### Implementation

- [ ] H047 [US2] `crates/openplush-hints/src/accept.rs` — `to_proposal()` converting a hint into 001's existing `Proposal`. Reuse, not a parallel mechanism: preview-then-accept, undo, and "nothing changes until confirmed" all come from 001 (Constitution VII)
- [ ] H048 [US2] `crates/openplush-hints/src/accept.rs` — line hints become a seam along the **whole chain**, not a starting point (FR-030). Region hints become a dart request, with direction and size chosen by 001's dart machinery from the cluster peak — this detector locates a dart, it does not design one
- [ ] H049 [US2] `crates/openplush-hints/src/accept.rs` — pre-acceptance sewability check against 001's validation: minimum piece size, pile multiple (001 FR-055), dart tightness, chart-is-a-disk. First three warn, the last refuses (FR-036)
- [ ] H050 [US2] `apps/web/src/hints/accept.tsx` — accept action, flat-pattern preview, keep or discard. **Symmetry has no accept button** — it gets a "use for mirroring" toggle instead, because it is a hint the maker uses rather than applies (data-model section 6)
- [ ] H051 [US2] `apps/web/src/hints/undo.ts` — accept, decline, and dismiss are all undoable through 001's existing undo stack (FR-035)

**Checkpoint**: US1 and US2 together are the feature's core loop. A maker can see a suggestion, understand it, try it, and keep it.

---

## Phase 5: US3 — Mirror work across a symmetric model (Priority: P2)

**Goal**: The detected mirror plane feeds 001's mirrored seam drawing, so one side of a symmetric model is enough.

**Independent Test**: On the quadruped the plane matches the model's actual plane; on the asymmetric fixture the category reports none rather than guessing.

### Tests for US3

- [ ] H052 [P] [US3] `crates/openplush-seams/tests/mirror_plane.rs` — a seam drawn on one side with mirroring on produces the correctly mirrored seam on the other
- [ ] H053 [P] [US3] `crates/openplush-hints/tests/symmetry.rs` — the detected plane is reported with its match score, so FR-005's "how closely the model matches it" is a number rather than a claim

### Implementation

- [ ] H054 [US3] `crates/openplush-seams/src/mirror.rs` — accept a symmetry plane from the hints stage as the plane for 001's FR-010 mirroring mode. This feature **supplies** that plane rather than inventing a second mechanism
- [ ] H055 [US3] `apps/web/src/hints/symmetry.tsx` — show the plane on the model with its match score; "use for mirroring" toggle
- [ ] H056 [US3] `apps/web/src/hints/symmetry.tsx` — when no plane is found, say so plainly, and when candidates were ambiguous say *that* rather than "no symmetry" — they are different facts and a maker can act on the difference

**Checkpoint**: symmetric models take roughly half the drawing work.

---

## Phase 6: US4 — Dismiss a hint and have it stay dismissed (Priority: P2)

**Goal**: A rejected hint leaves the list for good, but dismissals stay reviewable and reversible.

**Independent Test**: Dismiss a hint, save, reopen: still dismissed, and listed somewhere it can be restored.

### Tests for US4

- [ ] H057 [P] [US4] `crates/openplush-hints/tests/identity.rs` — `HintId` is stable across recomputation when the mesh has not changed, and across repair-setting changes, because it keys on provenance rather than `EdgeId`
- [ ] H058 [P] [US4] `crates/openplush-project/tests/hint_decisions.rs` — dismissals survive save and reopen with no schema migration (H-009)
- [ ] H059 [P] [US4] `crates/openplush-hints/tests/rematch.rs` — the three re-match cases from data-model section 4: exact match carries silently, more-than-half overlap carries and is flagged approximate, no overlap becomes an `OrphanedDismissal`
- [ ] H060 [P] [US4] `crates/openplush-hints/tests/rematch.rs` — a re-imported model at different resolution orphans its dismissals rather than silently carrying them. The honest outcome: a re-import is a new model, and guessing otherwise would be the wrong guess

### Implementation

- [ ] H061 [US4] `crates/openplush-hints/src/identity.rs` — `HintId` as a hash of category plus the sorted provenance key set (H-005)
- [ ] H062 [US4] `crates/openplush-hints/src/identity.rs` — the re-match ladder: exact, then overlap above a configurable fraction (default 0.5, tunable per spike H3), then orphan
- [ ] H063 [US4] `crates/openplush-project/src/hints.rs` — persist dismissals and per-category settings into the existing `dismissedHints` and `hintSettings` fields. **No migration**, by design (H-009)
- [ ] H064 [US4] `apps/web/src/hints/dismiss.tsx` — dismiss action, a dismissed-hints review list, restore action (FR-033)
- [ ] H065 [US4] `apps/web/src/hints/orphans.tsx` — surface orphaned dismissals for re-confirmation or release. **Never silently dropped and never silently kept** (FR-041) — the whole point of the clarification that produced this requirement

**Checkpoint**: the panel stays useful over a long editing session instead of filling with rejected suggestions.

---

## Phase 7: US5 — Tune how much is flagged (Priority: P3)

**Goal**: Sensitivity per category, so a detailed model can show less and a smooth one more.

**Independent Test**: On the faceted fixture, lowering sensitivity reduces the count; raising it only ever adds.

### Tests for US5

- [ ] H066 [P] [US5] Covered by H012's property test — the monotonicity guarantee *is* this story's correctness condition
- [ ] H067 [P] [US5] `apps/web/tests/hints-sensitivity.spec.ts` — Playwright: raising the slider never removes a visible hint. The user-facing form of H012, and the one a maker would actually notice

### Implementation

- [ ] H068 [US5] `apps/web/src/hints/sensitivity.tsx` — per-category slider with the tuned default from H005, showing the live count
- [ ] H069 [US5] `crates/openplush-project/src/hints.rs` — persist sensitivity settings (FR-034)
- [ ] H070 [US5] `crates/openplush-pipeline/src/hints.rs` — re-filter without recomputing signals, meeting SC-003's 1-second budget. This is the payoff of H-002's split

**Checkpoint**: all five stories functional.

---

## Phase 8: Lifecycle and recomputation

**Purpose**: FR-040 to FR-042. Lands after the detectors exist, because recomputation has nothing to recompute before then.

- [ ] H071 [P] `crates/openplush-hints/tests/lifecycle.rs` — hints recompute on model or size change; **already-accepted seams are untouched** (FR-040), because they are seams now and not hints
- [ ] H072 [P] `crates/openplush-hints/tests/lifecycle.rs` — the change summary lists hints added and hints no longer present (FR-042)
- [ ] H073 `crates/openplush-hints/src/lifecycle.rs` — invalidate signals on mesh change, re-filter on scale change, re-match decisions, build the change summary
- [ ] H074 `apps/web/src/hints/changes.tsx` — show what changed after a recomputation, so a maker who resized understands why the overlay moved rather than assuming a bug

---

## Phase 9: Polish

- [ ] H075 [P] Performance pass against SC-003: 5 s initial detection at 50k triangles, 1 s re-filter
- [ ] H076 [P] `apps/web/tests/hints-no-network.spec.ts` — zero network requests during detection and display (SC-009, FR-051)
- [ ] H077 [P] Accessibility pass: verify category palettes against a colour-blindness simulator; confirm the panel is fully keyboard-operable (FR-020, FR-023)
- [ ] H078 [P] `docs/explanation/why-seams-go-there.md` — extend 001's teaching document with what each hint category means and why. The same material as the in-app explanations, at length
- [ ] H079 `docs/reference/hint-categories.md` — reference for each category: signal, default threshold, what acceptance does. Each detector PR writes its own category's section (H026 sharp edges and concave creases, H029 darts, H030 symmetry), and H005's tuned defaults update the thresholds; H079 follows them as the closing pass: an index and a check against the code ([ADR 0003](../../docs/adr/0003-as-built-docs.md)). *2026-10-14: was written whole at the end*
- [ ] H080 Run the full [quickstart.md](./quickstart.md) walkthrough, including the monotonicity shell assertions
- [ ] H081 human:recruiting SC-002 usability test: five or more first-time makers, measure time to first seam with hints on vs. off, target 30% median improvement. Needs recruiting and a protocol; gates the release claim, not the build
- [ ] H082 SC-001 evaluation, **if H004 has landed**: 70% of hand-placed seam length within 5 mm of a hint. If the fixtures do not exist, **say so at release rather than quietly dropping the criterion** — an unverified headline claim is worth naming

---

## Dependencies and Execution Order

### Phase dependencies

- **Phase 0**: H001–H003 start any time; H005 needs H001; H007 needs H002; H008 blocked on H004
- **Phase 1**: needs 001's T069–T087 and T144 green
- **Phase 2**: needs Phase 1. **Blocks every story** — the signal/threshold split is load-bearing for all of them
- **Phase 3 (US1)**: needs Phase 2 and the thresholds from H005
- **Phase 4 (US2)**: needs US1's detectors plus 001's T107
- **Phase 5 (US3)**: needs H030 (symmetry detector)
- **Phase 6 (US4)**: needs H061/H062 identity; can run parallel to Phases 4 and 5
- **Phase 7 (US5)**: needs Phase 2 only; the UI needs Phase 3
- **Phase 8**: needs the detectors and the identity work
- **Phase 9**: needs everything

### Story independence

Better than 001's. US1 (see hints) genuinely ships alone and delivers the teaching value on its own. US2 (accept) needs US1's detectors but nothing else. US3, US4, and US5 are each independent of each other. Only the Phase 2 foundation is shared.

The one hard external dependency is 001's flatten and pattern stages, for US2's preview — which is what the spec's Q3 resolved and why there is no 3D-only fallback.

### Parallel opportunities

- All of Phase 0 except H005 and H008
- H010 and H011 together
- All of Phase 2's tests (H012–H014) together
- All of Phase 3's tests (H018–H025) together, then the four detectors H026/H029/H030 in parallel (H027 follows H026)
- Phase 6 alongside Phases 4 and 5
- All of Phase 9 except H079 (after the detectors) and H080

### Within each story

1. Tests written and observed failing
2. Signals before detectors, detectors before UI
3. Engine before shell, always
4. Story complete and independently verified before the next priority

---

## Traceability

| Spec requirement | Tasks |
|---|---|
| FR-001 four categories, toggleable | H010, H026–H030 |
| FR-002 sharp-edge chains with strength | H026, H027 |
| FR-003 concave creases, sign-distinguished | H026, H028 |
| FR-004 dart candidates | H029 |
| FR-005 symmetry, at most one, absence reported | H030, H056 |
| FR-006 sensitivity, monotone | H012, H016, H068 |
| FR-007 physical thresholds | H014, H016 |
| FR-008 stable identity | H057, H061 |
| FR-009 determinism | H013, H030 |
| FR-010 never alters geometry | H017 (enforced by the borrow checker) |
| FR-020 overlay, colour-blind-safe | H037, H077 |
| FR-021 inspection panel | H038 |
| FR-022 plain-language explanation | H011, H024, H031 |
| FR-023 keyboard-operable | H040, H077 |
| FR-024 empty category says so | H018, H039 |
| FR-025 flood message | H023, H039 |
| FR-030 one-action conversion, whole feature | H043, H048 |
| FR-031 preview before keeping | H044, H050 |
| FR-032 never auto-applied | H044 |
| FR-033 dismiss, reviewable, reversible | H064 |
| FR-034 decisions and settings persisted | H058, H063, H069 |
| FR-035 undoable | H051 |
| FR-036 unsewable warning before acceptance | H045, H049 |
| FR-040 recompute on model or size change | H071, H073 |
| FR-041 orphaned dismissals surfaced | H059, H062, H065 |
| FR-042 change summary | H072, H074 |
| FR-050 headless | H035 |
| FR-051 local-only | H076 |
| SC-001 hint recall | H008, H082 — **blocked on H004** |
| SC-002 faster first seam | H081 |
| SC-003 performance | H070, H075 |
| SC-004 sphere yields zero | H018 |
| SC-005 symmetry correctness | H022 |
| SC-006 acceptance always valid | H042 |
| SC-007 every hint explained | H024 |
| SC-008 determinism | H013 |
| SC-009 no network | H076 |
| SC-010 flood cap | H023 |
