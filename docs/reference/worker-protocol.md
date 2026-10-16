# Worker protocol (`@openplush/engine-client`)

What the shell ↔ worker protocol is **today**. The wire types are generated from `openplush-wasm`'s `serde` DTOs into `packages/engine-client/src/generated/` (`scripts/gen-ts-types.sh`, checked by `scripts/check.sh types`), so this page states only what the types do not. The [protocol contract](../../specs/001-pattern-pipeline/contracts/worker-protocol.md) describes the planned whole; where the two disagree about something built, this page and the code are right.

## Package surface

| Export | Kind | Notes |
|---|---|---|
| `EngineClient` | class | `new EngineClient(spawn)`; `spawn` is called on the first request, and again on the first request after each crash. Methods: `request`, `onRestarted` |
| `Call<T>` | interface | What `request` returns: `id`, `outcome`, `cancel()` |
| `Outcome<T>` | type | `{ ok: true, result, warnings, generation }` or `{ ok: false, error }`; `generation` is the worker generation that answered |
| `RequestOptions` | type | `onProgress`, and `uses`: the handles a request refers to |
| `Issued<H>` | type | `{ generation, handle }`: a `MeshHandle` or `PipelineHandle` with the worker generation that issued it. Build it from the success that returned the handle, with that outcome's `generation`, never the current one: a crash may have moved past it |
| `WorkerPort` | interface | `post`, `listen(onMessage, onError)` (returns a detach function) and `terminate`: what the client needs of a worker |
| `WorkerMessage`, `Crashed` | types | Anything a worker sends; `Crashed` (`{ crashed: true }`) is the worker script's own message and the one hand-written message type |
| `Op`, `RequestId`, `Progress`, `EngineErrorDto`, `BindingCode`, `WarningDto`, `MeshHandle`, `PipelineHandle` | generated types | Re-exported from `src/generated/` |
| `MockWorker` | class, from `@openplush/engine-client/mock` | A `WorkerPort` for tests: records what it `received`; `send` delivers a reply asynchronously, in order; what a listener throws lands in `errors` and later deliveries still run; `fail()` fires the `error` event; `terminated` and `listening` show what the client did to it |

## Messages

- **Requests.** `request(op, options?)` posts `{ id, op, payload }`. Ids are `u32`, start at 0, increase by one per request or cancel (a `StaleHandle` request takes one but posts nothing), and are never reused; past 2^32 − 1, `request` and `cancel` throw a `RangeError`. A request whose `post` throws is never pending.
- **Responses.** A message is read as progress if it has a `progress` key, otherwise as terminal by `ok`. The client keeps the **first** terminal message for an id and ignores every later message for it, progress included. A message for an id with no pending request is ignored. A message for a pending id with neither `progress` nor a boolean `ok` makes the client throw a `TypeError` from its message listener and settles nothing.
- **Progress** reaches the request's `onProgress` only before its terminal message.
- **Warnings.** `warnings` is always `[]`: `WarningDto` is uninhabited.
- **Cancel.** `call.cancel()` posts `{ op: "cancel", payload: { target: id } }` under a new id while the request is pending, and does nothing after its outcome settles. The request still ends with its own terminal message, `Cancelled` if the worker stopped it.

## Worker crash

A crash is a `{ crashed: true }` message or the worker's `error` event, whichever comes first. On a crash the client, in order:

1. detaches from the worker and terminates it, so nothing it sends later (a second crash signal, progress, a late terminal message) reaches the client;
2. settles every pending request with `EngineCrashed` (a request the worker had already answered keeps that answer);
3. increments the generation and calls each `onRestarted` listener once with `{ generation }`. Every listener runs even if one throws; the throws are then rethrown together as an `AggregateError`. A listener added during the crash first hears the next one.

Listeners run before any caller awaiting an outcome resumes, so `restarted` is observed before the `EngineCrashed` outcomes.

The next request spawns a new worker. A `cancel()` after the crash does nothing. A request whose `uses` holds a handle from an earlier generation settles at once with `StaleHandle` and posts nothing; its id is used up but never reaches the wire.

| Code | Raised by | Message |
|---|---|---|
| `EngineCrashed` | the client, on a crash | The engine stopped unexpectedly, so this action did not finish. |
| `StaleHandle` | the client, for an earlier generation's handle | This model was loaded before the engine restarted. Load it again. |

`StaleHandle` is also the binding's code for a released handle; no op releases a handle today.

## Ops

| `op` | Payload | Result |
|---|---|---|
| `cancel` | `{ target: RequestId }` | none |

Unknown payload fields are refused by the binding; unknown top-level keys are ignored.
