# Spike S1: does `baby_shark` build and run on `wasm32-unknown-unknown`?

**Status (2026-10-14): closed.** Decision: [ADR 0001](../../docs/adr/0001-preprocessing.md) (Proposed).

This is non-shipping research code. Nothing in `crates/`, `apps/`, or `packages/` imports it. It is a standalone Cargo project with its own `[workspace]` table and `Cargo.lock`, so the root workspace (`members = ["crates/*"]`) and `scripts/check.sh` never see it.

## The question

Task T001 asks whether `baby_shark` builds and runs on `wasm32-unknown-unknown`. Research R2 picked it for three preprocessing jobs:

- isotropic remeshing before flattening, because triangle quality drives solver conditioning;
- decimation for preview;
- voxel remeshing as the opt-in last-resort repair (data-model §3).

If it does not work on wasm32, isotropic remeshing becomes native-only and WASM flattening runs on the raw mesh. That changes what the interior solve (T080) can assume about its input.

"Builds" is not "runs": a rayon pool or `Instant::now` only fails at runtime. So the spike runs every operation under Node and compares the output byte for byte with native.

## Method

| Step | What | Why |
|---|---|---|
| Inventory | `cargo info`, the published `Cargo.toml`, and `cargo tree --target wasm32-unknown-unknown`; grep the crate source for `rayon`, threads, clock, fs, RNG, and panics | Finds what breaks wasm32 or the repo's gates before any build |
| Policy gates | `cargo deny check licenses bans` against the repo's `deny.toml` | Licenses must be on the allowlist; `rayon` is banned graph-wide |
| Harness | `src/lib.rs` builds a unit icosphere in code (subdivision 3: 642 vertices, 1,280 faces) and runs `IncrementalRemesher`, `EdgeDecimator` (quadric), and `VoxelRemesher` (marching cubes and dual contouring) | Exercises every operation R2 and data-model §3 name, on a closed mesh with no file I/O |
| Fingerprint | FNV-1a over vertex positions (bit patterns, in the mesh's iteration order), then faces as compacted indices | Byte-identical output gives an identical hash |
| Cross-target test | `tests/cross_target.rs` asserts each fingerprint against the value the native release binary printed. The same file runs as `#[test]` natively and as `#[wasm_bindgen_test]` on wasm32 (`wasm-bindgen-test-runner` 0.2.129 under Node 22) | A pass on wasm32 means the operation ran and matched native exactly |
| Repeat runs | Native binary run 10 to 20 times as separate processes, with and without `RAYON_NUM_THREADS=1` | `std` `HashMap` keys and rayon scheduling change between processes; a single run would hide either |
| Patch experiment | A scratch copy of `baby_shark` with `rayon`, `wasm-bindgen`, and `getrandom` made optional and `faer` default features off. Same tests, same hashes, `cargo deny` again, and a size probe. Not committed; the diff is in `results.md` | Shows whether an upstream feature flag is all that stands between the crate and the repo's gates |

## Run it

From this directory, with the repo toolchain (`rust-toolchain.toml` at the root applies here too) and `wasm-bindgen-test-runner` 0.2.129 (`scripts/check.sh --setup` installs it):

```sh
cargo run --release                                               # native fingerprints + timings
cargo test --release                                              # native oracle
RAYON_NUM_THREADS=1 cargo test --release                          # native, exact dual-contouring hash
cargo test --release --target wasm32-unknown-unknown              # same tests on wasm32 under Node
cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans
```

The root `.cargo/config.toml` sets `wasm-bindgen-test-runner` as the wasm32 runner. Debug-profile wasm tests pass too, but take about 60 s.

## Findings

Full output in [`results.md`](./results.md).

1. **It builds and runs on wasm32, and matches native byte for byte** [solid]. Isotropic remeshing, decimation, and both voxel meshers run under Node with no panic. Every fingerprint equals the native one. Rayon does not panic on wasm32: `rayon-core` 1.13 falls back to the current thread when it cannot spawn threads.
2. **It cannot be adopted as published** [solid]. It fails two of the repo's gates, and it has **no Cargo features** to switch the causes off:
   - `rayon` is an unconditional dependency, both directly and through `faer` 0.22 with default features. `cargo deny check bans` fails on `rayon` and `rayon-core`. `scripts/check-boundary.sh` rule 1 would fail any engine crate that depends on it.
   - On wasm32 it unconditionally depends on `wasm-bindgen` and `getrandom` (feature `js`, which pulls in `js-sys`), and compiles its own `#[wasm_bindgen]` bindings. Any wasm artifact that links it gains about 1,400 extra exports (`Mesh`, `PreparedDeform`, `BoundaryRing`, ...) and wasm-bindgen placeholder imports. That is boundary rule 1 again, and a public JS surface we did not write.
   - The licenses are fine: `cargo deny check licenses` passes on the whole graph.
3. **A small upstream change would clear both gates** [solid for this harness]. Making `rayon`, `wasm-bindgen`, and `getrandom` optional, turning off `faer`'s default features, and making three call sites sequential gives a graph with no `rayon`, `wasm-bindgen`, `js-sys`, `getrandom`, or `rand`. `cargo deny` bans and licenses pass, and all five fingerprints are unchanged on native and wasm32. The size probe drops from 891 KB (265 KB gzip) to 238 KB (85 KB gzip), and the extra exports drop from about 1,400 to 0.
4. **Determinism.** Remeshing, decimation, and marching-cubes voxel output were identical across 10 native processes and identical to wasm32 [likely: one input; `HashMap` is used for lookups only on these paths]. **Dual-contouring voxel output is not deterministic natively**: 20 runs on 4 threads gave 20 different hashes, because each leaf's faces are appended under a `Mutex` in thread-completion order. With one thread (`RAYON_NUM_THREADS=1`, or wasm32) it is stable, and wasm32 matches the single-thread native hash [solid].
5. **Library defects that a wrapper would have to guard** [solid]:
   - `min_faces_count(Some(320))` is ignored: unconditional decimation of the 1,280-face sphere returns 2 faces on 3 vertices, with no error.
   - The decimation threshold is an absolute quadric error in model units squared, with a cliff: 0.01 removes nothing, 0.1 leaves 204 faces.
   - The dual-contouring tile paths contain `todo!()` and `unimplemented!()`. On wasm32 a panic aborts the instance.
   - Voxel output is an unwelded triangle soup (vertices = 3 × faces).
   - All operations are one blocking call with no step or yield API. Remeshing takes about 105 ms natively at 9k output faces, which already breaks the ~50 ms yield rule in `crates/AGENTS.md`. Without an upstream change, it can only run as one uninterruptible call in the worker.
