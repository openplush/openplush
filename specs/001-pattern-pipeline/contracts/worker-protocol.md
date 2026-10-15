# Contract: shell ↔ worker protocol

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-13 | **Status**: Draft contract

*Changes*: 2026-10-14 (D-022): binding-level error codes alongside `EngineError` names, `ProposalId` as a hex string, and chart, chain and component locations. 2026-10-15 ([ADR 0009](../../../docs/adr/0009-wasm-panic-is-a-worker-crash.md)): a panic is a worker crash: the `EngineCrashed` code, worker generations, and what happens to in-flight requests and handles (§4, §6, §7).

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
  | { id: number; progress: Progress }           // zero or more, before the terminal message
  | { crashed: true };                           // the worker host's own message; see §4, Worker crash
```

Exactly one terminal message (`ok: true` or `ok: false`) per `id`. `{ crashed: true }` is the one message with no `id` and no Rust counterpart: the worker host sends it after a trap, so `packages/engine-client` types it by hand next to the generated types. `progress` messages are advisory and may be dropped under load.

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
| `applyProposal` | `PipelineHandle`, `ProposalId` (16-char hex string) | affected chart IDs | Only after explicit user acceptance. An ID no longer offered is `ProposalNotOffered` |
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
- The worker owns the objects. `release(handle)` frees; using a released handle returns an error with code `StaleHandle` rather than undefined behaviour. That code belongs to the binding, not to `EngineError`: the engine has no handles, so the WASM binding raises it before calling the engine (D-022).
- Handles do **not** survive a worker restart. On restart the client replays from the project document, which is the only durable state; replay arrives with the project document API (US7), and until then the shell reports the crash and the maker re-imports. The client commits a change to that document only after the operation's success response, so replay never repeats an operation that crashed.

### Worker crash

A panic in the engine traps the WASM instance; it cannot be caught on `wasm32-unknown-unknown`, and a trapped instance is never called again ([ADR 0009](../../../docs/adr/0009-wasm-panic-is-a-worker-crash.md)).

- The worker treats **any** exception from an export as a crash. It sends `EngineCrashed` as the terminal message for the request in flight, then `crashed`, then handles nothing more.
- On `crashed`, or on the `Worker`'s `error` event, the client answers every other pending request with `EngineCrashed`. It detaches its listeners before terminating the worker, so no request gets a second terminal message. It then increments the worker generation, emits one `restarted` event, and starts a new worker on the next request.
- The client never gives the app a bare wire handle. It wraps each one with the worker generation it was issued in (`{ generation, handle }`, a type of `packages/engine-client`'s own), so the wire types above stay as they are. A handle from an earlier generation gets `StaleHandle` from the client without reaching the worker, even if the new worker has reused its number.
- Cancelling a request that already has its terminal message does nothing. Progress from an earlier generation is dropped.

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
  code: string;                     // stable: an EngineError variant name, or a binding-level code (StaleHandle,
                                    // EngineCrashed), defined in openplush-wasm's own error type so the TS
                                    // generator sees it. EngineCrashed is listed there for the generator only: the host raises it, never Rust
  message: string;                  // plain language, shown to the maker
  locations?: { faces?: number[]; edges?: number[]; vertices?: number[]; pieces?: number[];
                charts?: number[]; chains?: number[]; components?: number[] };
  proposals?: Proposal[];           // e.g. ChartNotDisk carries its own fix (FR-013)
};
```

The UI highlights `locations` in the 3D view. A code the UI does not recognise must still display `message` and the locations — forward compatibility, so an engine ahead of the shell degrades to "something is wrong, here" instead of silence.

## 7. Testing

- `packages/engine-client` is tested against a **mock worker** implementing this contract, so shell tests need no WASM build. The mock can crash on demand, so the crash rules in §4 are tested without a real panic.
- The real worker is exercised by Playwright smoke tests on three fixtures (plan "CI").
- Generated TS types are checked into the repo and regenerated in CI; a diff fails the build, which is how drift is caught rather than discovered.
