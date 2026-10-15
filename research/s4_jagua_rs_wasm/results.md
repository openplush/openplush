# S4 results (2026-10-15)

## Versions

| Tool / crate | Version |
|---|---|
| rustc / cargo | 1.97.0 (2d8144b78 2026-07-07) / 1.97.0 (c980f4866 2026-06-30) |
| `jagua-rs` | 1.1.0 (latest on crates.io on 2026-10-15), MPL-2.0, `rust-version` 1.90 |
| `sparrow` (JeroenGar) | git `1fe3e31` (2026-10-13; "Port sparrow to jagua-rs 1.1.0" is `4629b22`), package version 0.3.0, MIT in `LICENSE` only |
| `wasm-bindgen` / `wasm-bindgen-test` / runner and CLI | 0.2.129 / 0.3.79 / 0.2.129 (same as the root `Cargo.lock`) |
| Node | v22.22.0 |
| cargo-deny | 0.20.2, against the repo's `deny.toml` |
| `wasm-opt`, `twiggy`, `wasm-tools` | **not installed**; not installed for this spike |
| Host | Linux x86_64, 4 cores |

The input is six simple polygons in millimetres: a 60 × 40 rectangle, two right triangles (70 × 50), two trapezoids (80 at the base, 50 at the top, 40 tall), and an L (50 × 60, arms 20 wide). The strip is 150 mm wide (fabric width) and 1,000 mm long. The candidate grid pitch is 2 mm.

## 1. Dependency inventory

`cargo info jagua-rs`: 1.1.0, MPL-2.0. Features: `bpp = []`, `spp = []`, no `default`. Dependencies from the published manifest, **none optional**:

```toml
anyhow = "1.0"            document-features = "0.2"   float-cmp = "0.10"
geo = { version = "0.33", default-features = false }   geo-types = "0.7"
getrandom = { version = "0.4", default-features = false, features = ["wasm_js"] }
itertools = "0.15"        log = "0.4"                 ndarray = "0.17"
ordered-float = "5.0"     rand_distr = "0.6"          rayon = "1.11"
serde = { version = "1.0", features = ["derive"] }     slotmap = "1.0"
svg = "0.18"              thiserror = "2"             web-time = "1.1"
```

`cargo tree --target wasm32-unknown-unknown -e normal` (abridged to the crates that matter):

```
jagua-rs v1.1.0
├── geo v0.33.1 ── i_overlay v4.5.2, rand v0.10.3, rand_pcg v0.10.2, robust v1.2.0, rstar v0.12.2
├── getrandom v0.4.3 ── wasm-bindgen v0.2.129
├── rayon v1.12.0 ── rayon-core v1.13.0 ── crossbeam-*
└── web-time v1.1.0 ── js-sys v0.3.106, wasm-bindgen v0.2.129
```

A grep of the source (8,737 lines) finds:

- **rayon**: only `probs/spp/io/import.rs` and `probs/bpp/io/import.rs` (`into_par_iter` / `par_iter` while importing an instance). Both are behind the `spp`/`bpp` features; the dependency is not.
- **web-time**: only `pub type Instant = web_time::Instant` in `lib.rs`, used by the `spp`/`bpp` problem and solution types (`Instant::now()` time stamps).
- **getrandom**: never referenced in the source. It is there only to turn on `wasm_js`.
- **rand_distr**: used only to reach `num_traits` re-exports (`ToPrimitive`, `FloatConst`). No sampling in the core.
- **Hash containers**: `HashSet` only in `util/assertions.rs` (debug assertions). Containers are `SlotMap`s.
- **Panics a wrapper must guard**: `DTransformation::new` calls `expect` on NaN rotation or translation; `Strip::set_width` asserts a positive width. Imports return `anyhow::Result` and check finiteness, so validated input never reaches those.
- **Precision**: `f32` everywhere (`Point(pub f32, pub f32)`, `DTransformation { rotation: NotNan<f32>, translation: (NotNan<f32>, NotNan<f32>) }`).

## 2. Policy gates (published crate)

