# S2a results (2026-10-18)

Toolchain: rustc 1.97.0 (2d8144b78 2026-07-07). Runner: wasm-bindgen-test-runner 0.2.129 on Node v22.22.0. cargo-deny 0.20.2. Native CPU flags: avx2, avx512f, fma, sse4_2.

## Native (`cargo run --release`, both configurations, identical)

```
n=10 unknowns=100 fingerprint=0xe433e5d06e73a7b4 residual=1.2434497875801753e-14
n=50 unknowns=2500 fingerprint=0x55a5307d75b46427 residual=5.950795411990839e-14
n=100 unknowns=10000 fingerprint=0x20aed9c3f66244a3 residual=7.638334409421077e-14
```

Three release runs and one debug run: byte-identical output.

## wasm32 (`--test cross_target`)

| Configuration | Release | Debug |
|---|---|---|
| A (`sparse-linalg`) | 2 passed | 2 passed |
| B (A + `std`) | 2 passed | not run |

## `cargo deny` against `../../deny.toml`

Both configurations:

```
warning[duplicate]: found 2 duplicate entries for crate 'equator'
warning[duplicate]: found 2 duplicate entries for crate 'equator-macro'
warning[duplicate]: found 2 duplicate entries for crate 'syn'
error[unmaintained]: paste - no longer maintained   (RUSTSEC-2024-0436; Solution: No safe upgrade is available!)
advisories FAILED, bans ok, licenses ok, sources ok
```

## `cargo tree -i paste -e normal,build`

```
paste v1.0.15 (proc-macro)
├── gemm v0.19.0 ── faer v0.24.4
├── gemm-c32, gemm-c64, gemm-common, gemm-f32, gemm-f64 v0.19.0
└── pulp v0.22.3 ── faer v0.24.4
```

## Configuration A, full normal-dependency set

bitflags 2.13.2, bytemuck 1.25.2, cfg-if 1.0.5, dyn-stack 0.13.2, dyn-stack-macros 0.1.3, equator 0.2.2 and 0.6.0, equator-macro 0.2.1 and 0.6.0, faer 0.24.4, faer-traits 0.24.0, gemm 0.19.0 (+ c32, c64, common, f32, f64), generativity 1.2.1, libm 0.2.16, nano-gemm 0.2.2 (+ c32, c64, core, f32, f64), num-complex 0.4.6, num-traits 0.2.19, once_cell 1.21.4, paste 1.0.15, proc-macro2, pulp 0.22.3, pulp-wasm-simd-flag 0.1.1, qd 0.8.0, quote, raw-cpuid 11.6.0, reborrow 0.5.5, seq-macro 0.3.6, syn, unicode-ident.

Configuration B adds libc 0.2.191, num_cpus 1.17.0, private-gemm-x86 0.1.22.
