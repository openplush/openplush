# Quickstart: validating source seam import

**Feature**: `003-source-seam-import` | **Date**: 2026-10-16

How to prove the feature works end to end once it is built. Commands follow [contracts/source-import.md](./contracts/source-import.md); types follow [data-model.md](./data-model.md).

## Prerequisites

- 001's import, repair, seam, flatten and pattern stages are working (see [plan.md](./plan.md), "Hard dependency on 001").
- Fixtures under `fixtures/meshes/source-seams/`, each with a license entry:

| Fixture | Built to prove |
|---|---|
| `capsule-uv.glb`, `capsule-uv.obj` | UV cuts along a known layout, same model in both formats (SC-001) |
| `capsule-hardshade.glb` | Same layout, plus hard-shaded edges that are not cuts (SC-002) |
| `sphere-band.glb` | Two ring cuts that leave a band that can't lie flat (US2) |
| `three-materials.glb` | "minky / cuddle" (case), "Polar fleece " (trailing space) and "Felt Nose" (US3) |
| `dead-end.glb` | Two deliberate dead-ended cuts, one cut ending on a material boundary (US4, FR-013) |
| `no-uv.obj` | No texture coordinates, one material (FR-005) |
| `reshaped-v1.glb`, `reshaped-v2.glb` | Same model before and after stretching one end cap (FR-050); spike Q4 measures the same on a real Blender edit |

## Scenarios

1. **UV seams in** (US1, SC-001)
   `openplush-cli import-seams capsule-uv.glb --json out.json`
   Expect: proposed edges equal the fixture's recorded layout exactly. The OBJ twin gives the same edge set by position.

2. **Shading is not a seam** (SC-002)
   Same command on `capsule-hardshade.glb`. Expect the same edge set as scenario 1.

3. **Closing seams** (US2, SC-003)
   `openplush-cli import-seams sphere-band.glb --json out.json`
   Expect: a non-empty `closing`. Then `pattern … --seams-from-model --accept-closing` produces pieces, and without `--accept-closing` it exits with `ChartNotDisk`.

4. **Materials to fabrics** (US3, SC-005)
   `openplush-cli import-seams three-materials.glb --json out.json`
   Expect: three regions; "minky / cuddle" and "Polar fleece " matched to the "Minky / cuddle" and "Polar fleece" presets; one created profile "Felt Nose" with the minky preset's values and an `UnreviewedFabric` warning that disappears once the profile is edited (FR-025); a seam on every material boundary.

5. **Dead ends** (US4, SC-004)
   `openplush-cli import-seams dead-end.glb --json out.json`
   Expect: `DeadEndCut` with exactly two tips. The cut that ends on a material boundary is not reported.

6. **No UVs** (FR-005)
   Expect: report `uv_set_read: null`, no UV seams, and one region, because the fixture has one material (with none, FR-005 proposes no regions).

7. **Re-import** (FR-050 to FR-052)
   `openplush-cli import-seams reshaped-v2.glb --project reshaped-v1.plush --on-existing merge --json diff.json`
   Expect: seams away from the stretched cap re-placed; any that aren't are listed as unplaceable, never silently gone.

8. **Determinism and privacy** (SC-007, SC-008)
   Run scenario 4 twice, natively and under WASM: byte-identical JSON. The web smoke test records zero network requests during import and accept.

9. **In the app** (FR-040 to FR-044)
   Import `sphere-band.glb`. Imported and extra seams are visually distinct; "accept all" is one undo step; keyboard alone can accept, partially accept, and decline.
