# Spike H1: default thresholds for the hint detectors (002 T005)

**Status (2026-10-20): the defaults below were accepted as 002 D-011 in the 002 plan's Decision Ledger, as provisional values.** They are tuned on five procedural fixtures and no real plush model, so they are [likely], not [solid]; 002 T004's hand-made patterns are what would settle them. Nothing here ships; 002 T028, 002 T029 and 002 T031 implement the same definitions.

Non-shipping research code (research/AGENTS.md). Standard library only; `requirements.txt` is empty on purpose.

```sh
python3 sweep.py             # the tables in results.md, about 2 s
python3 sweep.py --absolute  # the same dart table with research R1's |K| sum
python3 -m unittest -v       # 8 oracles
```

## The question

Research R1 and 002 D-003 fix the shape of each detector (a pure threshold over a precomputed signal, fixed absolute thresholds, chains and clusters built once) and leave the numbers open: the dihedral threshold for sharp edges, the lower one for concave creases, the minimum chain length, and the curvature threshold and integration radius for dart candidates. H1 asks which defaults let a typical plush yield useful hints with no adjustment.

## Method

`signals.py` implements the signals as reference code, with definitions copied from research R1 and data-model section 3:

- **Fold**: the angle between the two faces' normals on each interior edge, signed negative where the second face lies on the outside of the first (a valley). A boundary edge has none.
- **Chains**: built once at the most permissive fold (10°) from every qualifying edge; at a vertex with more than two, edges pair off by the smallest turning angle, ties by lowest edge key (data-model section 8). Each chain carries its length in mm and its length-weighted mean |fold|; the sweep filters chains by those two numbers.
- **Dart signal**: the sum of the angle defects (360° minus the corner angles) over every vertex within geodesic distance of the radius, found by Dijkstra over mesh edges, taken as an absolute value after summing. Clusters are connected groups of vertices over the most permissive threshold, built once, one candidate per cluster at its peak; a higher threshold keeps the clusters whose peak reaches it.

`sweep.py` counts hints per fixture over a grid of thresholds and places the quadruped's hints by region (the solids that make it, from `source_quadruped.py`). Fixtures: the cube, sphere and capsule OBJs (108, 168 and 192 triangles), `faceted.glb` (80) and `quadruped.glb` (9,564).

"Useful" is made testable by what the spec names. SC-010 and the spec's independent test: a smooth sphere gives no sharp-edge or concave-crease hint; a faceted model stays under 50 per category; on the quadruped the muzzle fold, the neck crease and the head dome are found.

## Findings

Numbers in [`results.md`](results.md).

