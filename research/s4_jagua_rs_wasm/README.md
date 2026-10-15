# Spike S4: `jagua-rs` on wasm32, bundle size, and discrete rotation sets

**Status (2026-10-15): closed.** Decision: [ADR 0007](../../docs/adr/0007-layout-nesting-engine.md) (Proposed).

This is non-shipping research code. Nothing in `crates/`, `apps/`, or `packages/` imports it. It is a standalone Cargo project with its own `[workspace]` table and `Cargo.lock`, and so is `probe/`. The root workspace (`members = ["crates/*"]`) and `scripts/check.sh` never see either.

## The question

Task T004 (research R6, S4) asks two things about `jagua-rs`, the nesting engine research R2 picked for v0.2:

- **Bundle size.** Nesting is v0.2, but v0.1 already fixes the layout design and the WASM bundle. If jagua-rs is large, that changes what v0.1 should plan for.
- **Discrete rotation sets.** Data-model §6 passes layout one rotation set per fabric: {0°} for napped fabric, {0°, 180°} for non-napped, {45°, 225°} for bias. Mirrored copies reflect about a named axis. Does jagua-rs express these, and does it enforce them?

S6 left a third question: jagua-rs 1.1.0 and `sparrow` (the optimizer from the same author) both pull `rayon`. Can that be switched off?

## Method

| Step | What | Why |
|---|---|---|
| Inventory | `cargo info`, the published `Cargo.toml`, and `cargo tree --target wasm32-unknown-unknown -e normal`. A grep of the crate source for `rayon`, clock, RNG, hash containers, and panics | Finds what breaks the repo's gates before any build |
| Policy gates | `cargo deny check licenses bans sources advisories` against the repo's `deny.toml`, plus `scripts/check-boundary.sh` rule 1 read against the tree | `rayon` is banned graph-wide; boundary rule 1 bans `wasm-bindgen`, `js-sys`, `rayon`, and `getrandom` from engine crates |
| Harness | `src/lib.rs` imports six asymmetric polygons (mm) with `ExtRotation::Discrete`, then places them on a 150 mm fabric strip with a greedy bottom-left placer of about 60 lines over jagua-rs's `Layout` and `CDEngine`. Run once with {0°} and once with {0°, 180°} | jagua-rs is a collision-detection engine, not an optimizer. This is the smallest layout that exercises import, rotation sets, collision queries, and placement |
| Cross-target test | `tests/cross_target.rs` asserts the rotation-set semantics and each nest's hash against the native release output. It runs as `#[test]` natively and as `#[wasm_bindgen_test]` on wasm32 (Node 22) | A pass on wasm32 means the placements are bit-identical to native |
| Repeat runs | The native binary run 20 times as separate processes | Catches per-process hash seeds and scheduling |
| Size probe | `probe/` is a `cdylib` with one `#[wasm_bindgen]` export. Without features it is an empty baseline; `--features jagua` links the harness. `probe/measure.sh` builds both at `opt-level` `z` and `s` (LTO, one codegen unit, `panic = "abort"`, strip), runs `wasm-bindgen --target nodejs`, calls the export under Node, and prints raw and gzip sizes | The delta over the baseline is what jagua-rs plus a minimal placer cost |
| Patch experiment | A scratch copy of jagua-rs with `rayon` and `web-time` moved behind the `spp`/`bpp` features and `getrandom` removed. Same tests, `cargo deny`, and size probe. Not committed; the diff is in `results.md` | Shows whether a feature flag upstream is all that stands between the crate and the gates |
| `sparrow` | Clone at `1fe3e31`. Build its library for wasm32, run `cargo deny`, and run a scratch probe that calls `sparrow::optimizer::optimize` on the same six pieces with an iteration-count `Terminator`. Native vs wasm32, sizes. Not committed; the probe is summarised in `results.md` | S6 asked whether sparrow's library can run on wasm32 without `rayon` |

## Run it

From this directory, with the repo toolchain (the root `rust-toolchain.toml` applies here) and `wasm-bindgen-test-runner` 0.2.129 (`scripts/check.sh --setup` installs it):

```sh
cargo run --release                                     # native nests + timings
cargo test --release                                    # native oracle
cargo test --release --target wasm32-unknown-unknown    # same tests on wasm32 under Node
cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans sources advisories
(cd probe && bash measure.sh)                           # bundle sizes; needs wasm-bindgen 0.2.129 and node
```

