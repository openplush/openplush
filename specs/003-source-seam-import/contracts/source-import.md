# Contract: source seam import

**Feature**: `003-source-seam-import` | **Date**: 2026-10-16 | **Status**: Draft contract

Additions to the single engine surface (`specs/001-pattern-pipeline/contracts/engine-api.md`). This file is their home until the code lands. The implementation slice that adds the stage also bumps 001's engine-api version and records the change in 001's Decision Ledger, as that contract requires. After that, rustdoc and `docs/reference/` are the source of truth (ADR 0003), and 001's contract gains a one-line pointer here, as it has for 002's `hints` stage.

## 1. Stages

```rust
/// Reads UV cuts and materials into a proposal; never touches the SeamSet.
pub fn source_import(
    mesh: &ManifoldMesh,
    existing: &SeamSet,
    fabrics: &[Fabric],
) -> StageResult<SourceImport>;

/// The difference a Replace or Merge would make, before the maker commits.
pub fn reimport_diff(
    old: &ProjectDocument,
    new_mesh: &ManifoldMesh,
    import: &SourceImport,
    on_existing: OnExisting,
) -> StageResult<ReimportDiff>;
```

- `source_import` is a pure read (Constitution VII). It takes `&ManifoldMesh` and `&SeamSet`, and its only output is the proposal. Accepting goes through 001's existing proposal application (T107) plus `set_fabrics`, as one undo step (T128).
- `existing` is there so the proposal never contains an edge that is already a seam (`Proposal::new`'s invariant), and so `closing` is computed against what the project will actually hold.
- Errors: `DeadEndCut` (data-model §4). Everything else this stage can hit is already a `ChartNotDisk` *proposal* rather than an error: the stage asks `propose_cuts` for remedies and does not fail on them.
- Warnings: none. Missing UVs, missing materials, extra UV sets and created profiles are facts for the report, not problems (spec FR-005).

## 2. Errors

| Variant | Carries | Remedy text (plain language, engine-supplied) |
|---|---|---|
| `DeadEndCut` | `tips: Vec<VertexId>`, `chains: Vec<Vec<EdgeId>>` | "This cut stops partway across the surface, so it separates nothing. In your modelling tool, extend it until it meets another seam or an edge of the model, or remove it, then import again." |

The string's final wording belongs in `docs/reference/` when the variant lands.

## 3. CLI (Constitution II)

```sh
openplush-cli import-seams model.glb --json import.json                         # the proposal and report
openplush-cli import-seams model.glb --project old.plush --on-existing merge --json diff.json
openplush-cli pattern model.glb --seams-from-model --accept-closing --size-mm 300 -o out/
```

- `--seams-from-model` without `--accept-closing` refuses with the `ChartNotDisk` error (proposals included) when the imported seams don't close every chart. Headless mode never accepts a proposal without a flag naming it.
- Exit code for `DeadEndCut`: the CLI's existing error code for `EngineError`. The JSON body lists every tip.

## 4. Worker protocol

One new request, `sourceImport`, and one new response carrying `SourceImport` as a protocol DTO, under ADR 0010's rules: finite floats only, and no 64-bit integers, so each proposal's `ProposalId` crosses as its 16-hex-digit string, as 001's proposals already do. `reimportDiff` likewise. Both are additive; the protocol document carries no version number to bump.