```
$ cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans sources advisories
error[banned]: crate 'rayon = 1.12.0' is explicitly banned
error[banned]: crate 'rayon-core = 1.13.0' is explicitly banned
advisories ok, bans FAILED, licenses ok, sources ok
```

`scripts/check-boundary.sh` rule 1 fails an engine crate that reaches `wasm-bindgen`, `web-sys`, `js-sys`, `rayon`, or `getrandom` through normal dependencies. The tree above reaches `rayon`, `getrandom`, `wasm-bindgen`, and `js-sys`, with no features enabled.

## 3. Rotation sets: the API

- **Input**: `jagua_rs::io::ext_repr::ExtOrientation { rotation: ExtRotation, reflection_axes: Vec<f32> }`, in degrees. `ExtRotation` is `Discrete { angles }`, `Stepped { step }`, or `Continuous {}`.
- **Imported**: `jagua_rs::geometry::AllowedOrientations`, which holds `rotations: RotationRange` and `rotations_after_reflection: Option<RotationRange>`, in radians. `RotationRange` (`jagua_rs::geometry::geo_enums`) is `None | Continuous | Discrete(Vec<f32>)`. Angles are normalised to [0, 2π), sorted with `total_cmp`, and deduplicated.
- **Check**: `AllowedOrientations::allows(&DTransformation) -> bool`, with a tolerance of 4 `f32` epsilons of a full turn.
- **Not checked**: `Layout::place_item` places whatever pose it is given, as its own crate docs say ("Placement itself does not validate collisions or orientation permissions").

What the tests assert (`tests/cross_target.rs`):

| Input | `rotations()` | `rotations_after_reflection()` | `allows` |
|---|---|---|---|
| {0°}, no axes | `None` | `None` (no mirroring) | 0 yes, π no |
| {0°, 180°}, no axes | `Discrete([0.0, π])` | `None` | π yes, π/2 no |
| {0°}, axis 90° (nap-parallel mirror) | `None` | `Discrete([π])` | reflected + π yes; reflected + 0 no |

Data-model §6's other cases follow from the same type [likely, from the source; not run]. Bias {45°, 225°} is a plain `Discrete` list. The napped "±3° tolerance" has no range form. It would be a discrete list (for example −3° to 3° in 1° steps) or a stricter {0°}.

## 4. Build, run, determinism

```
$ cargo run --release
Fixed: length 186 mm, hash 1852278804236050536  5.42ms
  Placement { id: 3, rotation: 0.0, translation: (39.692307, 20.461538) }
  Placement { id: 4, rotation: 0.0, translation: (105.69231, 50.46154) }
  Placement { id: 0, rotation: 0.0, translation: (32.0, 64.0) }
  Placement { id: 5, rotation: 0.0, translation: (82.333336, 97.333336) }
  Placement { id: 1, rotation: 0.0, translation: (25.333334, 152.66667) }
  Placement { id: 2, rotation: 0.0, translation: (97.333336, 152.66667) }
HalfTurn: length 156 mm, hash 17964957110741447323  7.10ms
  Placement { id: 3, rotation: 0.0, translation: (39.692307, 20.461538) }
  Placement { id: 4, rotation: 3.1415927, translation: (106.30769, 23.538464) }
  Placement { id: 0, rotation: 0.0, translation: (32.0, 64.0) }
  Placement { id: 5, rotation: 3.1415927, translation: (93.66667, 80.666664) }
  Placement { id: 1, rotation: 3.1415927, translation: (122.666664, 113.33333) }
  Placement { id: 2, rotation: 0.0, translation: (25.333334, 122.666664) }
```

Translations are of jagua's centred shape (import moves each outline's centroid to the origin).

| Run | Result |
|---|---|
| `cargo test --release` (native) | 5 passed |
| `cargo test` (native debug) | 5 passed, same hashes |
| `cargo test --release --target wasm32-unknown-unknown` | 5 passed (0.05 s) |
| `cargo test --target wasm32-unknown-unknown` (debug) | 5 passed (1.02 s) |
| Native binary, 20 separate processes | 20/20 identical for both sets |

