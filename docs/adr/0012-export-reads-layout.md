---
status: Accepted
date: 2026-10-18
deciders: Lavender (maintainer), accepted 2026-10-18; proposed by the implementing agent
---

# `openplush-export` may depend on `openplush-layout`

## Context and problem statement

`export_svg` and `export_pdf` take `&[Sheet]` (engine-api), and `Sheet` is what `layout` returns. The crate DAG lets `openplush-layout` and `openplush-export` depend on `openplush-pattern` only, with no edge between them, so export cannot name a type layout defines.

## Decision drivers

* The pipeline already runs layout and then export; the crate graph should not contradict it.
* `crates/AGENTS.md`: one responsibility per crate, depend downward only, no sideways edge without an ADR.
* Constitution X: no abstraction without a use.

## Considered options

1. **Export depends on layout.** One new edge, in the pipeline's own direction.
2. **Move `Sheet`, `Placement` and the page tiling into `openplush-pattern`.** No new edge, but pattern then owns placement, which is not its job (allowance, notches, labels, grain, dedupe).
3. **A new crate for the shared output types.** No edge between siblings, but a crate for three structs.

## Decision outcome

Chosen option: **1**. Export is the last stage and consumes layout's output, so the edge points the way data flows and adds no cycle. `scripts/check-boundary.sh` gains `openplush-export` on `openplush-layout` when the first export task imports it.

### Consequences

* 001 T090 waits for this ADR to be accepted.
* `openplush-pipeline` is unchanged: it already depends on both.
* If export ever needs more of layout than its output types, revisit option 3.
