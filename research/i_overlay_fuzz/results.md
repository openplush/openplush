# 001 T003 results (2026-10-15)

## Versions

| Tool / crate | Version |
|---|---|
| rustc / cargo | 1.97.0 (2d8144b78 2026-07-07) / 1.97.0 (c980f4866 2026-06-30) |
| `i_overlay` | **9.0.1**, MIT OR Apache-2.0. This is the version 001 T006 audited. 9.1.0 is on crates.io, but its changes since 9.0.1 are tree-solver performance work (commit log), so nothing required the bump |
| `i_float` / `i_shape` / `i_tree` / `i_key_sort` / `libm` | 5.0.0 / 5.0.0 / 0.19.1 / 0.11.0 / 0.2.16, all MIT |
| `robust` | 1.2.0 (oracle only; the root workspace's version) |
| `wasm-bindgen` / `wasm-bindgen-test` / runner | 0.2.129 / 0.3.79 / 0.2.129 (same as the root `Cargo.lock`) |
| Node | v22.22.0 |
| cargo-deny | 0.20.2, against the repo's `deny.toml` |

Corpus: seed `0x00573F0020261015` (24557592745742357), 10,000 cases, six families of about 1,667 cases each. Scales: 8,000 plush, 1,000 tiny and 1,000 huge, approximately (drawn per case).

## 1. Dependency inventory and gates

```
$ cargo tree --target wasm32-unknown-unknown -e normal     # identical on the host
├── i_overlay v9.0.1
│   ├── i_float v5.0.0
│   │   └── libm v0.2.16
│   ├── i_key_sort v0.11.0
│   ├── i_shape v5.0.0
│   └── i_tree v0.19.1
└── robust v1.2.0     (+ libm, harness only)

$ cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans
bans ok, licenses ok
```

- **`rayon` is optional**, behind `i_overlay`'s and `i_key_sort`'s `allow_multithreading` feature (off by default). That feature is also the only path to `std::thread::available_parallelism`. The engine must never turn it on.
- **No `getrandom`, `wasm-bindgen`, `js-sys`, clock, or `HashMap`/`HashSet`** in the `i_*` sources.
- **Transcendentals go through `libm`** in `i_float`, which is pure Rust, so the result is the same on every target.
- The `i_*` crates contain `unsafe` (`i_overlay` 52 sites, `i_tree` 19, `i_key_sort` 21 by grep). Our crates' `forbid(unsafe_code)` does not reach dependencies.
- **Panic sites on non-test paths** include:
  - `panic!("Fail cross result")` ×11 in `split/cross_solver.rs`;
  - `panic!("no move found")` in `string/extract.rs`;
  - `unreachable!()` in `mesh/int/join.rs`;
  - `expect("unique segment")` / `expect("unique section")` in outline and stroke section normalisation;
  - `expect("Invalid offset bounds")` in the float offset entry points;
  - `FloatPointAdapter::new` panics on non-finite or out-of-range bounds.

  None fired in this run.

**Issue 74** [likely]. The issue page was not reachable from this session: the GitHub API needs repository access, and the web page was blocked. The upstream git history has `fix/outline` (PR #75, merged 2026-05-18), whose commit "fix degenerate rotation" replaces `debug_assert!(cross != 0, "not possible! UniqueSegmentsIter guarantee it")` in the outline builder and adds the regression test `test_real_case_1_offset`. That test's contour has an exact 180° reversal. Issue 74 directly precedes PR #75, and R2 describes it as an outline-offset panic, so this is very likely the fix. 9.0.1 contains it.

## 2. The 10,000-case run

```
$ cargo run --release            # 63 s
$ cargo run --profile checked    # 66 s, debug-assertions + overflow-checks
```

The two outputs are **byte-identical**, hashes included. No `debug_assert!` fired and no integer overflow occurred.

### Library faults (what Constitution III is about)

| Method | Panic | Missing | Extra | NonSimple | FoldCovered |
|---|---|---|---|---|---|
| Strips, `i32` | 0 | 0 | 0 | 0 | 0 |
| Strips, `i64` | 0 | 0 | 0 | 0 | 0 |
| Strips, `i64`, short edges merged | 0 | 0 | 0 | 0 | 0 |
| `outline`, `i32`, one distance | 0 | 1,085 (precision, see below) | 0 | 0 | n/a |

### Allowance shortfall (mm)

Worst shortfall, and cases short by more than 1e-6 / 1e-4 / 1e-2 mm. "Long" means edges at least 1e-6 × the result's diagonal.

| Method | Scale | All edges | Long edges |
|---|---|---|---|
| Strips `i32` | plush | **6.85** (63 / 5 / 5) | 1.6e-6 (58 / 0 / 0) |
| Strips `i32` | tiny | 6.89 (18 / 18 / 18) | 9.5e-8 (0) |
| Strips `i32` | huge | 1.4e-3 (1,009 / 769 / 0) | same |
| Strips `i64` | plush / tiny / huge | 1.9e-11 / **5.84** (4 cases) / 1.3e-10 | 1.9e-11 / 1.8e-11 / 1.3e-10 |
| **Strips `i64` merged** | plush / tiny / huge | **2.4e-11 / 1.8e-11 / 1.3e-10** | same |
| `outline` `i32` | plush | 10.2 (7,953 / 7,656 / 3) | 2.6e-4 (7,953 / 7,656 / 0) |
| `outline` `i32` | tiny / huge | 10.7 / 5.7e-3 | 5.2e-3 / 2.8e-3 |

Reading it:

- Every shortfall above 1e-2 mm is on an edge shorter than the integer grid (all-edges column only). Merging those edges removes all of them.
- What remains is the grid: about diagonal / 2²⁹ for `i32`, which is 1.6e-6 mm on a 1 m piece. On `i64` it is far below float noise.
- `outline`'s integer construction (`MathMode::Integer`, fixed-point directions) is about 1e-4 relative. That is why it shows 1,085 Missing at the 1e-6 × diagonal sampling step: the offset is short by up to 2.6e-4 mm at plush scale.

### Conflicts in the requested allowance (the reference agrees: real geometry)

| Fault | Strips `i32` | Strips `i64` | `i64` merged | of which guards clear (merged) |
|---|---|---|---|---|
| GuardHitsOutline (fold faces the outline within reach) | 5,596 | 5,596 | 5,385 | — |
| Disconnected (island) | 2,773 | 2,777 | 2,620 | 389 |
| HoleAtOutline | 568 | 577 | 458 | 10 |
| HoleDetached | 375 | 379 | 371 | 85 |
| Pinch (contour revisits a vertex) | 101 | 94 | 75 | 1 |

These are frequent because the corpus is adversarial: a zero allowance on a random 1 in 4 edges, needles, and 0.01 mm pieces with 10 mm allowances. They are not a prediction for real pieces. With the guards clear, the remaining islands are mostly tiny-scale cases (allowance ≫ piece), where a fold guard cuts a neighbour's strip in two.

### Invalid inputs (6 mm on every edge; none passes `is_simple_ccw`)

| Input | Strips `i32` | Strips `i64` | `outline` |
|---|---|---|---|
| Upstream regression contour (exact reversal) | 1 shape | 1 shape | 1 shape |
| Zero-width spike | 1 shape | 1 shape | 1 shape |
| Repeated vertex | 1 shape | 1 shape | 1 shape |
| Clockwise | **5 shapes** | **5 shapes** | **empty** |
| Bow-tie | **4 shapes** | **4 shapes** | **empty** |
| Collinear only | 1 shape | 1 shape | **empty** |

No panic, but **wrong orientation or self-intersection gives quietly wrong output**: islands for the strips, and an empty result for `outline`. Input validation is therefore required, not optional.

## 3. Construction defects the fuzz found in the spike's own code (and fixed)

Each one is a trap 001 T085 could fall into too:

1. **A zero allowance makes a wedge vertex coincide with the corner.** A repeated vertex breaks a convexity test. Fix: dedup the vertices.
2. **With unequal allowances, the miter point can lie behind an offset's start**, and the wedge becomes a bowtie. The first run showed it as a detached island. Fix: miter only if the point is ahead of the incoming offset end and behind the outgoing offset start; otherwise bevel.
3. **At a reflex corner sharper than 90° beside a fold**, the neighbour's rectangle reaches over the fold's outer side. FoldCovered fired in 55% of a 2,000-case trial. Two local clips were tried and were wrong: one deleted whole strips, the other cut strips along the entire neighbouring edge, up to 12 mm short. Fix: a guard rectangle over each zero-allowance edge, subtracted from the pieces.
4. **Chaining booleans**, `outline ∪ (pieces − guards)`, uses two integer grids. On the same 2,000 cases, `i64` gave 1,306 cases with holes at the outline and 577 non-simple, against 108 and 21 in one call. Fix: one call, `(outline ∪ pieces) − guards`.

## 4. Determinism

```
$ cargo test --release                                   # 8 passed
$ cargo test --release --target wasm32-unknown-unknown   # 8 passed under Node
```

`first_200_cases_match_native` asserts `corpus_hash(SEED, 200) = 17622220365183590366` on both targets. That hash covers all four methods' output coordinates, bit for bit. In the native binary the first 1,000 cases hash to `17680898648509451964` twice in one process.

The first wasm32 run failed, and the cause was the harness. Native printed 11423592118044362244 and wasm32 printed 17622220365183590366. The generator used `f64::sin`, `cos`, `tan` and `powf` (platform libm natively, Rust's libm on wasm32) and `powi` (an LLVM intrinsic). After switching to the `libm` crate and `x * x`, native printed wasm32's value. So `i_overlay` was already deterministic across targets, and the inputs had differed.

## 5. Reproducers (smallest per class, `i64` merged; per-edge allowances in mm)

Run `cargo run --release --example inspect -- <case>` to print pieces, guards, the result and its self-contacts.

| Class | Case | Outline | Allowances |
|---|---|---|---|
| Disconnected | 3528 | `[[0.04178929842006819, 0.07684295574195156], [0.022901584731920555, 0.10294808402995731], [0.018276963889163875, 0.14020562659406316], [-0.12496139953865767, 0.10664267705007308]]` | `[10, 0, 10.858002336013204, 0]` |
| HoleAtOutline | 7472 | `[[297.0790310583748, 110.65418083305688], [-181.3261083135136, 281.0628765797204], [-181.42688698497963, 280.91525327443026], [-246.31210926486926, 210.8703094299872], [-238.81312063838067, -198.00519233684832], [-238.81303312593434, -198.00499567658906], [237.60213035507203, -222.35337066496336], [237.59917675041467, -222.3530967675315]]` | `[0, 3, 10, 5.252059308658957, 2.6523726799860405, 6, 0, 11.349250131790605]` |
| HoleDetached | 5964 | `[[8.841513581623117, 4.647660607126253], [0.9658972103063955, 10.459153656691045], [-2.2278343738632187, 7.641745676094785], [-7.827703196778129, 3.790028338332952], [-8.434501403108097, -2.339517759096847], [1.5929565443891207, -15.370464306425722], [6.879311779409597, -7.047083971260041], [8.831812208382566, -3.3843901322246066], [13.191168450937841, -2.683560788474874]]` | `[3, 0, 6, 6, 6, 10, 3, 6, 3]` |
| Pinch | 4994 | `[[93458.223192486, -57051.85947802505], [93446.11587905101, -57048.097834279986], [93419.66882671909, -57077.15939719139], [93435.73139625762, -57102.22464893883], [93435.72836942697, -57102.226270899526], [93461.81887303924, -57095.18824092918]]` | `[6, 6, 6, 0, 0, 3.3438095006427333]` |
| GuardHitsOutline | 2609 | `[[-156.9432657596817, 0.008831503350547743], [-143.5433519984813, -0.0005932664587484942], [-138.43987958537315, -0.007926181090352794], [-148.45627021880796, -0.014790137381280422], [-69.08107629208887, -0.023919136473202134]]` | `[10, 3, 0, 10, 8.303073414168422]` |
| `outline` Missing (precision) | 179 | `[[0.04933288707282389, 2.5819147255606178e-5], [-0.21545269321042526, 2.2975138312092458e-5], [-0.09513225441181597, 4.529468428117112e-6]]` | offset 7.670365513717354 |
| Sub-grid edge loses its strip (`i32`, before merge) | 3932 (6.85 mm short; the long-edge worst in the same run is 1.6e-6 mm) | see `inspect -- 3932` | — |

## 6. What was not done

- **The fixture corpus.** The fixtures are not yet flattened into pieces, so how often real plush outlines hit a conflict class is unmeasured.
- **`i_overlay` 9.1.0** was not fuzzed.
- **The `OverlayOptions::ogc()` output mode**, which splits pinches, was not evaluated.
- **`MathMode::Float`** was not fuzzed: its documentation disclaims cross-platform reproducibility, which rules it out.
- **wasm32 was not run on all 10,000 cases.** It ran the first 200 (about 3 s); the full corpus would take minutes and add no new code paths beyond the native run.
