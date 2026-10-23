---
status: Proposed
date: 2026-10-15
deciders: maintainer (proposed by the implementing agent from spike 001 T004)
---

# v0.2 nesting builds its own deterministic placer on a gate-clean `jagua-rs` core; `sparrow` is not a dependency; v0.1 layout is unchanged

## Context and problem statement

Research R2 picked `jagua-rs` as the v0.2 layout engine and `sparrow` (same author) as its optimizer. Data-model §6 passes layout one rotation set per fabric ({0°} napped, {0°, 180°} non-napped, {45°, 225°} bias; 001 D-046 later dropped bias and gave felt any quarter turn) and a mirror axis for mirrored copies. FR-053 says layout honours those sets, and FR-062 defers optimised nesting past v0.1. 001 T006 found that jagua-rs 1.1.0 and sparrow both pull `rayon`, which `deny.toml` bans.

001 T004, a spike, asks: how big is jagua-rs on wasm32, and does it support discrete rotation sets? A size surprise would change the v0.1 layout design even though nesting is v0.2. This ADR records the answer, what v0.2 nesting will be built from, and what v0.1 must keep open. Evidence: [`research/s4_jagua_rs_wasm/`](https://github.com/openplush/openplush/blob/main/research/s4_jagua_rs_wasm/README.md).

## Decision drivers

* Constitution I and its enforcement: no `wasm-bindgen`, `js-sys`, `rayon`, or `getrandom` reachable from `crates/` (`scripts/check-boundary.sh` rule 1). `cargo deny` bans `rayon` graph-wide and denies unknown git sources.
* Constitution V and SC-008: byte-identical output across runs and platforms. A nest that depends on wall-clock time fails this.
* Constitution III: no silent wrong output. A placement that breaks a piece's rotation set is a wrong pattern, because the nap ends up running the wrong way.
* Constitution IX: allowlisted licenses (MPL-2.0 is on the list).
* The constitution's Technical Constraints: `f64` throughout geometry.
* `crates/AGENTS.md`: no loop runs longer than ~50 ms without a yield point.
* Bundle size for a static PWA, and FR-053, FR-062, and US6.

## Considered options

1. Adopt `jagua-rs` 1.1.0 and `sparrow` as published.
2. Adopt `jagua-rs`'s core (no `spp`/`bpp`) once a crates.io release makes `rayon`, `web-time`, and `getrandom` optional. Write our own deterministic placer over its collision engine in `openplush-layout`. Don't depend on `sparrow`.
3. Fork or vendor patched `jagua-rs` and `sparrow` as pinned git dependencies.
4. Write our own nesting: collision detection over the `i_overlay` 9 and `f64` geometry the engine already has, plus our own placer.

## Decision outcome

Chosen option: **2**, with **4** as the fallback. jagua-rs does what nesting needs and runs bit-identical on wasm32. What blocks it is three manifest lines, not the design. sparrow is the strongest optimizer, but every way of taking it as a dependency breaks a gate or determinism.

What 001 T004 found:

* **Discrete rotation sets are first-class** [solid]. `ExtRotation::Discrete { angles }` imports into `jagua_rs::geometry::AllowedOrientations`, which holds a `RotationRange` (`None | Continuous | Discrete(Vec<f32>)`, radians). {0°} becomes `None`. {0°, 180°} becomes `Discrete([0, π])`. A reflection axis maps a nap-parallel mirror (local +Y) to "reflect about x, then rotate by π". `AllowedOrientations::allows` checks a pose. **But `Layout::place_item` checks neither orientation nor collisions** [solid], so our placer has to enforce both. In the harness, {0°, 180°} used the half turn on 3 of 6 pieces and cut the strip from 186 mm to 156 mm.
* **It runs on wasm32, bit-identical to native** [solid]. Two nests and three rotation-set checks pass under Node with the same hashes as native release and debug builds and as 20 separate native processes.
* **As published, it fails the gates** [solid]. `rayon`, `getrandom` (`wasm_js` → `wasm-bindgen`), and `web-time` (→ `js-sys`) are unconditional, and its only features are `spp` and `bpp`. `cargo deny` fails bans; licenses, sources, and advisories pass. Boundary rule 1 fails on four crates.
* **A small upstream change clears both gates without changing output** [solid for the 001 T004 harness]. `rayon` is used only by the `spp`/`bpp` importers and `web-time` only by their solutions, and `getrandom` is never referenced. Moving the first two behind those features and dropping the third gives a graph that passes `cargo deny` bans and licenses, with all five tests and hashes unchanged on both targets.
* **Bundle cost is moderate** [solid for the probe]. An empty wasm-bindgen cdylib is 12.7 KB (5.8 KB gzip). With jagua-rs and a minimal placer at `opt-level = "z"`, LTO, and `panic = "abort"`, it is 376.9 KB (140.8 KB gzip): about **+364 KB raw and +135 KB gzip**. `"s"` gives 424.6 KB (158.6 KB gzip). About 16% of the code is `geo` and `i_overlay` 4.5, reached through jagua's shape-modification path, which is a second major version beside the engine's `i_overlay` 9. There is no `wasm-opt` number because the tool isn't installed [verify].
* **sparrow** [solid]: its library builds and runs on wasm32 and avoids its own thread pool there. With a caller-supplied iteration-count `Terminator` and `FailureBased` shrink decay, it is bit-identical native and wasm32 and across worker counts on the 001 T004 instance, and it packs much tighter than the greedy placer (111 mm against 156 mm at 400 polls). It costs about 565 KB raw and 200 KB gzip in total. But it is not on crates.io (the crates.io `sparrow` is an unrelated crate), has no `license` field, depends unconditionally on `rayon`, `clap`, `fern`, `num_cpus`, `jiff`, and `test-case`, and fails `cargo deny` bans and sources. Its defaults stop on wall-clock time (9 + 1 minutes) and shrink by elapsed time, so the same seed gives different layouts on different machines. One call blocks with no yield point (1.3 s natively at 400 polls).
* **jagua-rs computes in `f32`** [solid]. An `f32` step is about 0.1 µm at 1.5 m, harmless for placement. But it conflicts with "`f64` throughout geometry".

Why not the others:

* **Option 1** fails `cargo deny` bans (both crates) and sources (sparrow), and boundary rule 1. sparrow's defaults also break Constitution V.
* **Option 3** adds a new source category (`allow-git`) and two forks to rebase. For sparrow it would also mean carrying a determinism patch.
* **Option 4** is the fallback (see More information). Principle X says to measure the baseline first. jagua-rs's quadtree and fail-fast surrogates are real work to rebuild, and 001 T004 shows they already pass every check except a manifest one.

### Consequences

* **v0.1 is unchanged** [solid]. +135 KB gzip is a moderate cost, not a surprise. v0.1 layout (page tiling, and FR-062's simple-rule placement that reports fabric length) doesn't use jagua-rs. Three things keep v0.2 a mapping rather than a redesign:
  * the layout input keeps a fabric's rotation set as an explicit **list of discrete angles** plus an optional **mirror axis**, both derived from the fabric profile, never stored (data-model §6a). This is the shape `ExtOrientation` takes [likely];
  * v0.1's simple placer enforces the same sets and reports a violation as a typed error, so FR-053 means the same before and after v0.2;
  * the napped "±3° tolerance" is a discrete angle list or {0°}, because jagua-rs has no small-range form. Decide which when v0.2 nesting is specified [speculative].
* **Bundle budget for v0.2** [likely]: plan for about 400 KB raw and 150 KB gzip for nesting, about 40 times the empty baseline. Load it as its own module, only when the maker asks for a fabric layout, so the first load of the v0.1 app doesn't pay for it [speculative; depends on how `openplush-wasm` is split]. Measure again with `wasm-opt` before fixing the budget.
* **Precision**: `openplush-layout` treats jagua-rs as a **placement search**. It converts outlines to `f32` at its adapter, takes back only rotation, reflection, and translation, applies them to the `f64` outlines, and verifies the result with its own `f64` overlap and rotation-set check. A failure is a typed error, never a clamp. This ADR proposes that as the documented exception to "`f64` throughout geometry" for nesting search only [likely].
* **The placer is ours.** It starts constructive and deterministic (the 001 T004 greedy bottom-left is about 60 lines). Later it can do bounded, seeded local search, with an explicit work budget instead of a time budget, and step-wise with a yield point in each step. sparrow's published algorithm (arXiv:2509.13329, cited in its source) is the reference for that search, not a dependency [speculative on how close it gets].
* Good: no new source category, no fork, no `deny.toml` exception. The gates enforce this decision: a PR that adds `jagua-rs` 1.1.0 or `sparrow` fails CI.
* Bad: v0.2 nesting depends on an upstream release we don't control [speculative on timing], and our placer will pack worse than sparrow until it has a local search [solid: 156 mm against 111 mm on the 001 T004 instance].
* Bad: nesting brings `geo` and a second `i_overlay` major version, which also counts toward bundle size.

### Confirmation

* `cargo deny check bans` and `scripts/check-boundary.sh` rule 1 reject `jagua-rs` 1.1.0 and `sparrow` in any engine crate. While this decision stands, CI is the check.
* Adoption needs a crates.io release of jagua-rs where `rayon`, `web-time`, and `getrandom` are optional or gone. To check one, bump the pin in `research/s4_jagua_rs_wasm/Cargo.toml` and run its five native and wasm32 tests and `cargo deny`. All must pass before `openplush-layout` adds the dependency. If the release changes placements, the hashes change, and that is reported, not re-recorded silently.
* The v0.2 nesting tasks carry an oracle that no placed piece breaks its fabric's rotation set or overlaps another in `f64`, independent of jagua-rs's own checks.

## Pros and cons of the options

### 1. Adopt both as published

* Good, because it is the least work and gives the tightest packing.
* Bad, because it fails `cargo deny` bans and sources and boundary rule 1, and sparrow's defaults are wall-clock-driven.

### 2. Gate-clean jagua-rs core and our own placer (chosen)

* Good, because it is gate-clean and deterministic by construction, and jagua-rs already handles discrete sets, mirrors, minimum separation, and fabric flaws (quality zones).
* Good, because the upstream change is three manifest lines and output-preserving.
* Bad, because it waits on upstream, packs less tightly at first, and stays `f32` inside the search.

### 3. Fork or vendor both

* Good, because it is available now, and 001 T004's patch already passes the gates.
* Bad, because it adds a new git source category and two forks to maintain, and sparrow still needs a determinism patch.

### 4. Own nesting over `i_overlay` 9

* Good, because it is `f64`, adds no new dependency or second `i_overlay`, and is step-wise by design.
* Bad, because the collision engine (quadtree, surrogates) is real work to rebuild, before any measurement shows it's needed.

## More information

* Evidence: [`research/s4_jagua_rs_wasm/README.md`](https://github.com/openplush/openplush/blob/main/research/s4_jagua_rs_wasm/README.md) and [`results.md`](https://github.com/openplush/openplush/blob/main/research/s4_jagua_rs_wasm/results.md). These hold the exact commands, versions, hashes, the patch diff, the size table, and the sparrow probe.
* Upstream: propose the 001 T004 patch to jagua-rs (`rayon` and `web-time` behind `spp`/`bpp`, `getrandom` dropped). For sparrow, a `license` field in `Cargo.toml` and a crates.io release would make it reconsiderable, but `rayon` and time-based termination remain [speculative].
* Follow-ups for the lead's docs PR: research R6 001 T004 result; tasks.md 001 T004; research R2 `jagua-rs` and `sparrow` rows; data-model §6 ("001 T004 confirms discrete rotation sets") and the napped tolerance row; plan v0.2 roadmap ("`jagua-rs` nesting"); research 001 T006's `[verify]` note on `rayon` via jagua-rs and sparrow.
* Revisit when (a) jagua-rs publishes the opt-out, (b) v0.2 nesting is specified, or (c) the upstream change is declined. If (c), option 4 is the preferred fallback over a fork [speculative].
* The jagua-rs paper (INFORMS Journal on Computing, per research R2) and the sparrow algorithm (arXiv:2509.13329, cited in `sparrow/src/optimizer/mod.rs`).
