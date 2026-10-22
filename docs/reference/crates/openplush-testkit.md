# openplush-testkit

<!-- index: the test fixtures loader, binary FBX test files built in code, and the independent oracles tests check results against. -->

What `openplush-testkit` contains **today**, and what it actually depends on. Dev-dependency only.

## Public items

### Fixtures

`fixtures`: `load(Kind, name)` returns a fixture's bytes, `names(Kind)` lists every fixture of a kind sorted, and `FixtureError` covers a bad name, a missing file, an entry that is not a file and a read error. Kinds: `Mesh` (`fixtures/meshes/*.obj`), `Defect` (`fixtures/defects/*.obj`), `Seams` (`fixtures/seams/*.json`).

### FBX test files

`fbx`: binary FBX 7.4 files built in code, written with `fbxcel`'s writer, arrays zlib-compressed as exporters write them. `Scene` holds `GlobalSettings` properties (`axes` sets the axis system), objects and object-to-object connections (`connect`, 0 the document's root), and `bytes()` writes it; `binary(top)` writes any top-level `Node`s. `object`, `model`, `mesh`, `material`, `layer`, `properties`, `vector` and `int` build the nodes as FBX lays them out.

### Oracles

`oracles`: checks that recompute a property of a flattened `Chart` (plain `xyz`, `uv` and face arrays) from raw coordinates, independent of the code under test:

- `injective` (every flat face strictly counter-clockwise, exact predicate)
- `developable` (both signed singular values within a tolerance of 1, so a mirrored face fails)
- `area_conserved` (signed flat area within a relative tolerance of 3D area)

and, on plain values:

- `outline_simple` (a closed outline with no crossing, touching, zero-length or doubled-back edge, wound counter-clockwise, all by exact predicates; an outline the predicates cannot decide fails)
- `seam_parity` (max(1.0 mm, 0.5% of the longer side), restated rather than taken from `SeamParity`)

Each reports an `OracleFailure`, which names the face, edge or lengths involved where one applies; a chart with no faces fails as `Empty`, and a NaN, infinite or negative tolerance as `BadTolerance`.

## Depends on

`openplush-geom`, `openplush-mesh`, `thiserror`, `fbxcel` (`writer` only)