## 5. WASM size

`probe/measure.sh`. Profile: `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`. "bindgen" is the `_bg.wasm` after `wasm-bindgen --target nodejs`, which is what ships. Each row also ran the export under Node: the baseline returned `0 1` and the jagua build `186 156`, matching native.

| Build | opt-level | cargo `.wasm` | after wasm-bindgen | gzip -9 | JS glue |
|---|---|---|---|---|---|
| Baseline (empty export) | z | 27,481 | 12,705 | 5,774 | 740 |
| jagua-rs 1.1.0 + greedy placer | z | 724,738 | **376,864** | **140,776** | 1,474 |
| Baseline | s | 27,655 | 12,705 | 5,774 | 740 |
| jagua-rs 1.1.0 + greedy placer | s | 781,078 | 424,576 | 158,550 | 1,474 |
| Patched jagua-rs (section 6) + placer | z | 409,492 | 374,278 | 139,414 | 742 |
| Patched jagua-rs + placer | s | 454,429 | 422,111 | 157,779 | 742 |

Delta over the baseline at `z`: **+364 KB raw, +135 KB gzip**. The published build's extra JS glue is one import (`__wbindgen_throw`). The patched build needs none.

Where the bytes go: an unstripped `z` build has 431 KB of code in 2,110 functions, attributed by the first crate in each v0 symbol. This is approximate, because generic code is credited to the crate that defines the generic.

| Crate(s) | Code bytes | Share |
|---|---|---|
| `core` (monomorphised iterators, sorts, formatting, float printing) | 167,120 | 38.8% |
| `jagua_rs` | 41,224 | 9.6% |
| `i_overlay` 4.5, `i_key_sort`, `i_tree`, `i_shape`, `geo` | 68,365 | 15.9% |
| `rustc_demangle`, `anyhow`, `std` (error and panic formatting) | 30,452 | 7.1% |
| `js_sys`, `wasm_bindgen` (removed in part by wasm-bindgen) | 23,636 | 5.5% |
| `alloc`, `dlmalloc`, `compiler_builtins`, `libm` | 35,359 | 8.2% |
| the exported `nest_length` (harness inlined into it) | 10,960 | 2.5% |

`geo`'s `buffer_with_style` and the `i_overlay` 4.5 solvers come in through `OriginalShape::convert_to_internal` → `shape_modification`, which handles offset and simplification. They are linked even with both off, because that is a runtime branch. The engine will already ship `i_overlay` 9 for seam allowance (R2), so nesting would bring a second major version of it.

## 6. Patch experiment (scratch, not committed)

A scratch copy of jagua-rs 1.1.0, swapped in with `[patch.crates-io]` in a scratch copy of this harness and probe:

```diff
 [features]
-bpp = []
-spp = []
+bpp = ["dep:rayon", "dep:web-time"]
+spp = ["dep:rayon", "dep:web-time"]
-[dependencies.getrandom]
-version = "0.4"
-features = ["wasm_js"]
-default-features = false
 [dependencies.rayon]
 version = "1.11"
+optional = true
 [dependencies.web-time]
 version = "1.1"
+optional = true
--- src/lib.rs
+#[cfg(any(feature = "spp", feature = "bpp"))]
 pub type Instant = web_time::Instant;
```

- `cargo tree --target wasm32-unknown-unknown -e normal`: no `rayon`, `rayon-core`, `getrandom`, `wasm-bindgen`, `js-sys`, or `web-time`. `rand` 0.10 and `rand_pcg` remain via `geo`, with no OS entropy source.
- `cargo deny … check licenses bans` → `bans ok, licenses ok`.
- `cargo test --release`, and `cargo test --release --target wasm32-unknown-unknown` (toolchain 1.97.0, same runner) → 5 passed each, **same hashes**.
- Size: see section 5. The patch changes the gates, not the size.

`sparrow` enables `spp`, so it would still bring `rayon` through this patch. That is correct: sparrow's own `Cargo.toml` depends on `rayon` anyway.

