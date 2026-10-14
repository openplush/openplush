---
status: Accepted
date: 2026-10-14
deciders: maintainer (delegated to the implementing agent, reviewed in the T022/T023 PR)
---

# Use the `robust` crate for exact geometric predicates

## Context and problem statement

T023 needs orientation and in-circle predicates whose **sign is exact**. Naive f64 evaluation returns wrong and mutually inconsistent signs near degeneracy (Kettner et al. 2008), and the engine makes topological decisions with these signs: weld and repair, chart construction, flip checks. A wrong sign there is plausible-looking wrong output, the failure Constitution III exists to prevent. This is the first geometry dependency of its kind, so AGENTS.md asks for an ADR.

## Decision drivers

* Exact signs (Constitution III, IV).
* Determinism, native and WASM (Constitution V): no FMA, no platform-specific arithmetic.
* License on the allowlist (Constitution IX), and builds for `wasm32-unknown-unknown`.
* Contributor burden: adaptive-precision arithmetic is subtle, and a bug in it is silent.

## Considered options

* `robust` 1.2.0 (GeoRust): a port of Shewchuk's adaptive-precision predicates.
* Hand-written Shewchuk predicates in `openplush-geom`.
* Exact big-rational arithmetic at runtime (`num-rational`).

## Decision outcome

Chosen option: **`robust`**. It is a direct port of Shewchuk's public-domain reference code, used by `geo`, MIT OR Apache-2.0, `no_std`-capable, and has no dependencies [solid]. Hand-writing in-circle alone is hundreds of lines of error-free-transformation arithmetic in which a typo yields a predicate that is right almost always [likely]. Big rationals are exact but allocate on every call, which is too slow for per-triangle checks in a 30k-triangle solve [likely].

### Consequences

* Good: exact signs for orient2d, orient3d, and incircle; nothing to maintain.
* Good: the public API (`openplush_geom::predicates`) exposes only `Sign`, so swapping the implementation later changes no caller.
* Bad: a third-party crate on the engine's lowest layer. Mitigated by the exact-oracle tests below, which would catch a regression in an upgrade.

### Confirmation

`crates/openplush-geom/tests/predicates.rs` checks every predicate against exact big-rational evaluation (a dev-dependency only) on near-degenerate inputs, including the Kettner et al. classroom grid. It also asserts that naive f64 is wrong on the same inputs, so the tests cannot pass on easy cases. `cargo deny` gates the license, and the wasm32 build gates portability.

## More information

Shewchuk, "Adaptive Precision Floating-Point Arithmetic and Fast Robust Geometric Predicates" (1997). Kettner, Mehlhorn, Pion, Schirra, Yap, "Classroom examples of robustness problems in geometric computations" (2008). ADR 0001 is reserved for the S1 preprocessing decision (T001).
