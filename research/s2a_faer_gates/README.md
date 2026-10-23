# Spike S2a: does `faer` without default features pass the repo's gates?

**Status (2026-10-18): closed. One gate fails: `cargo deny` advisories, on `paste` (RUSTSEC-2024-0436, unmaintained).** Per 001 D-041 and the maintainer's E-3, 001 T078 waits for the maintainer's decision.

Non-shipping research code. Nothing in `crates/`, `apps/`, or `packages/` imports it. It is a standalone Cargo project with its own `[workspace]` table and `Cargo.lock`, so the root workspace and `scripts/check.sh` never see it.

## The question

001 D-041 splits S2 (001 T002). S2a runs before 001 T078: does `faer` with `default-features = false` pass every gate an engine crate must pass (`cargo deny`, `scripts/check-boundary.sh`, wasm32) and give the same bits on native and wasm32 (Constitution V)? S2b, the timing, waits for SLIM (001 T080).

## Method

| Step | What | Why |
|---|---|---|
| Harness | `src/lib.rs` solves a shifted 2D grid Laplacian (SPD, 100 to 10,000 unknowns) with faer's sparse LLT, the solve SLIM's interior step repeats (001 T080) | Exercises the sparse path 001 T078 integrates, not just a build |
| Two configurations | A: `features = ["sparse-linalg"]`. B: A plus `std` (`--features std`) | `std` adds runtime SIMD dispatch for x86 and OS crates |
| Oracle | `tests/cross_target.rs`: each solution's FNV-1a fingerprint equals what the native release binary printed, and its residual, recomputed from the stencil without faer, is under 1e-10 | Bit-identical across targets, and right, not just repeatable |
| Targets | Native release and debug; wasm32 release and debug under `wasm-bindgen-test-runner` 0.2.129 (Node 22) | A pass on wasm32 means the solve ran and matched native exactly |
| Gates | `cargo tree` for the crates `check-boundary.sh` rule 1 bans; `cargo deny check licenses bans advisories sources` against the repo's `deny.toml` | The same gates CI applies to an engine crate |

## Findings

1. **Determinism: passes** [solid for this harness]. Both configurations give the same three fingerprints on native (AVX2/AVX-512 machine) and wasm32, release and debug, and across repeated runs. Residuals are 1e-14 to 8e-14.
2. **Boundary: passes for A** [solid]. No `rayon`, `rayon-core`, `getrandom`, `wasm-bindgen`, `web-sys` or `js-sys` in either configuration's tree. B also adds `libc`, `num_cpus` and `private-gemm-x86`. Nothing 001 T078 needs requires `std`, so A is the configuration to use.
3. **`cargo deny`: licenses, bans and sources pass; advisories fail** [solid]. `paste` 1.0.15 is unmaintained (RUSTSEC-2024-0436, "no safe upgrade available"). It reaches the graph through `gemm` 0.19, `gemm-common`, `gemm-f32`/`f64`/`c32`/`c64` and `pulp` 0.22, all under `faer` 0.24.4, the latest release. `paste` is a proc-macro: it runs at compile time and puts no code of its own in the engine or the wasm module. Duplicate `equator`, `equator-macro` and `syn` versions are warnings only.

## What this leaves for the maintainer

001 T078 cannot pass CI as the gates stand. The options:

- **Ignore RUSTSEC-2024-0436 in `deny.toml`**, with a reason line: compile-time only, no runtime code, no safe upgrade upstream. Revisit when `gemm`/`pulp` move to `pastey`. Smallest change; a gate exception needs the maintainer's say.
- **Wait for upstream** to replace `paste`. Blocks 001 T078 and everything after it in the flatten stage.
- **Another sparse solver.** None was evaluated here; research R2 chose `faer` for its sparse Cholesky and WASM support.

## Run it

From this directory, with the repo toolchain and `scripts/check.sh --setup`'s tools:

```sh
cargo run --release                                                    # native fingerprints
cargo test --release                                                   # native oracle, config A
cargo test --release --features std                                    # config B
cargo test --release --target wasm32-unknown-unknown --test cross_target
cargo test --release --target wasm32-unknown-unknown --test cross_target --features std
cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans advisories sources
cargo tree -i paste -e normal,build
```

The library target has no tests of its own; on wasm32, run the `cross_target` test target directly.