## 7. sparrow

**Manifest** (`1fe3e31`): lib + two bins (`sparrow`, `bench`). No `license` field. Non-optional dependencies include `jagua-rs` (`spp`), `rayon`, `clap`, `fern`, `num_cpus`, `jiff`, `numfmt`, `serde_json`, `tap`, `test-case`, and `getrandom` (`wasm_js`). `ctrlc` is gated off wasm32. Features: `live_svg`, `only_final_svg`, `simd` (nightly `portable_simd`), `tui`.

```
$ cargo build --release --lib --target wasm32-unknown-unknown --manifest-path ext/sparrow/Cargo.toml
    Finished `release` profile [optimized] target(s) in 36.94s

$ cargo deny --manifest-path ext/sparrow/Cargo.toml --config …/deny.toml check licenses bans
error[banned]: crate 'rayon = 1.12.0' is explicitly banned
error[banned]: crate 'rayon-core = 1.13.0' is explicitly banned
warning[no-license-field]: license expression was not specified in manifest for crate 'sparrow = 0.3.0'
bans FAILED, licenses ok
```

As a git dependency it would also fail `sources` (`unknown-git = "deny"`). As a non-root crate without a license field it would need a `[[licenses.clarify]]` entry, as S6 found.

**Source facts**:

- `Separator::new` builds a rayon `ThreadPool` natively and uses the global pool on wasm32 (`cfg!(target_arch = "wasm32")`). `move_items_multi` runs each worker with its own `Xoshiro256PlusPlus` seeded from the master RNG, then picks the best result.
- Termination is a `Terminator` trait (`kill`, `new_timeout`, `timeout_at`). `BasicTerminator` reads `jagua_rs::Instant` (`web-time`).
- The defaults (`DEFAULT_SPARROW_CONFIG`) are exploration `time_limit` 9 min, compression 1 min, `ShrinkDecayStrategy::TimeBased` (the shrink step is a function of elapsed time), `n_workers` 3, `rng_seed: None`.
- `EPOCH` (a `LazyLock<Instant>`) and the per-phase `Instant::now()` calls are used for logging, apart from `TimeBased`.

**Scratch probe** (not committed): a `cdylib` and `rlib` that depends on the clone by path. It imports the same six pieces into an `SPInstance` (strip `fixed_height` 150, initial width 400), sets `n_workers`, switches compression to `ShrinkDecayStrategy::FailureBased(0.95)`, and passes a `CountTerminator` whose `kill()` returns true after N polls per phase. The seed is 7.

| Run | {0°}, 50 polls | {0°, 180°}, 50 polls | {0°, 180°}, 400 polls |
|---|---|---|---|
| Native, 1 worker, 10 processes | 144.62904 (`0x4310a109`) ×10 | 144.67159 (`0x4310abed`) ×10 | |
| Native, 3 workers, 10 processes | 144.62904 ×10 | 144.67159 ×10 | 111.36969 (`0x42debd48`), 1.29 s |
| wasm32 (Node), 1 worker | 144.62904 | 144.67159 | |
| wasm32 (Node), 3 workers | 144.62904 (416 ms) | 144.67159 (161 ms) | 111.36969 |

Bit-identical native and wasm32, deterministic across processes and worker counts on this instance [solid for this instance; likely in general, since each worker has its own seeded RNG and the reduction picks by loss]. The used length is a strip length in mm; for comparison, the greedy placer used 186 and 156 mm.

Size, `opt-level = "z"`, same profile: cargo `.wasm` 944,296 bytes; after wasm-bindgen **565,248**; gzip -9 **200,133**. That is about +190 KB raw and +60 KB gzip over the jagua-only probe.

**Out of the box it is not deterministic across machines** [solid, from the source]. Both phases stop on wall-clock time, and `TimeBased` decay makes the search itself depend on elapsed time. Determinism needs a caller-supplied `Terminator` that counts work, plus `FailureBased` decay, as the probe does. Every call blocks until it terminates: no step API or yield point, only the `kill()` poll.
