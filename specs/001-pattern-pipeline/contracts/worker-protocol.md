# Contract: shell ↔ worker protocol

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-13 | **Status**: Draft contract

The engine runs in a Web Worker so the UI thread never blocks on geometry (Constitution "Technical Constraints"). This file defines the messages between `apps/web` and the worker hosting `bindings/openplush-wasm`, mediated by `packages/engine-client`.

TypeScript types are **generated from the Rust definitions**, not hand-written. Two hand-maintained copies of a protocol drift, and the drift shows up as a wrong pattern.

## 1. Rules

1. **Mesh state never crosses.** The worker owns the mesh and all intermediates. The UI holds opaque handles (`MeshHandle`, `PipelineHandle`). A 50k-triangle mesh is not a message payload.
2. **Control messages are small JSON.** Request/response, correlated by `id`.
3. **Geometry for rendering crosses as transferables.** `Float32Array` / `Uint32Array`, zero-copy. Positions are `f32` at this boundary only — `f64` everywhere inside the engine (Constitution "Numerics").
4. **Every request is cancellable.** The UI may cancel by `id`; the worker honours it at the next `step()` boundary.
5. **Errors cross as typed codes with locations**, never as strings the UI has to parse.

## 2. Envelope

```ts
type Request  = { id: number; op: string; payload: unknown };
type Response =
  | { id: number; ok: true;  result: unknown; warnings: Warning[]; transfer?: ArrayBuffer[] }
  | { id: number; ok: false; error: EngineErrorDto }
  | { id: number; progress: Progress };          // zero or more, before the terminal message
```

Exactly one terminal message (`ok: true` or `ok: false`) per `id`. `progress` messages are advisory and may be dropped under load.

`warnings` rides on every success. The UI is responsible for showing them and for FR-064's one-time acknowledgement; the worker only reports.

## 3. Operations

| `op` | Payload | Result | Notes |
|---|---|---|---|
| `import` | bytes (transferred), `ImportOptions` | `MeshHandle`, `ValidationReport` | Import and validate in one round trip — the UI always wants both |
| `repair` | `MeshHandle`, `RepairPlan` | `MeshHandle` (new), `RepairReport` | Never mutates in place; the pre-repair handle stays valid for before/after display (Constitution VII) |
| `renderMesh` | `MeshHandle`, LOD | positions, indices, normals (transferred) | For the 3D view |
| `setSeams` | `PipelineHandle`, `SeamSet` | affected chart IDs | Invalidates only affected charts (FR-014) |
| `pathBetween` | `MeshHandle`, two surface points | polyline of `EdgeId`s | Seam drawing: shortest surface path (FR-010) |
| `flattenStart` | `PipelineHandle`, `FlattenOptions` | — | Followed by `progress` messages |
| `flattenStep` | `PipelineHandle` | `Progress` | UI-driven pumping; keeps the worker responsive to cancels |
| `metrics` | `PipelineHandle` | per-chart distortion + heatmap buffer (transferred) | FR-021 |
| `proposals` | `PipelineHandle`, kind | `Proposal[]` | Darts, extra seams, splits — all preview-then-accept |
| `applyProposal` | `PipelineHandle`, `ProposalId` | affected chart IDs | Only after explicit user acceptance |
| `pieces` | `PipelineHandle`, `PatternOptions` | `PatternPiece[]` (outlines transferred) | FR-030 … FR-035 |
| `layout` | `PipelineHandle`, `LayoutOptions` | `Sheet[]` | |
| `export` | `PipelineHandle`, `ExportOptions` | bytes (transferred) | SVG or PDF; includes the warning summary page (FR-064) |
| `saveProject` | `PipelineHandle` | bytes (transferred) | |
| `loadProject` | bytes (transferred) | `PipelineHandle`, migration notes | FR-070 |
| `hints` | `MeshHandle`, `HintOptions` | `Hint[]` + overlay buffers | **Feature 002** |
| `cancel` | `{ target: number }` | — | Cancels the request with that `id` |

## 4. Handle lifetime

```ts
type MeshHandle     = { readonly __brand: 'mesh';     readonly n: number };
type PipelineHandle = { readonly __brand: 'pipeline'; readonly n: number };
```

- Handles are opaque integers, branded in TS so they cannot be swapped or arithmetic'd.
- The worker owns the objects. `release(handle)` frees; using a released handle returns `EngineError::StaleHandle` rather than undefined behaviour.
- Handles do **not** survive a worker restart. On restart the client replays from the project document, which is the only durable state.

## 5. Progress and cancellation

```ts
type Progress =
  | { kind: 'working'; chart: number; iteration: number; maxDistortion: number }
  | { kind: 'done' };
```

Cancellation is cooperative: the worker checks between `step()` calls. A cancelled operation produces `EngineError::Cancelled` and leaves no partial result reachable. There is deliberately no way to retrieve a half-finished flattening — a partially flattened piece is exactly the kind of plausible-looking wrong output the constitution forbids.

## 6. Errors

```ts
type EngineErrorDto = {
  code: string;                     // stable, matches EngineError variant names
  message: string;                  // plain language, shown to the maker
  locations?: { faces?: number[]; edges?: number[]; vertices?: number[]; pieces?: number[] };
  proposals?: Proposal[];           // e.g. ChartNotDisk carries its own fix (FR-013)
};
```

The UI highlights `locations` in the 3D view. A code the UI does not recognise must still display `message` and the locations — forward compatibility, so an engine ahead of the shell degrades to "something is wrong, here" instead of silence.

## 7. Testing

- `packages/engine-client` is tested against a **mock worker** implementing this contract, so shell tests need no WASM build.
- The real worker is exercised by Playwright smoke tests on three fixtures (plan "CI").
- Generated TS types are checked into the repo and regenerated in CI; a diff fails the build, which is how drift is caught rather than discovered.