1. **The dihedral threshold is set by the coarsest "smooth" shape, not by the creases.** The 168-triangle sphere's chains have mean folds between 20° and 25°: 19 chains at 20°, none from 25°. A sharp-edge default under 25° would flag a ball. The faceted icosphere folds 18 to 22.5°, so it reads the same. [solid on these meshes]
2. **The quadruped's sharp edges are small-radius tips, and the interesting creases are concave.** At 30° and 20 mm there is one sharp-edge chain (the muzzle tip); at 25° there are 41, mostly the feet, tail and ear tips of 7 to 10 mm radius on a 4 mm grid, which are most plausibly the mesh's facets rather than the shape [speculative]. The spec's three named features are the muzzle fold and neck crease (concave) and the dome (curvature), so a quiet sharp-edge default is the right outcome, not a miss. [likely]
3. **Concave creases separate cleanly from 25°.** The cube, sphere, capsule and faceted models have no concave fold at all. On the quadruped, at 20 mm: 30° and 35° find exactly the neck (2 chains) and the muzzle fold (1); 25° adds the leg and ear roots (13 chains in all), which research R1 calls load-bearing ("the join where a limb meets a body"); 20° adds four chains elsewhere. [likely]
4. **Length filters earn their keep at the concave default.** At 25° the quadruped has 45 chains with no length floor, 21 at 10 mm, 13 at 20 mm and 10 at 30 mm; 24 of the 45 are under 10 mm, a couple of edges of the 4 mm grid. 20 mm leaves the neck, muzzle and limb-root rings, which are tens of millimetres around. [likely]
5. **Signed integration is the working dart signal; R1's absolute sum is not.** Over a disk of 15 mm the quadruped yields 20 clusters at 30° with the signed sum and one with the absolute sum: a grid mesh alternates saddle and peak vertices, and summing |defect| measures that noise and merges the whole surface into one region. Summing first and taking the absolute value cancels it. The cube is unaffected (8 corners, 90° each). This revises R1's "integrated absolute Gaussian curvature" wording. [solid on these meshes]
6. **The radius sets what size of feature qualifies, and 15 mm is the middle.** At 10 mm the quadruped yields 11 candidates at 60° including all four feet and two leg-root clusters; at 20 mm the sphere and the capsule start to register (1 and 2 clusters at 60°). At 15 mm and 60° the quadruped has 7 (two feet, an ear, the muzzle tip, the neck, the tail, the chest), the cube 8, and the sphere, capsule and faceted model none. A 60° disk at 15 mm is the curvature of a spherical cap of radius about 26 mm (2π(1 − cos(15/R)) = 60°), so the default says "tighter than a ball of 51 mm diameter needs a dart". [likely for the fixture counts, speculative as a statement about fabric; 001's Fabric presets are what should confirm it]
7. **Clustering puts the head dome's hint on an ear.** The head-top vertex reads 174° at 15 mm, well over 60°, but it is connected to the ear roots, and a cluster is located at its peak, which is on the left ear at (−14, 71, 157) with 325°. The dome is found, in the sense that its region is flagged, but the hint's marker sits on the ear. 002 T031 should know this: either place the marker at the cluster's centroid of mass, or split clusters at a neck in the signal. Not decided here. [solid]
8. **The faceted fixture does not flood by chain count.** Its 120 edges chain into 10 chains at 20° and none from 25°, under the 50-per-category cap of SC-010 at every threshold, so the flood path (data-model section 8, "reported as flooded") is not reached by it. The fixture's note counts edges; hints are chains. Either the cap counts something finer than chains, or `faceted.glb` needs a denser or noisier sibling to exercise the flood. Flagged for the maintainer, not changed (the fixture is merged and the cap's unit is a spec question).

## Recommended defaults (accepted provisionally as 002 D-011)

| Quantity | Default | Why |
|---|---|---|
| Sharp edge, mean fold | 30° | Above the coarse sphere's 24° and the faceted icosphere's 22.5°; one hint on the quadruped, not 41 |
| Concave crease, mean fold | 25° | Lower than sharp edges, as R1 predicted; silent on every non-concave fixture; finds the neck, the muzzle fold and the limb roots |
| Minimum chain length | 20 mm | Drops single-facet chains (45 → 13 at the concave default); physical units, per FR-007 |
| Dart integration radius | 15 mm | The middle radius; the disk spans a few cells on the 4 mm quadruped and a few triangles on the coarse fixtures |
| Dart curvature, summed angle defect | 60° | Seven candidates on the quadruped, eight on the cube, none on the sphere, capsule or faceted model |

Sensitivity maps the defaults down monotonically (002 D-002); this spike sets only where the default sits.

## Limits

- Five fixtures, three of them 100 to 200 triangles. Dihedral folds depend on tessellation, so every fold number above is as much a statement about these meshes as about shapes.
- No real plush model and no hand-made pattern (002 T004), so "useful" is judged by the spec's named features and by silence on smooth shapes, not by agreement with a sewist.
- One tessellation of the quadruped (4 mm grid). A finer mesh would lower every fold and raise the dart signal's resolution; the chain-length and radius defaults are in millimetres precisely so they would not move, but the dihedral ones would.
- Chains here pair every junction by smallest turning angle; a detector that ended chains at junctions would give more, shorter chains and move every count.