The root `.cargo/config.toml` sets `wasm-bindgen-test-runner` as the wasm32 runner.

## Findings

Full output in [`results.md`](./results.md).

1. **It builds and runs on wasm32, bit-identical to native** [solid]. Both nests and all rotation-set checks pass under Node. Native is identical across 20 processes. Native, wasm32 release, and wasm32 debug give the same hashes. The core library uses no hash-container iteration, no clock, and no RNG on the path the harness exercises.
2. **Discrete rotation sets are first-class** [solid]. `ExtRotation::Discrete { angles }` (degrees) imports into `AllowedOrientations` (`jagua_rs::geometry`), which holds a `RotationRange` (`None | Continuous | Discrete(Vec<f32>)`, radians). {0°} becomes `RotationRange::None`. {0°, 180°} becomes `Discrete([0, π])`. Reflection axes become `rotations_after_reflection`: a mirror across the nap-parallel axis (local +Y, 90°) is "reflect about x, then rotate by π". `AllowedOrientations::allows` checks a pose. **But `Layout::place_item` checks neither collisions nor orientation**, so the placer has to enforce both. With {0°, 180°}, the greedy nest used a half turn on 3 of 6 pieces, and the strip got shorter: 156 mm instead of 186 mm.
3. **As published, it fails the repo's gates, and no feature turns the causes off** [solid]. Its features are `spp` and `bpp` only, both off by default. `rayon`, `getrandom` (feature `wasm_js`, which brings `wasm-bindgen`), and `web-time` (which brings `js-sys` and `wasm-bindgen`) are unconditional. `cargo deny` fails bans (`rayon`, `rayon-core`). Licenses, sources, and advisories pass. Boundary rule 1 would fail on all four banned crates.
4. **A small upstream change clears both gates without changing output** [solid for this harness]. `rayon` is used only by the `spp`/`bpp` importers and `web-time` only by their solutions, so moving both behind those features and dropping `getrandom` (nothing in the source calls it) removes all four from the normal graph. With that change, `cargo deny` bans and licenses pass, and all five tests pass on both targets with the same hashes.
5. **Bundle size: about +360 KB raw, +135 KB gzip** [solid for this probe]. Empty wasm-bindgen baseline: 12.7 KB (5.8 KB gzip). With jagua-rs and the greedy placer at `opt-level = "z"`: 376.9 KB (140.8 KB gzip). At `"s"`: 424.6 KB (158.6 KB gzip). The patch hardly changes this (374.3 KB, 139.4 KB gzip), because the banned crates were in the graph but not linked. `wasm-opt` is not installed here, so there is no post-`wasm-opt` number [verify]. Code attributed by symbol: about 10% is jagua-rs itself, about 16% is the `geo` and `i_overlay` 4.5 family (reached through the importer's shape-modification path, even with simplification and offset off), and the rest is monomorphised `core`/`alloc`, `anyhow`, the allocator, and formatting.
6. **sparrow works on wasm32 but is not adoptable as a dependency** [solid]. Its library builds for wasm32 and avoids its own thread pool there. Driven by an iteration-count `Terminator`, with `ShrinkDecayStrategy::FailureBased` and a fixed seed, it is deterministic: native gives the same result across 10 processes, with 1 and with 3 workers, and wasm32 matches native bit for bit (50 polls: 144.63 mm; 400 polls: 111.37 mm against the greedy 156 mm). It adds about 565 KB raw and 200 KB gzip in total. It fails `cargo deny` bans (`rayon` is unconditional), has no `license` field, is not on crates.io, and its defaults are **wall-clock-driven**: a 9 + 1 minute time limit and `TimeBased` shrink decay, so out of the box the same seed gives different layouts on different machines. One `optimize` call blocks for its whole run (1.3 s natively at 400 polls) with no yield point.
7. **jagua-rs computes in `f32`** [solid]. Every coordinate, angle, and transform is `f32`. At 1.5 m of fabric, an `f32` step is about 0.1 µm, which is harmless for placement. But the constitution says `f64` throughout geometry, so whatever nests with jagua-rs must treat it as a placement search and apply and verify the final rigid transforms in `f64`.
