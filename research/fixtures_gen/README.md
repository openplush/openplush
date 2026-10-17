# Fixture generator

Non-shipping research code that writes the procedural part of [`fixtures/`](../../fixtures/README.md): the clean base meshes, one model per defect class, and reference seam sets. Every file it writes is committed, and the tests prove the committed files are exactly what it produces.

```sh
python3 generate.py            # rewrite fixtures/meshes (including meshes/source-seams), fixtures/defects, fixtures/seams
python3 -m unittest -v         # oracles over the committed files, plus a byte-for-byte regeneration check
```

Standard library only (`requirements.txt` is empty on purpose). The interpreters it was run with are in [`results.md`](results.md). Coordinates are rounded to 6 decimals, so a last-bit difference in a platform's `sin`/`cos` would only show up if it straddled a rounding tie [solid: the nearest current value is 3e-8 mm from one, about a million times a last-bit error at this scale].

## What it makes

All coordinates are millimetres, faces are triangles wound counter-clockwise seen from outside, and OBJ indices are 1-based.

| File | Shape |
|---|---|
| `meshes/cube.obj` | 90 mm cube, each side a 3 × 3 grid of quads (108 triangles). A 12-triangle cube has no interior vertices, so a chart would have nothing to flatten and the defects would have to sit on top of each other; the grid gives each face chart 4 interior vertices and keeps every coordinate and edge midpoint exact |
| `meshes/sphere.obj` | Radius 50 mm, 12 segments × 8 bands (168 triangles) |
| `meshes/capsule.obj` | Radius 30 mm, 120 mm long: two hemispheres and a cylinder (192 triangles) |
| `defects/*.obj` | The cube plus exactly one defect each; the second header line says what and where |
| `seams/*.json` | One reference seam set per mesh: sphere in six gores, capsule as two caps and a slit tube, cube as a cross-shaped net, in the [seam-set format](../../fixtures/README.md#seam-set-format) |
| `meshes/source-seams/capsule-uv.obj`, `.glb` | The capsule with UV coordinates cut along meridian 0 (pole to pole) and around the tube's upper rim (z = +30 mm, where it meets the hemisphere), so two UV islands: the upper hemisphere and the rest. Written by `source_seams.py` the way Blender's exporters write a UV-unwrapped model: the glTF has one vertex per distinct (position, UV), so it splits at every cut, and flips V; the OBJ indexes a `vt` per corner. Pole corners in an island share one UV, so a pole fan carries no cut. `seams/capsule-uv.json` lists the cuts by the OBJ's vertex indices |
| `meshes/source-seams/capsule-hardshade.glb` | The same capsule and UVs, with the lower hemisphere flat-shaded, so its glTF also splits vertices where normals differ. Its UV cuts are still exactly `seams/capsule-uv.json` |
| `meshes/source-seams/sphere-band.obj`, `.glb` | The sphere with UVs cut around two rings (z = ±35.36 mm), like a neck and a waist: two caps and the band between them, each its own UV island in a polar projection, so no meridian is cut. The band is an annulus, so it cannot lie flat in one piece. Its seam set is `seams/non-disk/sphere-band.json`, kept out of `seams/`' top level because the every-chart-is-a-disk check covers only that level |

### Defect fixtures

Each one is the cube with one change, chosen so each fixture trips exactly one detector on its input mesh. The oracle welds coincident positions and drops duplicate faces, then evaluates every topology detector together on that welded, de-duplicated mesh; it repairs nothing.

| Class | Change | Why it does not trip anything else |
|---|---|---|
| `non-manifold-edge` | A closed tetrahedron shares one cube edge (4 faces on it) | Its faces join the cube's fan at both edge ends, so no bowtie; winding is judged only across 2-face edges |
| `bowtie-vertex` | A closed tetrahedron touches one cube corner | Components are counted by shared vertices, so on input it is still one component |
| `flipped-normals` | The +x side is wound inward | Still closed and manifold; the signed volume stays positive |
| `zero-area-face` | One top face is split at the midpoint of edge `ab`, and a collinear sliver `(a, b, midpoint)` closes the gap | Still closed, manifold and consistently wound |
| `duplicate-face` | One face repeated, same winding | The extra edge use disappears once duplicates are dropped |
| `hole` | The centre quad of the top side removed | One 4-edge boundary loop, no bowtie |
| `unwelded-split-vertices` | The top side gets its own coincident copies of its 12 border vertices, as a glTF exporter splits at a UV seam | Welding by position restores the clean cube |
| `self-intersection` | One top vertex pushed through the bottom side, off the grid lines | Topology unchanged; the crossings are transverse |
| `multi-component` | A separate closed tetrahedron beside the cube | Closed, manifold, outward |

"Exactly one" holds for the input, not after repair. Repair splits bowtie vertices before it counts components, so `bowtie-vertex` becomes two components, the cube and the tetrahedron, and a repaired run also reports `multi-component` [solid]. The `non-manifold-edge` fin can likewise end up as its own component once its edge is split [likely]. A test over the engine's repair report should assert that the named class is present, not that it is the only one.

Non-finite coordinates (NaN, ∞) are not a defect class here: the engine rejects them at import with `EngineError::NonFiniteInput` and never repairs them, so they belong with the OBJ import tests rather than in the corpus that measures detection.

### Not covered

The repair pipeline handles these, but the corpus has no fixture for them yet:

- Repeated indices: a face that names one vertex twice.
- An opposite-orientation duplicate face, which repair flags as a double-sided sheet.
- A non-orientable (Möbius-like) winding conflict.

## The oracles

`test_fixtures.py` and `test_source_seams.py` read the committed files and share no code with `generate.py` or `source_seams.py`, except the one test that regenerates into a temporary directory and compares bytes:

- **Clean meshes**: every detector reads zero, Euler characteristic 2, no unused vertex, positive signed volume.
- **Defects**: one file per class and no others; each file trips exactly its own detector. Detectors: coincident positions (weld), repeated vertex sets, faces per edge (> 2 non-manifold, 1 boundary), repeated directed edges across 2-face edges plus closed components with signed volume ≤ 0 (winding), face fans per vertex joined through shared edges (bowtie), vertex-connected components, area below `1e-12 × bbox diagonal²`, and Moller–Trumbore edge-through-triangle tests between faces that share no vertex (self-intersection).
- **Seams**: the JSON is type-checked field by field (an index of `1.0` or `false` is rejected, not compared equal); every listed pair is a mesh edge; cutting virtually (faces flood-filled across uncut edges, corners joined across uncut edges) leaves every chart with Euler characteristic 1 and one boundary loop. The same oracle must reject an uncut closed mesh and an annulus, so it cannot pass vacuously.
- **Source seams**: the UV cuts recomputed from the OBJ's `vt` (an interior edge whose corner UVs differ across it at either endpoint) are exactly the reference seam set, and the rule is checked on a cut that differs at one endpoint only. The layout itself is pinned: 9 meridian edges pole to pole plus the 12 edges of the tube's upper rim, leaving two disk-shaped charts. The glTF is a GLB with valid chunks; matched to the OBJ by position, it has the same triangles and the same corner UVs (V flipped), with one vertex per distinct (position, UV) and no more. Its POSITION bounds equal the data. `capsule-hardshade.glb`, matched to the same OBJ, has the same triangles and UV cuts, more than 50 interior edges whose normals differ but whose UVs do not, and unit-length normals. `sphere-band`'s UV cuts equal its seam set, are two full 12-edge rings at mirrored heights, and leave exactly two disks and one annulus. Every face keeps its outside-seen counter-clockwise winding in UV space, so no island is mirrored.
