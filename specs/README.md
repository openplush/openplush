# Specifications

OpenPlush uses spec-driven development ([Spec Kit](https://github.com/github/spec-kit)). Each feature gets a numbered directory, and work flows `specify → clarify → plan → tasks → implement`.

This index exists so you can tell at a glance what is specified, what is planned, and what is actually buildable.

## Status

| Feature | Spec | Planned | Tasks | Implementation |
|---|---|---|---|---|
| [001 pattern pipeline](./001-pattern-pipeline/) | ✅ Q1–Q3 closed | ✅ | ✅ 179 tasks | Phase 1 done except T017–T018 (need fixtures); Phase 2 under way (T022–T032, T034–T039, T042, T175, T177 and T178 merged; T033 deferred) |
| [002 seam-placement hints](./002-seam-placement-hints/) | ✅ Q1–Q3 closed | ✅ | ✅ 82 tasks | Blocked on 001 |
| [003 source seam import](./003-source-seam-import/) | ✅ clarifications closed | — | — | Blocked on 001 |

001 and 002 are fully specified and planned; 003 is a draft spec. 001's workspace, CI gates and governance (Phase 1) are done, except the determinism and golden workflows (T017–T018), which need the fixture corpus. Spikes S6 (licenses) and, on synthetic charts, S7 (boundary-pinned flattening) are done too. Phase 2 (foundational types) is under way: geometry core, robust predicates, the triangle soup, the half-edge mesh with its validator and property tests, the element IDs and proposal types, and the error and warning channels (`EngineError`, `WarningSet`, `Staged`/`StageResult`) are merged; the test machinery, fixtures and binding skeletons (T032, T034–T039) are merged too; the golden compare (T033) waits for the first output to compare. Next: the binding's protocol layer (T179), then generated TypeScript types (T041), then the engine client (T040). The open spikes `T001`–`T005`, `T007` on real fixtures, and `T008` can run alongside.

## Reading order

New to the project? Read in this order:

1. [`.specify/memory/constitution.md`](../.specify/memory/constitution.md) — the binding principles. Everything else defers to it.
2. [`001-pattern-pipeline/spec.md`](./001-pattern-pipeline/spec.md) — what the product is.
3. [`001-pattern-pipeline/plan.md`](./001-pattern-pipeline/plan.md) — the architecture and the Decision Ledger.
4. [`001-pattern-pipeline/tasks.md`](./001-pattern-pipeline/tasks.md) — where to start working.

## What each file is for

| File | Contains | Does not contain |
|---|---|---|
| `spec.md` | What and why, in plain language | Any technology choice |
| `research.md` | Library audits, algorithm comparisons, confidence-tagged findings | Decisions (those go in the plan) |
| `plan.md` | Architecture, repo layout, Decision Ledger, phasing | Task-level detail |
| `data-model.md` | Types, invariants, edge-case handling | Public API surface |
| `contracts/` | The public surfaces: engine API, worker protocol, project schema | Internal structure |
| `quickstart.md` | The acceptance walkthrough — when this works, the feature is done | Aspirational prose |
| `tasks.md` | Numbered, file-path-exact work items | Rationale (that lives upstream) |
| `checklists/requirements.md` | Spec quality gate, with its failures recorded honestly | |

## Feature dependency

002 depends hard on 001, and not incidentally: four of the seven capabilities it needs come from 001's pipeline, and its flat-pattern preview *is* 001's flatten plus pattern stages. See [002's plan](./002-seam-placement-hints/plan.md) for the exact task-level dependency — it starts partway through 001's Phase 3, not after all of 001.

## Deferred and out of scope

Named so nobody implements them by accident:

| Thing | Status | Where |
|---|---|---|
| Automatic seam layout | v0.2 — 002 is its groundwork | 001 FR-015, D-015 |
| Inserted pieces (ears in a seam) | v0.3; the data model supports it from day one | 001 FR-016 |
| Texture transfer for print-on-fabric | v0.2 | 001 FR-043 |
| Optimised nesting | v0.2 (v0.1 uses simple packing) | 001 FR-062 |
| Stuffed-shape preview, inverse inflation | **Out of scope** — excluded, not deferred | 001 Out of scope, D-012 |
| Direction-dependent stretch, anisotropic flattening | **Out of scope** | 001 FR-026, D-014 |
| Thickness compensation, shave-zone markings | **Out of scope** | 001 Out of scope |
| Learned/ML detection | **Out of scope** for both features | Constitution X |

## Known holes

Worth knowing before planning a release:

- **No fixtures exist.** 001 `T035`–`T037` and 002 `H001`–`H003`. Several success criteria cannot be evaluated at all until they do.
- **No model ships with a known-good hand-made pattern** (002 `H004`). 002's headline claim, SC-001, is unmeasurable without one.
- **001's fabric preset values are guesses** (research R7, spike `T008`). Plausible, not measured, and tagged `[verify]` throughout.
- **Five minor spec defaults remain open** in 001, each with a default already wired into `project.schema.json`. Listed in 001's spec under "Open Clarifications".
- **Spike `T007` (S7) is the highest-risk unknown**: the exact-seam-parity decision (D-013) rests the whole pattern stage on boundary-pinned flattening, and nobody has verified it stays injective on real meshes.

## Active feature pointer

`.specify/feature.json` is local and git-ignored; it tells Spec Kit commands which feature you are working on. Point it at the one you are actually working on:

```json
{"feature_directory":"specs/001-pattern-pipeline"}
```

The Spec Kit git extension creates a local `NNN-feature-slug` branch from `main`, which is the trunk. Rename it to the slice name before pushing (`git branch -m 003/01-spec`); see [`AGENTS.md`](../AGENTS.md#commits-and-prs).
