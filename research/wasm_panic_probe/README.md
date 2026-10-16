# Probe: what a Rust panic looks like from JavaScript on wasm32

**Status (2026-10-15): closed.** Decision: [ADR 0009](../../docs/adr/0009-wasm-panic-is-a-worker-crash.md) (Accepted).

This is non-shipping research code. Nothing in `crates/`, `apps/`, or `packages/` imports it. It is a standalone Cargo project with its own `[workspace]` table, `Cargo.lock` and `rust-toolchain.toml`, so the root workspace and `scripts/check.sh` never see it.

## The question

`crates/AGENTS.md` said `openplush-wasm` "wraps each export and converts panics to typed errors". ADR 0006 planned to catch `i_overlay` panics at the allowance stage boundary. Both assume a panic can be caught on `wasm32-unknown-unknown`. This probe checks that, and what the worker sees instead.

## Method

`src/lib.rs` is a `cdylib` with the workspace's edition (2024) and the lint levels that matter here (`unsafe_code = "forbid"`, the panic lints). It exports:

| Export | What it does |
|---|---|
| `start` (`#[wasm_bindgen(start)]`) | Installs a `std::panic::set_hook` hook that forwards the message to an imported `console.error` |
| `ok` | Reads a `thread_local!` `RefCell`; proves the instance works |
| `caught` | Panics inside `std::panic::catch_unwind` and reports whether it was caught |
| `boom_while_borrowed` | Panics while that `RefCell` is mutably borrowed |
| `oom` | Allocates 256 MiB blocks until allocation fails |
| `recurse` | Recurses without bound |

`run.cjs` calls each from Node and prints what returned or what was thrown.

## Run it

```zsh
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target nodejs --out-dir pkg target/wasm32-unknown-unknown/release/wasm_panic_probe.wasm
node run.cjs
```

Versions: rustc 1.97.0 (2026-07-07), wasm-bindgen 0.2.129 (lib and CLI), Node v22.22.0. `rustc --print cfg --target wasm32-unknown-unknown` reports `panic="abort"`; the host target reports `panic="unwind"`.

## Results

```
ok (before): returned 0
probe hook: panicked at src/lib.rs:45:28:
inside catch_unwind
caught: threw RuntimeError: unreachable
ok (after caught): returned 0
probe hook: panicked at src/lib.rs:37:9:
deliberate panic while borrowed
boom_while_borrowed: threw RuntimeError: unreachable
probe hook: panicked at src/lib.rs:27:22:
RefCell already mutably borrowed
ok (after trap): threw RuntimeError: unreachable
oom: threw RuntimeError: unreachable
recurse: threw RangeError: Maximum call stack size exceeded
```

1. **`#[wasm_bindgen]` builds under `unsafe_code = "forbid"`**, with no warnings, for an export, a `start` function and an `extern` import [solid for 0.2.129]. The first real export needs no `unsafe` exception.
2. **A panic is a trap.** It reaches the JS caller as `WebAssembly.RuntimeError: unreachable`, synchronously, at the call site [solid].
3. **`catch_unwind` catches nothing.** The closure's panic traps straight through it [solid]. Any plan to convert a panic into a typed error inside Rust fails on this target.
4. **The panic hook runs before the trap** and its message reaches JS through an import [solid]. This is the only way to keep the message [likely]: on this target the default hook writes to stderr, which has no destination; the probe has no run without its own hook.
5. **A trapped instance is poisoned.** The `RefCell` borrowed when the panic hit stays borrowed, so the next, unrelated call panics too ("RefCell already mutably borrowed") [solid]. The allocator and wasm-bindgen's tables are likely in the same position, though only the `RefCell` is tested [likely]. The instance must be discarded, not reused.
6. **Out of memory traps without the hook** (no message), and **unbounded recursion surfaces as `RangeError`**, not `RuntimeError` [solid in Node 22; browsers may word it differently]. So the host must treat *any* exception from an export as a crash, not only `RuntimeError`.
7. The first call to `caught` left the instance usable (`ok (after caught)` returned) only because nothing was borrowed at the time. Reuse is not safe in general (finding 5).
