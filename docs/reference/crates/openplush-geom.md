# openplush-geom

<!-- index: vectors, exact predicates, the tolerance constants and millimetre units, and what the crate depends on. -->

What `openplush-geom` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

- `DVec2`, `DVec3`
- exact predicates `orient2d`, `orient3d`, `incircle` returning `Sign`
- `epsilon`:
  - `ModelTolerances`, `SeamParity`, the constants behind them
  - `UV_CUT`, the absolute UV-unit threshold above which two corner UVs make a cut
  - `FOLD_ANGLE`, 5° in radians, within which two adjacent faces' normals being antiparallel makes a fold
  - `DELAUNAY_COT`, the slack below zero a cotangent sum must pass before an edge is swapped
  - `UNIT_QUATERNION_TOLERANCE`, 1e-6, how far a rotation quaternion's squared length may be from 1
- `units` (`Mm`, `Scale`, with `Scale::to_mm` for a length and `Scale::point_to_mm` for a flat position)
- `GeomError`

## Depends on

`robust`, `thiserror`
