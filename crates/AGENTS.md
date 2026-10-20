# Engine Lane — `crates/` and `bindings/`

Applies in addition to the root `AGENTS.md`. Each rule's reason is in [`docs/reference/code-rules.md`](../docs/reference/code-rules.md), under the same heading.

## Where code goes
- Put code in the crate that owns its concern: [the ownership table](../docs/reference/crates.md#which-crate-owns-what).
- Depend downward only (DAG in `specs/001-pattern-pipeline/plan.md`). A new sideways edge needs an ADR.
- `bindings/*` depend on `openplush-pipeline` only.

## Rust rules
- No `unwrap`, `expect`, `panic!`, or panicking index on user-derived data. Allowed only for internal invariants, with a `// INVARIANT:` comment naming it.
- One `thiserror` error enum per crate. Errors carry element IDs (`FaceId`, `EdgeId`, `VertexId`).
- Newtype IDs across every API — never raw `usize`/`u32`.
- `f64` for geometry. Tolerances come from `openplush_geom::epsilon`: model-space ones relative to the bbox diagonal, physical ones (seam parity) in millimetres; no literal `1e-9`s in algorithm code.
- `#![forbid(unsafe_code)]` in engine crates unless an ADR says otherwise.

## Rust design
- Make invalid states unrepresentable: private fields plus one validating constructor (`Proposal::new` is the model); typestate at stage boundaries.
- `pub(crate)` by default; `pub` only for what another crate calls.
- Borrow in, own out. No `clone()` just to satisfy the borrow checker.
- Enums over `bool` flags and stringly-typed values; `#[non_exhaustive]` on public enums that will grow.
- Iterators over index loops unless the index is the point.
- Docs follow the root comment rules. Only `openplush-pipeline` requires a doc on every item.
- Modules ≤ 400 lines, functions ≤ 60 lines, ≤ 5 parameters. Split by responsibility. A function already over the limit carries `#[expect(clippy::too_many_lines, reason = "…")]` until it is split; never add a new one.

## WASM constraints
- Every crate in `crates/` builds for `wasm32-unknown-unknown`.
- `usize` is 32 bits there, tests included: no `usize` value, literal or shift past `u32::MAX`; use `u64`. Run `scripts/check.sh wasm-check`.
- No `std::fs`, `std::thread`, `std::time::Instant`, `rayon`, or OS randomness. `faer` with `default-features = false`.
- Long operations expose step-wise sessions (`step() -> Progress`). No loop runs > ~50 ms without a yield point.
- No `catch_unwind`: a panic is a bug and a worker crash ([ADR 0009](../docs/adr/0009-wasm-panic-is-a-worker-crash.md)). `openplush-wasm` installs a panic hook that forwards the message to the worker console.

## Determinism
- No `HashMap`/`HashSet` or unstable sorts in output-affecting code; each opt-out is listed in `scripts/disallowed-opt-outs.txt`.
- Sort with a total order (`total_cmp`), tie-breaking on ID.
- Property tests run with fixed seeds in CI; a failing seed becomes a committed regression case.

## Geometry gotchas
Why each holds: [meshes](../docs/explanation/meshes.md), [seams](../docs/explanation/seams.md), [flattening](../docs/explanation/flattening.md).
- Weld before any topology work.
- UVs and provenance are per-corner (half-edge), not per-vertex.
- `twin(h) = h ^ 1`, `edge(h) = h >> 1`. Allocate half-edges in pairs.
- Boundary half-edges are explicit (`face = None`). A boundary vertex's outgoing half-edge is its boundary half-edge.
- A chart flattens only if it is a disk: connected, one boundary loop, χ = 1. Darts are slits, not separations.
- Never bake physical scale into the flattener.
- Every flattened triangle needs det J > 0; line searches must preserve it.
- Seams are edge flags. The mesh never mutates when seams change.

## Tests
- Each algorithm has an oracle test independent of its implementation.
- Mesh data over ~20 triangles comes from `openplush-testkit` fixtures, not inline literals.
- Golden output uses `insta`. Snapshot changes are accepted by a human, never by an agent.

## Solver notes
- Reuse the symbolic factorization across iterations; refactor numerically only.
- Keep solver ordering fixed.
