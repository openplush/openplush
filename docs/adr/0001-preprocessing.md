---
status: Proposed
date: 2026-10-14
deciders: maintainer (proposed by the implementing agent from spike 001 T001)
---

# Do not adopt `baby_shark` 0.3.12; both targets flatten the repaired mesh until a gate-clean release exists

## Context and problem statement

Research R2 picked `baby_shark` for three preprocessing jobs: isotropic remeshing before flattening (triangle quality drives solver conditioning), decimation for preview, and voxel remeshing as the opt-in last-resort repair ([data-model §3](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md#3-non-manifold-handling-repair-pipeline)). Plan decision 001 D-007 adopted it "pending T001". The plan's Technical Context also has flattening run on a remeshed proxy of at most ~30k triangles.

001 T001, a spike, asks whether the crate builds and runs on `wasm32-unknown-unknown`. If it doesn't, remeshing would be native-only and WASM would flatten the raw mesh, which changes what the interior solve (001 T080) can assume. This ADR records the answer and what the engine does about it. Evidence: [`research/s1_baby_shark_wasm/`](https://github.com/openplush/openplush/blob/main/research/s1_baby_shark_wasm/README.md).

## Decision drivers

* Constitution I and its enforcement: no `wasm-bindgen`, `js-sys`, `rayon`, or `getrandom` reachable from `crates/` (`scripts/check-boundary.sh` rule 1). `cargo deny` bans `rayon` graph-wide.
* Constitution V: byte-identical output, native and WASM. Constitution II: the CLI computes what the web app computes.
* Constitution III and VII: no silent wrong output; destructive repairs are opt-in and reported.
* Constitution IX: allowlisted licenses.
* `crates/AGENTS.md`: no loop runs longer than ~50 ms without a yield point.
* Solver conditioning for 001 T080, and SC-005's 10 s budget, which the remeshed proxy was meant to secure.

## Considered options

1. Adopt `baby_shark` 0.3.12 as published, in both targets.
2. Native-only remeshing; WASM flattens the raw mesh (the fallback 001 T001 names).
3. Don't adopt it now. Both targets flatten the repaired mesh. Propose opt-out features upstream and re-evaluate when a crates.io release has them.
4. Fork or vendor a patched `baby_shark` as a pinned git dependency.
5. Write our own isotropic remesher in `openplush-mesh` (split, collapse, flip, smooth, project; Botsch and Kobbelt 2004).

## Decision outcome

Chosen option: **3**. Neither published-crate option can pass the repo's gates, and option 3 keeps native and WASM identical without taking on a fork.

What 001 T001 found:

* **It builds and runs on wasm32, byte-identical to native** [solid]. Isotropic remeshing, quadric decimation, and both voxel meshers ran under Node through `wasm-bindgen-test`, and every output hash matched the native release build. Rayon doesn't panic: `rayon-core` 1.13 falls back to the current thread.
* **As published, it fails two gates in any crate under `crates/`** [solid]. It has no Cargo features. `rayon` is unconditional, both directly and through `faer` 0.22 default features, so `cargo deny check bans` fails. On wasm32 it unconditionally pulls `wasm-bindgen` and `getrandom` (feature `js`) and compiles its own `#[wasm_bindgen]` bindings. Those add about 1,400 exports and wasm-bindgen placeholder imports to any wasm artifact that links it, so boundary rule 1 fails. Licenses pass (MIT, whole graph).
* **A small upstream change would fix it** [solid for the 001 T001 harness]. Making those three dependencies optional, turning off `faer`'s default features, and making three voxel call sites sequential gives a graph that passes `cargo deny` bans and licenses. Every output hash is unchanged on both targets, and the size probe shrinks from 891 KB to 238 KB (265 KB to 85 KB gzipped).

Why not the others:

* **Option 1** fails `cargo deny` and the boundary check. Gates are not waived.
* **Option 2** fails the same gates: `cargo metadata` and `cargo deny` resolve every target, so a `cfg(not(target_arch = "wasm32"))` dependency is still in the graph. It would also break Constitution V and II: the CLI and the web app would flatten different meshes and print different patterns from the same input.
* **Option 4** brings in a new source category (`allow-git`) and a fork to maintain. It would also inherit the defects below.
* **Option 5** is the fallback (see More information), not the first move. Principle X says to measure the baseline first, and nothing yet shows that the repaired input mesh is too poorly conditioned or too large to flatten.

### Consequences

* Good: native and WASM flatten the same mesh, so the determinism CI job and golden tests mean the same thing in both targets.
* Good: no new dependency category, no fork, no `deny.toml` exception. The gates already enforce this decision: a PR that adds `baby_shark` 0.3.12 fails CI.
* Bad, **for 001 T080**: the interior solve can't assume isotropic, well-shaped triangles. It gets the repaired mesh: slivers, high-valence vertices, and edge lengths spread over orders of magnitude [solid]. 001 T080 should:
  * keep 001 T007's cotan-then-uniform harmonic start and its flip-capped line search. 001 T007 converged on rim slivers with minimum angles around 1.8° [likely, synthetic charts only];
  * stay scale-equivariant, never normalising by a target edge length;
  * report non-convergence as a typed error rather than returning a poor map.

  How much conditioning degrades on real scanned or sculpted fixtures is unmeasured [verify, with the fixture corpus].
* Bad, **for SC-005**: the "remeshed proxy of at most ~30k triangles" dial in the plan's Technical Context doesn't exist yet. Charts are as large as the repaired input, up to ~200k triangles in total. 001 T002 must measure SLIM on charts of raw-input size, not proxy size [likely to matter].
* Bad, **for data-model §3**: the voxel-remesh last resort is unavailable in v0.1. Hopeless input ends in the typed errors and `RepairReport` the earlier repair steps already produce. Constitution VII makes destructive repair opt-in anyway, so nothing silently changes. Preview decimation, if wanted, belongs to the shell's renderer, not the engine.
* Conditions on re-adoption, from 001 T001 [solid]. They apply to any later version, upstream or forked:
  * use marching cubes (`MeshingMethod::Manifold`) only. Dual contouring gave 20 different outputs in 20 native runs on 4 threads, because it appends faces in thread-completion order, and its tile paths contain `todo!()`;
  * weld the voxel output, which is an unwelded triangle soup;
  * guard decimation's face floor. `min_faces_count` is ignored: a 1,280-face sphere decimated to 2 faces with no error;
  * derive the decimation threshold from the bbox diagonal. It is an absolute model-units-squared quantity with a cliff (0.01 removed nothing; 0.1 left 204 faces);
  * expect no step or yield API. Remeshing to 9k faces takes about 105 ms natively in one call.

### Confirmation

* `cargo deny check bans` and `scripts/check-boundary.sh` rule 1 reject `baby_shark` 0.3.12 in any engine crate. While this decision stands, CI is the check.
* Re-adoption needs a crates.io release where `rayon`, `wasm-bindgen`, and `getrandom` can be turned off. To check one, bump the pin in `research/s1_baby_shark_wasm/Cargo.toml`, add `default-features = false`, and run its five native and wasm32 tests. All must pass byte-identical before an engine crate adds the dependency.
* 001 T080's tests and T002's measurements run on unremeshed input. If either shows the raw mesh can't meet 001 T080's convergence or SC-005's budget, this ADR is revisited.

## Pros and cons of the options

### 1. Adopt as published

* Good, because it is the least work and covers all three jobs.
* Bad, because it fails `cargo deny` bans (`rayon`) and boundary rule 1 (`wasm-bindgen`, `js-sys`, `getrandom`, `rayon`), and puts about 1,400 foreign exports on the WASM surface.
* Bad, because dual contouring is nondeterministic natively, and decimation ignores its face floor.

### 2. Native-only remeshing

* Good, because the browser bundle stays clean.
* Bad, because it still fails both gates, since the gates resolve every target.
* Bad, because native and WASM would produce different patterns from the same input, breaking Constitution V and II.

### 3. Don't adopt now; flatten the repaired mesh; upstream opt-out features (chosen)

* Good, because it is gate-clean and deterministic in both targets with no new dependency.
* Good, because 001 T001 showed the upstream change is small and output-preserving.
* Bad, because 001 T080 loses the conditioning guarantee, and SC-005 loses its proxy dial until a remesher exists.
* Bad, because adoption depends on an upstream release we don't control [speculative on timing].

### 4. Fork or vendor a patched crate

* Good, because it is available now, and 001 T001's patch already passes the gates.
* Bad, because it adds a new git source (`allow-git`) and a fork to rebase, and it inherits the defects listed under Consequences.

### 5. Own remesher in `openplush-mesh`

* Good, because it can be f64, step-wise with yield points, deterministic by construction, and provenance-preserving (seams drawn on the input mesh map onto the proxy).
* Bad, because it is new geometry code to write and test, before any measurement shows it's needed.

## More information

* Evidence: [`research/s1_baby_shark_wasm/README.md`](https://github.com/openplush/openplush/blob/main/research/s1_baby_shark_wasm/README.md) and [`results.md`](https://github.com/openplush/openplush/blob/main/research/s1_baby_shark_wasm/results.md). These hold the exact commands, versions, hashes, the patch diff, and the size table.
* Follow-ups for the lead's docs PR: 001 D-007 status; the plan's Technical Context ("flattening runs on a remeshed proxy"); research R6 001 T001 result; tasks.md 001 T001 and T002 (the remeshed-proxy dial); data-model §3 last-resort row; plan.md Primary dependencies (`baby_shark` pending 001 T001) and Trade-offs ("remeshed flattening proxy"); research.md's `[verify]` note on rayon via baby_shark.
* Revisit when (a) `baby_shark` publishes opt-out features, (b) 001 T002 reports SLIM wall time on raw-size charts, or (c) fixture runs show 001 T080 convergence failures on poorly shaped input. If (b) or (c) arrives before (a), option 5 is the preferred fallback [speculative].
* Botsch, Kobbelt, "A Remeshing Approach to Multiresolution Modeling" (SGP 2004), the algorithm `IncrementalRemesher` implements.
