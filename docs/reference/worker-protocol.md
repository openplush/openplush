# Worker protocol (`@openplush/engine-client`)

What the shell ↔ worker protocol is **today**. The wire types are generated from `openplush-wasm`'s `serde` DTOs into `packages/engine-client/src/generated/` (`scripts/gen-ts-types.sh`, checked by `scripts/check.sh types`), so this page states only what the types do not. The [protocol contract](../../specs/001-pattern-pipeline/contracts/worker-protocol.md) describes the planned whole; where the two disagree about something built, this page and the code are right.

## Package surface

| Export | Kind | Notes |
|---|---|---|
| `EngineClient` | class | `new EngineClient(spawn)`; `spawn` is called once, on the first request |
| `Call<T>` | interface | What `request` returns: `id`, `outcome`, `cancel()` |
| `Outcome<T>` | type | `{ ok: true, result, warnings }` or `{ ok: false, error }` |
| `WorkerPort` | interface | `post` and `listen`: what the client needs of a worker |
| `WorkerMessage` | type | Anything a worker sends |
| `Op`, `RequestId`, `Progress`, `EngineErrorDto`, `BindingCode`, `WarningDto` | generated types | Re-exported from `src/generated/` |
| `MockWorker` | class, from `@openplush/engine-client/mock` | A `WorkerPort` for tests: records what it `received`; `send` delivers a reply asynchronously, in order; what a listener throws lands in `errors` and later deliveries still run |

## Messages

- **Requests.** `request(op, onProgress?)` posts `{ id, op, payload }`. Ids are `u32`, start at 0, increase by one per message the client sends (cancels included), and are never reused; past 2^32 − 1, `request` and `cancel` throw a `RangeError`. A request whose `post` throws is never pending.
- **Responses.** A message is read as progress if it has a `progress` key, otherwise as terminal by `ok`. The client keeps the **first** terminal message for an id and ignores every later message for it, progress included. A message for an id with no pending request is ignored. A message for a pending id with neither `progress` nor a boolean `ok` makes the client throw a `TypeError` from its message listener and settles nothing.
- **Progress** reaches the request's `onProgress` only before its terminal message.
- **Warnings.** `warnings` is always `[]`: `WarningDto` is uninhabited.
- **Cancel.** `call.cancel()` posts `{ op: "cancel", payload: { target: id } }` under a new id while the request is pending, and does nothing after its outcome settles. The request still ends with its own terminal message, `Cancelled` if the worker stopped it.

## Ops

| `op` | Payload | Result |
|---|---|---|
| `cancel` | `{ target: RequestId }` | none |

Unknown payload fields are refused by the binding; unknown top-level keys are ignored.
