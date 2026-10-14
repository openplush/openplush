# Fixture generator

Non-shipping research code that writes the procedural part of [`fixtures/`](../../fixtures/README.md): the clean base meshes, one model per defect class, and reference seam sets. Every file it writes is committed, and the tests prove the committed files are exactly what it produces.

```sh
python3 generate.py            # rewrite fixtures/meshes, fixtures/defects, fixtures/seams
python3 -m unittest -v         # oracles over the committed files, plus a byte-for-byte regeneration check
```

Standard library only (`requirements.txt` is empty on purpose). Generated with Python 3.13.16; any Python 3.11+ should reproduce the same bytes. Coordinates are rounded to 6 decimals, so a last-bit difference in a platform's `sin`/`cos` would only show up if it straddled a rounding tie [solid: the nearest current value is 3e-8 mm from one, about a million times a last-bit error at this scale].

## What it makes

All coordinates are millimetres, faces are triangles wound counter-clockwise seen from outside, and OBJ indices are 1-based.

| File | Shape |
|---|---|
| `meshes/cube.obj` | 90 mm cube, each side a 3 × 3 grid of quads (108 triangles). A 12-triangle cube has no interior vertices, so a chart would have nothing to flatten and the defects would have to sit on top of each other; the grid gives each face chart 4 interior vertices and keeps every coordinate and edge midpoint exact |
| `meshes/sphere.obj` | Radius 50 mm, 12 segments × 8 bands (168 triangles) |
| `meshes/capsule.obj` | Radius 30 mm, 120 mm long: two hemispheres and a cylinder (192 triangles) |
| `defects/*.obj` | The cube plus exactly one defect each; the second header line says what and where |
| `seams/*.json` | One reference seam set per mesh: sphere in six gores, capsule as two caps and a slit tube, cube as a cross-shaped net |

### Defect fixtures

Each one is the cube with one change, chosen so it trips exactly one detector when the detectors run in the repair pipeline's order (weld, drop duplicates, then topology):

| Class | Change | Why it does not trip anything else |
|---|---|---|
| `non-manifold-edge` | A closed tetrahedron shares one cube edge (4 faces on it) | Its faces join the cube's fan at both edge ends, so no bowtie; winding is judged only across 2-face edges |
| `bowtie-vertex` | A closed tetrahedron touches one cube corner | Components are counted by shared vertices, so it is still one component |
| `flipped-normals` | The +x side is wound inward | Still closed and manifold |
| `zero-area-face` | One top face is split at the midpoint of edge `ab`, and a collinear sliver `(a, b, midpoint)` closes the gap | Still closed, manifold and consistently wound |
| `duplicate-face` | One face repeated, same winding | The extra edge use disappears once duplicates are dropped |
| `hole` | The centre quad of the top side removed | One 4-edge boundary loop, no bowtie |
| `unwelded-split-vertices` | The top side gets its own coincident copies of its 12 border vertices, as a glTF exporter splits at a UV seam | Welding by position restores the clean cube |
| `self-intersection` | One top vertex pushed through the bottom side, off the grid lines | Topology unchanged; the crossings are transverse |
| `multi-component` | A separate closed tetrahedron beside the cube | Closed, manifold, outward |

Non-finite coordinates (NaN, ∞) are not a defect class here: the engine rejects them at import with `EngineError::NonFiniteInput` and never repairs them, so they belong with the OBJ import tests rather than in the corpus that measures detection.

### Seam-set format

```json
{
  "format": "openplush-seam-set",
  "version": 1,
  "mesh": "meshes/cube.obj",
  "vertexIndexBase": 0,
  "note": "what the cut is",
  "seams": [{"edge": [0, 1], "kind": "cut"}]
}
```

`mesh` is relative to `fixtures/`. An edge is a pair of **0-based OBJ vertex indices** (the order of `v` lines), smaller first, and the list is sorted. This names an edge without depending on the engine's half-edge numbering. `kind` takes the project schema's seam kinds. Every chart left by the cut is a topological disk.

## The oracles

`test_fixtures.py` reads the committed files and shares no code with `generate.py`, except the one test that regenerates into a temporary directory and compares bytes:

- **Clean meshes**: every detector reads zero, Euler characteristic 2, no unused vertex, positive signed volume.
- **Defects**: one file per class and no others; each file trips exactly its own detector. Detectors: coincident positions (weld), repeated vertex sets, faces per edge (> 2 non-manifold, 1 boundary), repeated directed edges across 2-face edges (winding), face fans per vertex joined through shared edges (bowtie), vertex-connected components, area below `1e-12 × bbox diagonal²`, and Moller–Trumbore edge-through-triangle tests between faces that share no vertex (self-intersection).
- **Seams**: every listed pair is a mesh edge; cutting virtually (faces flood-filled across uncut edges, corners joined across uncut edges) leaves every chart with Euler characteristic 1 and one boundary loop. The same oracle must reject an uncut sphere and an annulus, so it cannot pass vacuously.
