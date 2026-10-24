# Specifications

OpenPlush uses spec-driven development ([Spec Kit](https://github.com/github/spec-kit)). Each feature gets a numbered directory, and work flows `specify → clarify → plan → tasks → implement`.

This index lists the features and how to read their files. How far each feature's spec, plan and implementation have got is on the [status page](../docs/start/status.md).

## Features

| Feature | What it specifies |
|---|---|
| [001 pattern pipeline](./001-pattern-pipeline/) | A 3D model in, a printable plush sewing pattern out |
| [002 seam-placement hints](./002-seam-placement-hints/) | Where on the model seams are likely to help, shown before the maker draws them |
| [003 source seam import](./003-source-seam-import/) | Seams and fabric regions read from the model's own UV map and materials |
| [004 docs site](./004-docs-site/) | A documentation site for contributors and makers |

## Repository process

Work on how the repository runs: agents, merging, checks. No maker sees a change from it, so the status page counts it apart from the features above.

| Feature | What it specifies |
|---|---|
| [005 factory hardening](./005-factory-hardening/) | Unattended factory runs: one structured grant, a handoff from every run, merge-gate setup |

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

- **Fixtures and models**: what the corpus still lacks, and what that leaves unmeasurable, is on the [status page](../docs/start/status.md#known-gaps).
- **001's fabric preset values** in research R7: stretch is the Fabric Stretch Guide's estimate, `[likely]` (short faux fur `[speculative]`); the other fields are `[verify]`. What that leaves is on the [status page](../docs/start/status.md#known-gaps).
- **001's minor spec defaults** are open questions, each with a default already wired into `project.schema.json`. Listed in 001's spec under "Open Clarifications".

## Active feature pointer

`.specify/feature.json` is local and git-ignored; it tells Spec Kit commands which feature you are working on. Point it at the one you are actually working on:

```json
{"feature_directory":"specs/001-pattern-pipeline"}
```

The Spec Kit git extension creates a local `NNN-feature-slug` branch from `main`, which is the trunk. Rename it to the slice name before pushing (`git branch -m 003/01-spec`); see [`AGENTS.md`](../AGENTS.md#commits-and-prs).
