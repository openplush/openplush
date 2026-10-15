---
status: Proposed
date: 2026-10-15
deciders: maintainer (accepts in review of this PR); proposed by the implementing agent
---

# A panic in the WASM engine is a worker crash, not a typed error

## Context and problem statement

`crates/AGENTS.md` said "`openplush-wasm` wraps each export and converts panics to typed errors", and ADR 0006 planned to catch `i_overlay` panics at the allowance stage boundary. On the pinned toolchain (1.97.0), `wasm32-unknown-unknown` builds with `panic = "abort"`: a panic traps the instance and `catch_unwind` catches nothing. [`research/wasm_panic_probe/`](../../research/wasm_panic_probe/README.md) confirms this and shows that the trapped instance is unsafe to reuse. So the rule cannot be implemented as written. If native code caught a panic that the browser cannot, the same input would end in a typed error on one target and a crash on the other (Constitution V).

The worker protocol has to say what the shell sees when the engine dies mid-request, and what happens to the requests and handles that were in flight.

## Decision drivers

* Constitution V: the same input gives the same outcome on both targets, errors included.
* Constitution III and AGENTS.md, Fail loudly: no silent fallback. A crash must reach the maker as a crash, not as missing output.
* The pinned stable toolchain (Constitution, Technology Constraints): no nightly-only features.
* The worker protocol's invariant: exactly one terminal message per request `id` (§2).

## Considered options

1. **A panic is a bug that crashes the worker.** The host reports it with a code and restarts the worker.
2. **Nightly `-Zbuild-std` with `panic = "unwind"` and wasm exception handling**, so `catch_unwind` works.
3. **Keep the rule as written** and find out at the first export.

## Decision outcome

Chosen option: **1**, because it is the only one that works on the pinned stable toolchain and behaves the same on both targets.

* **No engine code uses `catch_unwind`.** A panic stays what `crates/AGENTS.md` already says it is: a bug, allowed only behind an `// INVARIANT:`. The native CLI exits with Rust's panic code (101); the browser reports `EngineCrashed`. Both are "the engine crashed", on both targets.
* **`openplush-wasm` installs a panic hook at start** that forwards the panic message to the worker console. Without it, the message is lost: the default hook writes to stderr, which has no destination on this target [likely]. Out of memory aborts without running the hook, so the host never depends on a message.
* **The worker wrapper treats any exception from an export as a crash**: `RuntimeError` from a trap, `RangeError` from stack exhaustion, anything else. It posts `EngineCrashed` as the terminal message for the request in flight, posts `crashed`, and handles no further messages. The instance is never called again.
* **The client**, on `crashed` or on the `Worker`'s `error` event (which carries no request id):
  1. answers every other pending request with `EngineCrashed`;
  2. detaches its listeners **before** terminating the worker, so nothing from the old worker can arrive as a second terminal message;
  3. increments the worker generation and emits one `restarted` event;
  4. starts a new worker on the next request.
* **Handles carry the worker generation** on the client side. A handle from an earlier generation gets `StaleHandle` from the client without being sent, even if the new worker has reused its number. `StaleHandle` therefore has two sources: the binding (a released handle) and the client (an earlier generation).
* **Cancelling a request that already has its terminal message is a no-op.** Progress messages from an earlier generation are dropped.
* **`EngineCrashed` is listed in the binding's own code list**, next to `StaleHandle`, so the code has one Rust definition that T179(b) exports to TypeScript (`EngineErrorDto.code` itself stays a `string`, so an unknown code still displays). Only the host raises it.
* **Replay.** The client commits a change to the project document only after the operation's success response. So the last committed document never contains the change that crashed, and replaying it after a restart cannot repeat the crash. Replay needs the project document API (US7). Until then, the shell reports the crash and the maker re-imports.

### Consequences

* Good, because it works on stable, and a crash is loud and located in time: the maker sees which action failed.
* Good, because native and WASM agree. A panic is a crash on both, and no code path depends on catching one.
* Bad, because a panic loses the operation, and before US7 the session's in-memory state too. This is acceptable only because a panic is a bug: input validation is the defence (ADR 0006 steps 1–6), not recovery.
* Bad, because `StaleHandle` now has two raisers. The code means the same thing from both: "this handle refers to nothing".
* Neutral: `#[wasm_bindgen]` builds under the workspace's `unsafe_code = "forbid"` (probe finding 1), so the first export needs no `unsafe` exception.

### Confirmation

* [`research/wasm_panic_probe/`](../../research/wasm_panic_probe/README.md) records the commands, versions and output this ADR rests on.
* T040's tests against the mock worker:
  * a crash with N requests pending gives N `EngineCrashed` responses and no second terminal message for any id;
  * a cancel that races the crash is a no-op;
  * the next request runs on generation 2, an earlier generation's handle gets `StaleHandle`, and `restarted` fires once.
* T099 wraps every export call in the real worker.

## Pros and cons of the options

### Nightly `panic = "unwind"`

* Good, because a panic could become a typed error, located like any other.
* Bad, because it needs nightly (`-Zbuild-std`) and wasm exception-handling support in every target browser, against the pinned stable toolchain.
* Bad, because a caught panic still leaves engine state half-updated. Recovery would need an audit of every unwind path.

### Keep the rule

* Bad, because it cannot be implemented. The first export would discover that, after code had been written against it.

## More information

* As built, once T040 and T099 land: `docs/reference/` gets the worker protocol page. Until then the contract is [`contracts/worker-protocol.md`](../../specs/001-pattern-pipeline/contracts/worker-protocol.md) §4 and §6.
* Changes ADR 0006 step 7, D-026, the T003 note, research.md's S3 recommendation 5 and data-model §7's Robustness bullet. Each said "catch panics at the stage boundary".
