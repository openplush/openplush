# 001 T001 results (2026-10-14)

## Versions

| Tool / crate | Version |
|---|---|
| rustc / cargo | 1.97.0 (2d8144b78 2026-07-07) / 1.97.0 (c980f4866 2026-06-30) |
| `baby_shark` | 0.3.12 (latest on crates.io on 2026-10-14), MIT |
| `rayon` / `rayon-core` | 1.12.0 / 1.13.0 (via `baby_shark`, `faer` 0.22.6, `gemm-common` 0.18.2) |
| `faer` | 0.22.6 (`baby_shark`'s own; default features) |
| `wasm-bindgen` / `wasm-bindgen-test` / runner | 0.2.129 / 0.3.79 / `wasm-bindgen-test-runner` 0.2.129 (same as the root `Cargo.lock`) |
| Node | v22.22.0 |
| cargo-deny | 0.20.2, against the repo's `deny.toml` |
| Host | Linux x86_64, 4 cores (`nproc`) |

The input is a unit icosphere at subdivision 3: 642 vertices and 1,280 faces, mean edge ≈ 0.095.

## 1. Dependency inventory

`cargo info baby_shark`: version 0.3.12, license MIT. The published manifest has **no `[features]` table**. Its dependencies:

```toml
[dependencies]
bitflags = "2.9.1"
faer = "0.22.4"          # default features: std, rayon, sparse-linalg, rand, npy
nalgebra = "0.34.0"
nalgebra-glm = "0.20.0"
num-traits = "0.2.19"
rayon = "1.8.1"          # not optional

[target.'cfg(target_arch ="wasm32")'.dependencies]
getrandom = { version = "0.2", features = ["js"] }   # not optional
wasm-bindgen = "0.2.100"                              # not optional
```

`src/lib.rs` has `#[cfg(target_arch = "wasm32")] mod wasm;`, which holds the crate's own `#[wasm_bindgen]` exports.

`cargo tree --target wasm32-unknown-unknown -e normal -i rayon-core`:

```
rayon-core v1.13.0
└── rayon v1.12.0
    ├── baby_shark v0.3.12
    ├── faer v0.22.6
    └── gemm-common v0.18.2
```

`getrandom` 0.2.17 arrives through `baby_shark` and through `rand_core` → `rand` 0.8.8 → `faer` and `num-complex`. `wasm-bindgen` arrives through `baby_shark` and `getrandom`.

A grep of the crate source finds:

- **rayon** is used in only three places, all on the voxel path: `voxel/mesh_to_volume.rs` (`par_iter` then `collect`, which keeps order), and `voxel/root_node/tree_node.rs` and `voxel/internal_node/tree_node.rs` (`par_bridge().into_par_iter().for_each`).
- **faer** is used only by `deform.rs`.
- **No** `std::thread`, `Instant`, `SystemTime`, or RNG calls. Filesystem access (`fs::File`) appears only in `io/mod.rs`, which the harness doesn't call.
- **Panics**: `todo!("Dual contouring, tile support")` ×3 and `unimplemented!()` in `voxel/meshing/dual_contouring.rs`; `unimplemented!` in the voxel tree nodes; `todo!` in `io/obj.rs` and `geometry/primitives/triangle3.rs`.

## 2. Policy gates (published crate)

```
$ cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses
licenses ok

$ cargo deny --manifest-path Cargo.toml --config ../../deny.toml check bans
error[banned]: crate 'rayon = 1.12.0' is explicitly banned
error[banned]: crate 'rayon-core = 1.13.0' is explicitly banned
bans FAILED
```

`scripts/check-boundary.sh` rule 1 fails any engine crate that reaches `wasm-bindgen`, `js-sys`, `rayon`, or `getrandom` through normal dependencies, with every feature enabled. `baby_shark` reaches all four, so it fails that rule no matter which crate in `crates/` would depend on it. A `cfg(not(target_arch = "wasm32"))` dependency doesn't help: `cargo metadata` and `cargo deny` resolve every target.

## 3. Build and run

```
$ cargo build --target wasm32-unknown-unknown --release
    Finished `release` profile [optimized] target(s) in 1m 32s

$ cargo test --release --target wasm32-unknown-unknown
     Running tests/cross_target.rs (target/wasm32-unknown-unknown/release/deps/cross_target-….wasm)
running 4 tests
test voxel_remesh_runs_and_matches_native ... ok
test isotropic_remesh_runs_and_matches_native ... ok
test decimation_runs_and_matches_native ... ok
test decimation_floor_is_ignored ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 filtered out; finished in 0.47s
```

After the dual-contouring test was added (final code): `test result: ok. 5 passed; 0 failed` on wasm32 release (0.63 s), native release, and native with `RAYON_NUM_THREADS=1`. The debug-profile wasm32 run also passes, in 61 s.

The voxel path calls `par_iter` and does not panic on wasm32. `rayon-core` 1.13's `default_global_registry` catches the "unsupported" error from thread spawning and builds a one-thread pool on the current thread.

## 4. Fingerprints (native release, `cargo run --release`)

```
input icosphere: 642 vertices, 1280 faces
remesh(0.05)            Fingerprint { vertices: 4548, faces: 9092, hash: 933930466446281728 }  104.166247ms
decimate(1e-1)          Fingerprint { vertices: 104, faces: 204, hash: 7995362684396460200 }  3.104208ms
decimate(1e-2)          Fingerprint { vertices: 642, faces: 1280, hash: 11710329126619424277 }  1.677466ms
decimate_to_floor(320)  Fingerprint { vertices: 3, faces: 2, hash: 9774961354207999086 }  21.074504ms
voxel(0.05)             Ok(Fingerprint { vertices: 44880, faces: 14960, hash: 7896778119327624079 })  56.31826ms
voxel_fp(0.05)          Ok(Fingerprint { vertices: 44784, faces: 14928, hash: 6623905968992935240 })  67.947433ms
-- RAYON_NUM_THREADS=1
voxel(0.05)             Ok(Fingerprint { vertices: 44880, faces: 14960, hash: 7896778119327624079 })  90.641807ms
voxel_fp(0.05)          Ok(Fingerprint { vertices: 44784, faces: 14928, hash: 3411817374792925112 })  101.089847ms
```

`voxel` uses marching cubes (`MeshingMethod::Manifold`). `voxel_fp` uses dual contouring (`MeshingMethod::FeaturePreserving`).

## 5. Determinism

| Operation | Native, separate processes | Native vs wasm32 |
|---|---|---|
| Isotropic remesh | 10/10 identical | identical (asserted) |
| Decimate (error 0.1) | 10/10 identical | identical (asserted) |
| Decimate (floor 320) | 10/10 identical | identical (asserted) |
| Voxel, marching cubes | 20/20 identical (4 threads); same with 1 thread | identical (asserted) |
| Voxel, dual contouring | **20 distinct hashes in 20 runs** (4 threads); 5/5 identical with `RAYON_NUM_THREADS=1` | wasm32 equals the 1-thread native hash (asserted) |

Same vertex and face counts every time. Only the order differs. `TriangulateVisitor::dense` extends a shared `Mutex<Vec<_>>` once per leaf, in whatever order the threads finish.

`std` `HashMap`s seed from OS randomness per process natively, and use fixed keys on wasm32. The remesher's `Grid` and the decimator's quadric map are hash maps used for lookups. Identical output across processes is evidence, for this input, that their iteration order doesn't reach the output [likely].

## 6. Library behaviour a wrapper must guard

- **The decimation floor is ignored.** `EdgeDecimator::<f64, AlwaysDecimate>::new().min_faces_count(Some(320))` turns a closed 1,280-face sphere into **2 faces on 3 vertices**, with no error. The face-count check breaks out of the inner loop, and the outer loop (up to 200 passes) refills the queue.
- **The decimation threshold is absolute and cliff-shaped.** The error is in model units squared. Thresholds from 1e-6 to 1e-2 removed nothing; 0.1 left 204 faces.
- **Voxel output is a triangle soup**: 44,880 vertices for 14,960 faces, so it must be welded before topology work.
- **Voxel input is f32 only.**
- **No progress or yield API.** Each operation is one call. Native remeshing to 9k faces takes about 105 ms.

## 7. Patch experiment (scratch, not committed)

The change: a scratch copy of `baby_shark` 0.3.12, swapped in with `[patch.crates-io]` in a scratch copy of this harness.

```diff
 [dependencies.faer]
 version = "0.22.4"
+default-features = false
+features = ["std", "sparse-linalg"]
 [dependencies.rayon]
 version = "1.8.1"
+optional = true
 [target.'cfg(target_arch ="wasm32")'.dependencies.getrandom]
 version = "0.2"
 features = ["js"]
+optional = true
 [target.'cfg(target_arch ="wasm32")'.dependencies.wasm-bindgen]
 version = "0.2.100"
+optional = true
+[features]
+default = []
+parallel = ["dep:rayon", "faer/rayon"]
+wasm-bindings = ["dep:wasm-bindgen", "dep:getrandom"]
```

```diff
--- src/lib.rs
-#[cfg(target_arch = "wasm32")]
+#[cfg(all(target_arch = "wasm32", feature = "wasm-bindings"))]
 mod wasm;
--- src/voxel/mesh_to_volume.rs
-use rayon::prelude::*;
-            .par_iter()
+            .iter()
--- src/voxel/root_node/tree_node.rs, src/voxel/internal_node/tree_node.rs
-            .par_bridge()
-            .into_par_iter()
             .for_each(...)
```

The experiment made the three call sites plainly sequential. An upstream PR would `cfg(feature = "parallel")` both paths.

What it showed:

- `cargo tree --target wasm32-unknown-unknown -e normal`: no `rayon`, `rayon-core`, `wasm-bindgen`, `js-sys`, `getrandom`, or `rand`.
- `cargo deny … check bans licenses` → `bans ok, licenses ok`.
- `cargo test --release` (native, `RAYON_NUM_THREADS=1` path, so exact hashes) and `cargo test --release --target wasm32-unknown-unknown` → `5 passed`. **Every fingerprint is unchanged.** Dual contouring is now deterministic natively, because it runs on one thread.

## 8. WASM size

The probe is a scratch `cdylib` that depends on this harness and exports `run_remesh`, `run_decimate`, and `run_voxel` as plain `extern "C"` functions. Profile: `opt-level = "s"`, LTO, `codegen-units = 1`, `panic = "abort"`, `strip`. No `wasm-opt`.

| Build | `.wasm` bytes | gzip -9 | Exports | Imports |
|---|---|---|---|---|
| Published `baby_shark` 0.3.12 | 890,985 | 265,398 | 1,426 (`__wbg_mesh_free_…`, `__wbindgen_describe_…`, …) | 6 (`__wbindgen_placeholder__.*`) |
| Patched (above) | 238,025 | 85,480 | 4 (`memory` + 3 probes) | 0 |

The published build can't even be instantiated without wasm-bindgen post-processing, because it imports `__wbindgen_placeholder__`.
