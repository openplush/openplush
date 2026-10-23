# openplush-flatten

<!-- index: the injectivity check, distortion measures and their summary, and the step-wise flattening session. -->

What `openplush-flatten` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

### Injectivity

`injectivity::verify(chart, faces)` takes `(FaceId, [DVec2; 3])` triangles and passes only if every one is strictly counter-clockwise by the exact `orient2d` predicate (det J > 0).

### Errors

`FlattenError`:

- `Flip` (every inverted or collinear face, in input order)
- `EmptyChart` (no faces)
- `Unjudgeable` (a face with a non-finite or extreme coordinate, the predicate's `GeomError` attached; it is returned at once and wins over a flip found earlier)
- `Degenerate` (a face with zero, non-finite or overflowing area, or naming a vertex with no position)
- `NotADisk` (not one connected, consistently wound surface with a single boundary loop and Euler characteristic 1)
- `SolveFailed` (a sparse solve refused)
- `BoundaryNotRealizable` (the boundary could not close at its 3D edge lengths; carries `residual`, how far the boundary polygon at the interior's own turning ends from its start, in the chart's units, and `dart`, the vertices of the shortest edge path from a boundary vertex to the interior vertex with the most angle defect, or `None` when the chart has no interior vertex; parity is never relaxed)

`NotADisk`, `SolveFailed` and `Degenerate` name only the chart (and the face, for `Degenerate`).

### Distortion

`metrics::distortion(chart, xyz, faces, uv)` measures a layout (`uv` indexed like `xyz`) face by face, in input order: each `Distortion` holds the singular values of the Jacobian from the face's 3D shape to its flat one, `stretch()` (σ₁, the largest length ratio) and `compression()` (σ₂, the smallest), with `area_ratio()` (σ₁σ₂) and `conformal_ratio()` (σ₁/σ₂); no thresholds, since tolerances are the fabric's. It refuses with `EmptyChart` (no faces), `Degenerate` (the first face with no 3D area, a vertex with no position in `xyz` or `uv`, or a non-finite or overflowing Jacobian) or `Flip` (every face with det J ≤ 0, in input order).

### Summary

`summary::summarize(chart, xyz, faces, uv, stopped_by)` sums a layout up from the same measures, with the same refusals:

- `most_stretched()` and `most_compressed()` (the face with the largest σ₁ or smallest σ₂ and that value, the first in input order on a tie)
- `tail_stretch()` and `tail_compression()` (the largest σ₁, or smallest σ₂, such that the faces at least that distorted cover 5% of the 3D area)
- `stretch_excess()` and `compression_deficit()` (Σ A·max(0, σ₁ − 1) / ΣA and Σ A·max(0, 1 − σ₂) / ΣA over 3D areas A)
- `area_ratio()` (total flat over total 3D area)
- `stopped_by()` (the `StoppedBy` it was given: why the flatten that made the layout stopped, which the layout cannot show)

The means are running means, so σ near `f64::MAX` never overflows a partial sum.

### Flattening session

`FlattenSession::new(chart, xyz, faces)` flattens one chart a step at a time; it reads `xyz` and owns `faces`, holding no borrow, so a caller can keep the session between steps: `step()` runs one SLIM iteration and returns `Progress::Working { iteration, energy }` or `Progress::Done` (further steps stay `Done`); `finish()` returns every vertex's flat position in vertex order with why the flatten stopped, or `None` before `Done`, so a half-flattened layout is never handed out; dropping the session cancels it. Errors are `FlattenError`.

`StoppedBy` (`#[non_exhaustive]`) is why: `IterationCap` (200 iterations; an iteration that reaches the cap and also gains less than the threshold counts as the cap), `Settled` (an iteration lowered the energy by less than 1e-9 of its value) or `NoDescent` (no step tried lowered it, or the chart has no interior vertex to move).

## Crate-private

- `solver` (sparse symmetric positive-definite solves, analysed once per sparsity pattern)
- `boundary` (a chart's boundary laid flat at its 3D edge lengths)
- `fallback` (the gap and the dart reported when that boundary will not close)
- `interior`, the SLIM solve: the boundary stays where `boundary` put it; the interior starts from a cotangent-weighted harmonic fill, or a uniform-weighted one if that flips a face (both flipped: `Flip` for the start with fewer flipped faces, the cotangent one on a tie); each iteration solves SLIM's reweighted least squares with one analysis for the whole run, then a line search capped at the longest step that flips no face; it settles when an iteration lowers the symmetric Dirichlet energy by less than 1e-9 of its value, when no tried step lowers it, or after 200 iterations (reaching the cap is not an error: the layout is valid)

## Depends on

`openplush-geom`, `openplush-mesh`, `openplush-seams`, `faer` (no default features), `libm`, `thiserror`
