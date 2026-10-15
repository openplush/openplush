# Spike S3: is `i_overlay` robust enough for per-edge seam allowances?

**Status (2026-10-15): closed.** Decision: [ADR 0006](../../docs/adr/0006-allowance-construction.md) (Proposed).

This is non-shipping research code. Nothing in `crates/`, `apps/`, or `packages/` imports it. It is a standalone Cargo project with its own `[workspace]` table and `Cargo.lock`, so the root workspace (`members = ["crates/*"]`) and `scripts/check.sh` never see it.

## The question

Task T003 asks whether `i_overlay` panics or returns wrong geometry when it builds seam allowances, and so how defensive the allowance construction (T085) must be. Research R2 picked `i_overlay` for "seam allowance offset + union" and recorded an open outline-offset panic upstream (iOverlay issue 74).

A single-distance polygon offset is not enough. Data-model §7 needs a different allowance on each edge (seam default, per-seam override, opening, hem, and 0 for a fold or an edge-stitched edge), so the cut region is a union of per-edge strips. The spike builds that union, fuzzes it, and checks every result against facts computed without `i_overlay`. Panics matter, but Constitution III cares more about output that is quietly wrong.

## Method

| Step | What | Why |
|---|---|---|
| Inventory | `cargo info`, `cargo tree` (host and wasm32), `cargo deny check licenses bans` against the repo's `deny.toml`; grep the `i_*` crates for panics, threads, clock, hash maps | License, boundary and determinism gates before any run |
| Construction | `src/allowance.rs`. For a simple CCW outline and an allowance per edge in mm: one rectangle per edge on its outer side; one wedge per convex corner (the two offset lines' miter point, beveled past a miter limit of 4); one *guard* rectangle over the outer side of each zero-allowance edge. Result: **(outline ∪ pieces) − guards in one `FloatOverlay` call**, NonZero fill | The data-model §7 construction, plus the fold rule the fuzz showed it needs |
| Comparison | `i_overlay`'s own `outline` offset (`OutlineStyle`, miter join at the same limit), with one distance per case | The library's built-in offset; it cannot take per-edge distances |
| Engines | Strips on the `i32` engine (the default) and the `i64` engine; strips on `i64` after merging edges shorter than 1e-6 × the diagonal; outline on `i32` | Separates grid precision from algorithm faults |
| Corpus | `src/generate.rs`, SplitMix64, seed `0x00573F0020261015`, 10,000 cases in six families: star-shaped (3–63 vertices), x-monotone (4–63), stars with split vertices (edges 1e-9–1e-3 of the size), stars with near-collinear inserted points (off the line by 1e-12–1e-4), needles of 0.05°–5° (outward and inward), and slivers squashed 1e-1–1e-4. Sizes: 80% plush (10–1000 mm), 10% tiny (0.01–1 mm), 10% huge (1e4–1e6 mm); 25% shifted up to 1e5 mm from the origin. Allowances per edge: 0, 3, 6, 6 or 10 mm, or one in five uniform on [0, 12] mm. Every outline is checked simple and CCW with exact predicates before use | Near-degenerate shapes at and beyond the supported scale range |
| Oracle | `src/oracle.rs`. Every piece and guard is convex, so membership in the reference region is a winding test plus half-plane tests, independent of `i_overlay`. Points within 1e-7 × the diagonal of any reference edge are skipped (rounding decides there). Sampled points: five stations on every edge, offset by ±1e-6 × the diagonal and at the allowance ± that step; one interior point per piece; the centroid of every three consecutive result vertices; 64 uniform points over both sides' bounding box | Catches lost area (**Missing**) and stray area (**Extra**) anywhere, including in the allowance band itself |
| Topology | Outer contours (more than one is **Disconnected**), holes (touching the outline, or not), contours that revisit a vertex (**Pinch**) or cross themselves (**NonSimple**), each with exact predicates; a zero-allowance edge whose outer side is inside the result (**FoldCovered**); a guard reaching into the outline (**GuardHitsOutline**) | Constitution III's post-pattern invariant: the cut outline is one simple polygon |
| Shortfall | Bisection along each edge's outward normal at its midpoint: how far short of its allowance the result's coverage ends, in mm. Reported for all edges and for edges at least 1e-6 × the diagonal long | A precision number in mm, not just a yes/no |
| Panics | `catch_unwind` around every call, with a hook that records the location; every run is also repeated in a `checked` profile (release optimisation, `debug-assertions` and `overflow-checks` on) | The pre-fix upstream outline bug was a `debug_assert!`; a release build would hide it, or wrap an overflowing multiplication silently |
| Invalid inputs | Six outlines the post-flatten invariant excludes, including upstream's regression contour for its outline fix | Shows what input validation must reject before the call |
| Determinism | Each method's output coordinates, FNV-1a hashed bit for bit. Two runs in one process; `tests/cross_target.rs` asserts the hash of the first 200 cases (all four methods) natively and as `#[wasm_bindgen_test]` under Node | Constitution V |

## Run it

From this directory, with the repo toolchain and `wasm-bindgen-test-runner` 0.2.129 (`scripts/check.sh --setup` installs it):

```sh
cargo run --release                                    # 10,000 cases: tables, reproducers, invalid inputs
cargo run --profile checked                            # the same with debug asserts and overflow checks
cargo test --release                                   # native oracle tests
cargo test --release --target wasm32-unknown-unknown   # same tests on wasm32 under Node
cargo run --release --example inspect -- 535           # one case: pieces, guards, result, self-contacts
cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans
```

`cargo run --release -- <cases> <seed>` runs another corpus.

## Findings

Full output and reproducers: [`results.md`](./results.md).

1. **No panics** [solid for this corpus]. 10,000 valid cases × 4 methods, in release and in the checked profile, and all six invalid inputs, including upstream's regression contour. The release and checked outputs are byte-identical. Issue 74 is very likely the `debug_assert!` that upstream's PR #75 (2026-05-18) replaced, which is in 9.0.1 [likely; the issue page was not reachable from this session].
2. **No quietly wrong booleans on valid input** [solid for this corpus]. The strip construction never lost or gained area against the independent reference on either engine: 0 Missing, 0 Extra, 0 NonSimple, 0 FoldCovered.
3. **Invalid input gives quietly wrong output** [solid]. A clockwise or bow-tie outline gives 4–5 islands from the strips and an empty result from `outline`, with no error. Validating the input is mandatory.
4. **Precision depends on the engine, and on sub-grid edges** [solid].
   - On `i32` (the default) the allowance falls short by up to 1.6e-6 mm at plush scale, and by up to 1.4e-3 mm on 1e4–1e6 mm pieces.
   - An edge shorter than the integer grid loses its strip completely: up to 6.9 mm short.
   - On `i64`, after merging edges shorter than 1e-6 × the diagonal, the worst shortfall over all 10,000 cases is 1.3e-10 mm.
5. **The built-in `outline` offset is the weaker option** [solid]. It takes one distance, so it cannot express per-edge allowances. Its default `MathMode::Integer` falls short of the requested offset by up to 2.6e-4 mm at plush scale (5e-3 mm worst). `MathMode::Float` is more accurate, but its documentation does not promise cross-platform bitwise reproducibility. `variable_stroke` interpolates width per vertex, round joins only, so it cannot express a step change of allowance at a vertex either.
6. **Two booleans in sequence leave slivers** [solid]. Each `FloatOverlay` builds its own float-to-integer grid from its inputs' bounding box. On a 2,000-case trial, running `pieces − guards` and then `outline ∪ result` left sub-grid gaps along folds: 1,306 cases with holes at the outline and 577 non-simple, against 108 and 21 for one call, `(outline ∪ pieces) − guards`.
7. **Most "faults" are real conflicts in the requested allowances, not library errors.** The reference agrees with `i_overlay` on every one of them [solid]. They are frequent in this deliberately spiky corpus:
   - a fold whose guard reaches into the outline, usually a reflex corner sharper than 90° beside it;
   - allowances that close a pocket over a notch (a hole);
   - a guard that cuts a neighbour's strip off (an island);
   - two regions meeting at one vertex (a pinch).

   How often they happen on real plush pieces is unmeasured [verify, with the fixture corpus].
8. **Cross-target determinism holds for `i_overlay`, but not for code that calls `std`'s float maths** [solid]. Every method's output for the first 200 cases is byte-identical on native and wasm32. The first wasm run disagreed, and the cause was this harness: the generator called `f64::sin`, `cos`, `tan` and `powf`, which come from the platform libm natively and from Rust's own libm on wasm32. Routing them through the `libm` crate, as `i_float` already does, made the hashes equal. The construction itself uses only `+ − × ÷ sqrt`, which IEEE 754 makes exact on both targets.
