# Engine Lane — `crates/` and `bindings/`

Applies in addition to the root `AGENTS.md`.

## Where code goes
| Crate | Owns |
|---|---|
| `openplush-geom` | f64 vectors, robust predicates, the tolerance policy (`epsilon`), millimetre units |
| `openplush-mesh` | `TriangleSoup`, validation, repair, half-edge `ManifoldMesh`, and `id_newtype!`, which defines `u32` ID newtypes |
| `openplush-io` | bytes → `TriangleSoup` (OBJ, glTF, STL, PLY) |
| `openplush-seams` | `SeamSet`, cut-graph completion, darts, symmetry |
| `openplush-fields` | curvature, nap/grain direction fields |
| `openplush-flatten` | charts, Tutte/LSCM init, SLIM/ARAP, distortion metrics |
| `openplush-pattern` | allowance, notches, labels, grain, piece dedupe |
| `openplush-decal` | markings, detail parts, texture transfer |
| `openplush-layout` | page tiling, nesting adapter |
| `openplush-export` | SVG/PDF → bytes |
| `openplush-project` | project document, schema versions, migrations |
| `openplush-pipeline` | the only public engine API |

Depend downward only (DAG in `specs/001-pattern-pipeline/plan.md`). `bindings/*` depend on `openplush-pipeline` only; it re-exports the project document API (D-020). A new sideways edge needs an ADR.

## Rust rules
- No `unwrap`, `expect`, `panic!`, or panicking index on user-derived data. Allowed only for internal invariants, with a `// INVARIANT:` comment naming it.
- One `thiserror` error enum per crate. Errors carry element IDs (`FaceId`, `EdgeId`, `VertexId`) so the UI can highlight them.
- Newtype IDs across every API — never raw `usize`/`u32`.
- `f64` for geometry. Tolerances come from `openplush_geom::epsilon`: model-space ones relative to the bbox diagonal, physical ones (seam parity) in millimetres; no literal `1e-9`s in algorithm code.
- `#![forbid(unsafe_code)]` in engine crates unless an ADR says otherwise.

## Rust design
- Make invalid states unrepresentable: private fields plus one validating constructor (`Proposal::new` is the model); typestate at stage boundaries (Principle III).
- `pub(crate)` by default; `pub` only for what another crate calls (`unreachable_pub` is denied).
- Borrow in, own out. No `clone()` just to satisfy the borrow checker.
- Enums over `bool` flags and stringly-typed values; `#[non_exhaustive]` on public enums that will grow.
- Iterators over index loops unless the index is the point.
- Docs follow the root comment rules: one line, saying what the signature doesn't. Only `openplush-pipeline`, the public API, requires a doc on every item.
- Modules ≤ 400 lines, functions ≤ 60 lines, ≤ 5 parameters (`scripts/check.sh` fails above). Split by responsibility, not by convenience. A function already over the limit carries `#[expect(clippy::too_many_lines, reason = "…")]` until it is split; never add a new one.

## WASM constraints
Every crate in `crates/` must build for `wasm32-unknown-unknown`.
- No `std::fs`, `std::thread`, `std::time::Instant`, `rayon`, or OS randomness. `faer` with `default-features = false`.
- Long operations expose step-wise sessions (`step() -> Progress`). No loop runs > ~50 ms without a yield point.
- A panic cannot be caught on `wasm32-unknown-unknown` (`panic = "abort"`): it traps the instance. So no engine code relies on `catch_unwind`; a panic is a bug, and the shell reports it as a worker crash (`EngineCrashed`), as the native CLI exits 101 ([ADR 0009](../docs/adr/0009-wasm-panic-is-a-worker-crash.md)). `openplush-wasm` must install a panic hook that forwards the message to the worker console.

## Determinism
- `HashMap`/`HashSet` and unstable sorts are disallowed in output-affecting code (clippy `disallowed-types`, `disallowed-methods`); each opt-out is listed in `scripts/disallowed-opt-outs.txt`.
- Sort with a total order (`total_cmp`), tie-breaking on ID.
- Property tests run with fixed seeds in CI; a failing seed becomes a committed regression case.

## Geometry gotchas
- glTF exporters split vertices at UV/normal seams. Weld before any topology work.
- UVs and normals are per-corner (half-edge), not per-vertex.
- Twins are paired indices: `twin(h) = h ^ 1`, `edge(h) = h >> 1`. Allocate half-edges in pairs.
- Boundary half-edges are explicit (`face = None`). A boundary vertex's outgoing half-edge is its boundary half-edge.
- A chart flattens only if it is a disk: connected, one boundary loop, χ = 1. Darts are slits, not separations.
- Flattening is scale-equivariant. Never bake physical scale into the flattener.
- Every flattened triangle needs det J > 0; line searches must preserve it.
- Seams are edge flags. The mesh never mutates when seams change.

## Tests
- Each algorithm has an oracle test independent of its implementation (developable surfaces flatten with ≈ 0 distortion; invariants hold).
- Mesh data over ~20 triangles comes from `openplush-testkit` fixtures, not inline literals.
- Golden output uses `insta`. Snapshot changes are accepted by a human, never by an agent.

## Solver notes
- Reuse the symbolic factorization across iterations; refactor numerically only.
- Keep solver ordering fixed so results stay deterministic.
