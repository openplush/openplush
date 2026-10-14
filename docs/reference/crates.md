# Crates

What each engine crate contains **today**, and what it actually depends on. Which crate *should* own a new piece of code is a rule, kept in [`crates/AGENTS.md`](../../crates/AGENTS.md#where-code-goes); the allowed dependency graph is in the [plan](../../specs/001-pattern-pipeline/plan.md#crate-dependency-dag-enforced) and enforced by `scripts/check-boundary.sh`. Signatures and item docs: `cargo doc --workspace --no-deps --open`.

## Built

| Crate | Public items | Depends on |
|---|---|---|
| `openplush-geom` | `DVec2`, `DVec3`; exact predicates `orient2d`, `orient3d`, `incircle` returning `Sign`; `epsilon` (`ModelTolerances`, `SeamParity`, and the constants behind them); `units` (`Mm`, `Scale`); `GeomError` | `robust`, `thiserror` |
| `openplush-mesh` | `TriangleSoup` and `SourceCorner`; the half-edge `ManifoldMesh` (read-only accessors, `outgoing` returning `Outgoing`, `boundary_loops`, `debug_validate`; no public constructor); IDs `VertexId`, `EdgeId`, `HalfEdgeId`, `FaceId`, `ComponentId`, `MeshElement`; `MeshError` with `CornerArray` and `Counted` | `openplush-geom`, `thiserror` |
| `openplush-seams` | `Proposal` (built only by `Proposal::new`), `ProposalId`, `ProposalKind`, `SeamReason`; `SeamKind`, `ChartFailure`; IDs `ChartId`, `SeamChainId`; `SeamsError`. No `SeamSet` or charting yet | `openplush-mesh`, `thiserror` |
| `openplush-io` | `Format` (OBJ, glTF, GLB, STL, PLY). No parsers yet | none |
| `openplush-pattern` | `PieceId`. No pattern code yet | none |
| `openplush-pipeline` | The public API: `EngineError`, the warning types, `Staged`/`StageResult`, and re-exports. See [engine-api.md](engine-api.md) | `openplush-io`, `-mesh`, `-pattern`, `-seams`, `thiserror` |

## Scaffolds (no code yet)

`openplush-fields`, `openplush-flatten`, `openplush-decal`, `openplush-layout`, `openplush-export`, `openplush-project`, and `openplush-testkit` (dev-dependency only) exist so the workspace and its boundary checks are in place. Each crate's purpose is in `crates/AGENTS.md`; testkit's is in its own crate doc.

## Bindings

Shells in `bindings/` may depend on `openplush-pipeline` and nothing else from `crates/` (`scripts/check-boundary.sh`).

| Binding | What it is today | Depends on |
|---|---|---|
| `openplush-cli` | The argument-parsing skeleton; no stage is wired yet. See [cli.md](cli.md) | `clap` ([ADR 0004](../adr/0004-cli-argument-parsing.md)) |
