# Add a fixture model

Build a hand-modelled test model in Blender and add it to `fixtures/models/`. Models a script can make (cubes, spheres, capsules, defects, seam sets) belong in [`research/fixtures_gen/`](../../research/fixtures_gen/README.md) instead.

## Build

1. Scene Properties > Units: Metric, Unit Scale `0.001`, Length millimetres. Size the model in real units (a plush is roughly 100-600 mm).
2. Model it closed and manifold: Edit Mode > Select > All by Trait > Non-Manifold selects nothing. Shift+N recalculates normals outward.
3. Apply every transform (Ctrl+A > All Transforms) and every modifier you want kept (a Mirror included).
4. Keep a symmetric model's centre line exactly on x = 0.

## Export

| Format | Menu | Settings |
|---|---|---|
| `.glb` | File > Export > glTF 2.0 | Format: glTF Binary; Apply Modifiers on; UVs on; Materials on; Animation and Skinning off |
| `.obj` | File > Export > Wavefront | Apply Modifiers on; Triangulated Mesh on; UV Coordinates on; Normals off unless the model needs hard shading; Materials on only if it has some |

Reopen the exported file in Blender and check its size and orientation. Name the file for what it proves (`near-symmetric.glb`, not `ear-test-2.glb`).

## Add

1. Put the file in `fixtures/models/` (a task's fixture group may use a subdirectory, such as `source-seams/`).
2. Add a row to [`fixtures/LICENSES.md`](../../fixtures/LICENSES.md): path, `CC0-1.0` (or `CC-BY-4.0` with its attribution text), and "Hand-modelled in Blender by <name>". Only models you made yourself, or that are CC0 or CC-BY, may ship.
3. One fixture group per PR, with no code in it (the `lane:fixtures` path). Reviewers look at the model; they do not read a diff.
