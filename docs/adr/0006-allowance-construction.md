---
status: Proposed
date: 2026-10-15
deciders: maintainer (proposed by the implementing agent from spike S3, T003)
---

# Build allowances as one `i_overlay` boolean of per-edge strips on the `i64` engine, validated before and after

## Context and problem statement

Seam allowance differs along one outline: a seam default, per-seam overrides, a separate opening allowance, hem allowance on unsewn edges, and zero on folds and on edge-stitched or appliqué edges (FR-030, FR-057). A single-distance polygon offset can't express that, so [data-model §7](../../specs/001-pattern-pipeline/data-model.md#7-seam-allowance) builds the cut line as a union of per-edge strips with `i_overlay`. Research R2 recorded an open outline-offset panic upstream (iOverlay issue 74) and asked for fuzzing.

T003 (spike S3) asks how robust that is, and so how defensive T085 (`openplush-pattern/src/allowance.rs`) must be. Evidence: [`research/s3_i_overlay_fuzz/`](../../research/s3_i_overlay_fuzz/README.md).

## Decision drivers

* Constitution III: a violated invariant is a typed error, never quietly wrong output. The post-pattern invariant is "cut outline is a simple polygon".
* Constitution V: byte-identical output, native and WASM.
* Constitution IX and `deny.toml`: allowlisted licenses; no `rayon`.
* `crates/AGENTS.md`: no panics on user-derived data, and panics never cross into JS.
* Physical tolerance: allowance is measured in mm. A plush maker cuts to about 0.5 mm, so anything below 1e-3 mm is invisible.

## Considered options

1. `i_overlay`'s built-in `outline` offset, one distance per piece.
2. `i_overlay`'s `variable_stroke` around the outline, unioned with the outline.
3. Per-edge strips and corner wedges, unioned with `i_overlay` on the default `i32` engine.
4. As 3, plus a fold guard over each zero-allowance edge, as **one** `i_overlay` call on the `i64` engine, with input and output validation.
5. Our own offset-curve construction (straight skeleton / weighted offset), no boolean library.

## Decision outcome

Chosen option: **4**. It is the only option that expresses per-edge allowances and stayed exact against an independent oracle on 10,000 near-degenerate cases.

What S3 found (10,000 seeded cases, near-degenerate by design):

* **No panics** on valid input, in release or with debug assertions and overflow checks on (byte-identical output in both). None on six invalid inputs either, including upstream's regression contour [solid for this corpus]. Issue 74 is very likely the outline `debug_assert!` that upstream's PR #75 removed on 2026-05-18, which ships in 9.0.1 [likely; the issue page was not reachable].
* **No wrong booleans**. The strip construction never lost or gained area against an independent membership oracle, on either engine [solid for this corpus].
* **Invalid input gives quietly wrong output, without a panic.** A clockwise or bow-tie outline gives 4–5 islands from the strips, and an empty result from `outline` [solid].
* **Precision.** `i32` falls short by up to 1.6e-6 mm at plush scale and 1.4e-3 mm on 10 m to 1 km pieces. An edge shorter than the integer grid loses its strip entirely, up to 6.9 mm short. `i64` after merging sub-1e-6-diagonal edges: 1.3e-10 mm worst case [solid].
* **Two calls leave slivers**. Each `FloatOverlay` picks its own integer grid from its inputs' bounding box. Chaining two booleans left sub-grid gaps along folds, which showed up as false holes, pinches and islands. One call removed them [solid].
* **Conflicts are common in a spiky random corpus.** Allowances that close a pocket, a fold beside a sharp reflex corner, a guard that cuts off part of a neighbour's strip, and regions joined at a vertex. `i_overlay` computed every one correctly: the requested geometry has no simple-polygon answer [solid]. Their rate on real pieces is unmeasured [verify, with fixtures].
* **Determinism**: all four methods were byte-identical, native and wasm32, on 200 cases. The only divergence came from the harness calling `std`'s `sin`, `cos` and `powf`, which use the platform libm natively [solid].

### What T085 does

1. **Validate the input**, and return a typed error naming the piece, never a fix-up:
   * the outline is simple and counter-clockwise, every coordinate finite, at least 3 vertices;
   * one allowance per edge, finite and ≥ 0;
   * zero only where FR-057 allows it.

   The flatten stage's injectivity invariant should already guarantee the first condition; T085 re-checks it with `openplush-geom`'s exact predicates because a panic or a quiet failure there is the expensive one [likely].
2. **Merge edges shorter than ε**, where ε = 1e-6 × the piece's bbox diagonal (relative, per data-model §11). The merged edge keeps the following edge's allowance. Re-check simplicity after the merge, and return a typed error if it fails. Without the merge, a sub-grid edge silently loses its allowance [solid].
3. **Construct the pieces in f64 with `+ − × ÷ sqrt` only**. No `sin`, `cos`, `powf`, `powi` or `hypot` on the output path; any that become necessary go through `libm`.
   * A rectangle on the outer side of each edge with allowance > 0.
   * A wedge at each convex corner: a **miter** to the point where the two offset lines meet. Fall back to a **bevel** when that point lies more than 4 × the larger allowance from the vertex, or behind either offset's start. With unequal allowances it can, and the quad would be a bowtie.
   * At reflex corners, nothing: the strips overlap.
