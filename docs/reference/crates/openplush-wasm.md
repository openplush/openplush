# openplush-wasm

<!-- index: the WebAssembly binding: its panic hook, the worker protocol's DTOs and their generated TypeScript types. -->

What the `openplush-wasm` binding is **today**, and what it actually depends on. Like every binding, it may depend on `openplush-pipeline` and nothing else from `crates/` ([Bindings](README.md#bindings)).

## What it is today

The `wasm-bindgen` facade, a `cdylib` for `wasm32-unknown-unknown`. Its start function installs a panic hook that forwards the panic message to `console.error` ([ADR 0009](../../adr/0009-wasm-panic-is-a-worker-crash.md)); it exports only that start function.

### Protocol

`protocol`: the worker protocol's `serde` DTOs ([ADR 0010](../../adr/0010-protocol-types.md)):

- `EngineErrorDto` (`code`, `message`, optional `locations` and `proposals`), built from an `EngineError`, whose location categories are absent when empty and whose proposal IDs are 16 lowercase hex digits
- `BindingCode` (`StaleHandle`, `EngineCrashed`)
- `Progress` (tagged by `kind`: `working` with `chart`, `iteration` and `maxDistortion`, or `done`), whose `Working` is built only by `Working::new`, which refuses a NaN or infinite distortion with `ProtocolError::NonFiniteMaxDistortion`
- the envelope: `Request` (a `RequestId`, a `u32` on the wire, plus an `Op` tagged by `op` with its `payload`, which refuses unknown payload fields; `cancel { target }` today) and `Response<T>`, built only by `Response::success` (`ok: true`, `result`, `warnings`), `Response::failure` (`ok: false`, `error`) and `Response::progress`, where `warnings` is always `[]` because `WarningDto` is uninhabited
- `MeshHandle` and `PipelineHandle`, each `{ n }`, branded `'mesh'` and `'pipeline'` in TypeScript

### Generated TypeScript types

`scripts/gen-ts-types.sh` generates each DTO's TypeScript type with `ts-rs` into `packages/engine-client/src/generated/`; `scripts/check.sh types` fails if those files differ from a fresh export, contain a `bigint`, or type an optional field as `| null`.

## Depends on

`openplush-pipeline`, `serde`, `thiserror`, `wasm-bindgen`; dev: `serde_json`, `ts-rs`
