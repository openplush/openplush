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
| `openplush-testkit` | Dev-dependency only. `fixtures`: `load(Kind, name)` returns a fixture's bytes, `names(Kind)` lists every fixture of a kind sorted, and `FixtureError` covers a bad name, a missing file, an entry that is not a file and a read error. Kinds: `Mesh` (`fixtures/meshes/*.obj`), `Defect` (`fixtures/defects/*.obj`), `Seams` (`fixtures/seams/*.json`). `oracles`: checks that recompute a property of a flattened `Chart` (plain `xyz`, `uv` and face arrays) from raw coordinates, independent of the code under test: `injective` (every flat face strictly counter-clockwise, exact predicate), `developable` (both signed singular values within a tolerance of 1, so a mirrored face fails) and `area_conserved` (signed flat area within a relative tolerance of 3D area); and, on plain values, `outline_simple` (a closed outline with no crossing, touching, zero-length or doubled-back edge, wound counter-clockwise, all by exact predicates; an outline the predicates cannot decide fails) and `seam_parity` (max(1.0 mm, 0.5% of the longer side), restated rather than taken from `SeamParity`). Each reports an `OracleFailure`, which names the face, edge or lengths involved where one applies; a chart with no faces fails as `Empty`, and a NaN, infinite or negative tolerance as `BadTolerance` | `openplush-geom`, `openplush-mesh`, `thiserror` |
| `openplush-pipeline` | The public API: `EngineError`, the warning types, `Staged`/`StageResult`, and re-exports. See [engine-api.md](engine-api.md) | `openplush-io`, `-mesh`, `-pattern`, `-seams`, `thiserror` |

## Scaffolds (no code yet)

`openplush-fields`, `openplush-flatten`, `openplush-decal`, `openplush-layout`, `openplush-export`, and `openplush-project` exist so the workspace and its boundary checks are in place. Each crate's purpose is in `crates/AGENTS.md`.

## Bindings

Shells in `bindings/` may depend on `openplush-pipeline` and nothing else from `crates/` (`scripts/check-boundary.sh`).

| Binding | What it is today | Depends on |
|---|---|---|
| `openplush-cli` | The argument-parsing skeleton; no stage is wired yet. See [cli.md](cli.md) | `clap` ([ADR 0004](../adr/0004-cli-argument-parsing.md)) |
| `openplush-wasm` | The `wasm-bindgen` facade skeleton: it builds as a `cdylib` for `wasm32-unknown-unknown` and exports nothing yet | `openplush-pipeline`, `wasm-bindgen` |