4. **Fold guards.** Over each zero-allowance edge, a rectangle on its outer side, as deep as any piece reaches (4 × the largest allowance). No fabric may lie past a fold or an edge-stitched edge [solid as geometry; the sewing rule is the data-model's].
5. **One boolean**: `FloatOverlay::<_, i64>::from_subj_and_clip(outline + pieces, guards).overlay(Difference, NonZero)`. Never chain two overlays on one piece. Keep the `allow_multithreading` feature off: it is the only path to `rayon` [solid].
6. **Post-validate, and map each failure to a typed error carrying the piece ID and, where known, the edge IDs.** Proposed variants, named in T085:
   * result empty, or more than one outer contour → `AllowanceSplitsPiece`;
   * any hole → `AllowanceEnclosesGap`;
   * a contour that revisits a vertex or crosses itself → `AllowanceNotSimple`;
   * a fold guard that reaches into the outline → `FoldConflict`.

   This checks the output with the engine's own exact predicates. It is O(n²) on cut-line vertices at most, and a sweep if profiling asks for one.
7. **Catch panics at the stage boundary** anyway, and convert them to `AllowanceFailed { piece }`. The fuzz found none, but the library has `panic!` and `expect` on internal paths (the cross solver, outline section normalisation) [solid that they exist; speculative that real input reaches them].
8. **Keep the fuzz**. Port the oracle and a 1,000-case seeded subset into `openplush-pattern`'s property tests (`crates/AGENTS.md`: fixed seeds; failing seeds become regression cases).

### Consequences

* Good: per-edge allowances, folds and openings in one construction, exact to about 1e-10 mm, deterministic across targets.
* Good: every conflict in the requested allowance surfaces as a typed error with an ID the UI can highlight, instead of a quietly wrong cut line (Constitution III).
* Good: the dependency is small and gate-clean: `i_overlay` 9.0.1 (MIT OR Apache-2.0), plus `i_float`, `i_shape`, `i_tree`, `i_key_sort` and `libm`, all MIT. No `rayon`, `getrandom` or `wasm-bindgen` on the normal graph for either target.
* Bad: T085 carries validation, merging and post-validation code around one library call, about as much code as the construction itself.
* Bad: the corner rule changes. MVP joins are still miters with a limit, as data-model §7 says, but at a convex corner beside a fold the miter now runs out to the fold line, and corners sharper than about 29° are beveled. Data-model §7's "simplify input polygons first" becomes the short-edge merge above.
* Bad: how often real plush pieces hit a conflict error is unknown. If fixtures show it is common, T085 needs a remedy (a warning plus filling holes, or trimming strips at a sharp reflex corner beside a fold) rather than a hard error [speculative].

### Confirmation

* T085's tests port the S3 oracle (independent membership plus exact topology checks) as seeded property tests, on native and wasm32.
* The determinism job compares native and WASM output on the allowance stage.
* `cargo deny` keeps `rayon` out; a review checklist item keeps `allow_multithreading` off.
* Re-run the spike before bumping `i_overlay`: change the pin in `research/s3_i_overlay_fuzz/Cargo.toml`, update `CORPUS_200` only if the output legitimately changed, and run both profiles.

## Pros and cons of the options

### 1. Built-in `outline` offset

* Good, because it is one call, with joins built in.
* Bad, because it takes one distance per contour and cannot express per-edge allowances or folds.
* Bad, because its default integer construction falls short by up to 2.6e-4 mm at plush scale (5e-3 mm worst), and its float mode doesn't promise cross-platform reproducibility.
* Bad, because it returns an empty result for a clockwise or self-intersecting outline, with no error.

### 2. `variable_stroke`

* Good, because it varies the width along a path.
* Bad, because width is interpolated per vertex and the joins are round only. A step change of allowance at a vertex, or a fold, can't be expressed.

### 3. Strips on the default `i32` engine, unvalidated

* Good, because it is the simplest form of data-model §7.
* Bad, because sub-grid edges silently lose their allowance, and conflicts come out as holes, islands or pinches with no error.

### 4. Strips + guards, one `i64` call, validated (chosen)

* Good, because it was exact on every fuzz case, with every conflict detectable.
* Bad, because of the validation and merge code around the call.

### 5. Own offset-curve construction

* Good, because there is no dependency, and it is f64 throughout.
* Bad, because it is new robust-geometry code. The union and conflict handling it would need is exactly what `i_overlay` already does correctly.

## More information

* Evidence: [`research/s3_i_overlay_fuzz/README.md`](../../research/s3_i_overlay_fuzz/README.md) and [`results.md`](../../research/s3_i_overlay_fuzz/results.md): commands, versions, counts per failure class, reproducers.
* Follow-ups for the lead's docs PR: research R2's `i_overlay` row (issue 74 status, "simplify first" becomes "merge short edges"); research R6 S3 result; tasks.md T003 (done) and T085 (defensiveness, error variants); data-model §7 "Corners" and "Robustness" bullets.
* Revisit when (a) fixture runs measure the conflict rate on real pieces, (b) `i_overlay` is bumped (9.1.0 is out; its changes are performance work in the tree solver [likely, from the commit log]), or (c) v0.2 adds reflected corners.
