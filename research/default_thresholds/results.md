# Spike 002 T005: results

Run on 2026-10-20, Linux x86_64, standard library only. `python3 sweep.py` prints the tables below (about 2 s); the output is byte-identical on CPython 3.11.17, 3.12.3 and 3.13.16, and so is `python3 sweep.py --absolute`. `python3 -m unittest -v`: 8 tests, OK, on all three. `mypy --strict --python-version 3.11 signals.py sweep.py test_default_thresholds.py` (mypy 2.3.1, `MYPYPATH=../symmetry_margin`): no issues.

What the rows are and what they show is in [`README.md`](README.md). Fixtures are `fixtures/meshes/{cube,sphere,capsule}.obj`, `faceted.glb` and `quadruped.glb`.

| Fixture | vertices | faces |
|---|---|---|
| cube | 56 | 108 |
| sphere | 86 | 168 |
| capsule | 98 | 192 |
| faceted | 42 | 80 |
| quadruped | 4784 | 9564 |

## Sharp edges: chains built once at 10°, counted by mean fold and length

| Fixture | min length | ≥ 20° | ≥ 25° | ≥ 30° | ≥ 35° | ≥ 40° | ≥ 45° |
|---|---|---|---|---|---|---|---|
| cube | 0 mm | 6 | 6 | 6 | 6 | 6 | 6 |
| cube | 10 mm | 6 | 6 | 6 | 6 | 6 | 6 |
| cube | 20 mm | 6 | 6 | 6 | 6 | 6 | 6 |
| cube | 30 mm | 6 | 6 | 6 | 6 | 6 | 6 |
| sphere | 0 mm | 19 | 0 | 0 | 0 | 0 | 0 |
| sphere | 10 mm | 19 | 0 | 0 | 0 | 0 | 0 |
| sphere | 20 mm | 19 | 0 | 0 | 0 | 0 | 0 |
| sphere | 30 mm | 19 | 0 | 0 | 0 | 0 | 0 |
| capsule | 0 mm | 16 | 16 | 2 | 0 | 0 | 0 |
| capsule | 10 mm | 16 | 16 | 2 | 0 | 0 | 0 |
| capsule | 20 mm | 16 | 16 | 2 | 0 | 0 | 0 |
| capsule | 30 mm | 16 | 16 | 2 | 0 | 0 | 0 |
| faceted | 0 mm | 10 | 0 | 0 | 0 | 0 | 0 |
| faceted | 10 mm | 10 | 0 | 0 | 0 | 0 | 0 |
| faceted | 20 mm | 10 | 0 | 0 | 0 | 0 | 0 |
| faceted | 30 mm | 10 | 0 | 0 | 0 | 0 | 0 |
| quadruped | 0 mm | 199 | 87 | 7 | 2 | 0 | 0 |
| quadruped | 10 mm | 133 | 55 | 1 | 0 | 0 | 0 |
| quadruped | 20 mm | 83 | 41 | 1 | 0 | 0 | 0 |
| quadruped | 30 mm | 58 | 35 | 1 | 0 | 0 | 0 |

Quadruped sharp edges by region (min length 20 mm):

- ≥ 20°: elsewhere 28, feet 16, leg roots 10, muzzle tip 8, tail 8, tail root 7, dome 3, ears 3
- ≥ 25°: feet 12, elsewhere 8, tail 8, tail root 5, dome 3, ears 3, muzzle tip 2
- ≥ 30°: muzzle tip 1
- ≥ 35°: —
- ≥ 40°: —
- ≥ 45°: —

## Concave creases: chains built once at 10°, counted by mean fold and length

| Fixture | min length | ≥ 15° | ≥ 20° | ≥ 25° | ≥ 30° | ≥ 35° |
|---|---|---|---|---|---|---|
| cube | 0 mm | 0 | 0 | 0 | 0 | 0 |
| cube | 10 mm | 0 | 0 | 0 | 0 | 0 |
| cube | 20 mm | 0 | 0 | 0 | 0 | 0 |
| cube | 30 mm | 0 | 0 | 0 | 0 | 0 |
| sphere | 0 mm | 0 | 0 | 0 | 0 | 0 |
| sphere | 10 mm | 0 | 0 | 0 | 0 | 0 |
| sphere | 20 mm | 0 | 0 | 0 | 0 | 0 |
| sphere | 30 mm | 0 | 0 | 0 | 0 | 0 |
| capsule | 0 mm | 0 | 0 | 0 | 0 | 0 |
| capsule | 10 mm | 0 | 0 | 0 | 0 | 0 |
| capsule | 20 mm | 0 | 0 | 0 | 0 | 0 |
| capsule | 30 mm | 0 | 0 | 0 | 0 | 0 |
| faceted | 0 mm | 0 | 0 | 0 | 0 | 0 |
| faceted | 10 mm | 0 | 0 | 0 | 0 | 0 |
| faceted | 20 mm | 0 | 0 | 0 | 0 | 0 |
| faceted | 30 mm | 0 | 0 | 0 | 0 | 0 |
| quadruped | 0 mm | 267 | 131 | 45 | 17 | 5 |
| quadruped | 10 mm | 114 | 66 | 21 | 3 | 3 |
| quadruped | 20 mm | 48 | 26 | 13 | 3 | 3 |
| quadruped | 30 mm | 21 | 12 | 10 | 2 | 2 |

Quadruped concave creases by region (min length 20 mm):

- ≥ 15°: leg roots 22, elsewhere 10, tail root 5, ears 4, muzzle fold 4, neck 2, muzzle tip 1
- ≥ 20°: leg roots 12, ears 4, elsewhere 4, muzzle fold 3, neck 2, muzzle tip 1
- ≥ 25°: leg roots 6, muzzle fold 3, ears 2, neck 2
- ≥ 30°: neck 2, muzzle fold 1
- ≥ 35°: neck 2, muzzle fold 1

## Dart candidates: clusters of the integrated angle defect over a geodesic disk

| Fixture | radius | ≥ 30° | ≥ 45° | ≥ 60° | ≥ 90° | ≥ 120° | ≥ 180° |
|---|---|---|---|---|---|---|---|
| cube | 10 mm | 8 | 8 | 8 | 6 | 0 | 0 |
| cube | 15 mm | 8 | 8 | 8 | 6 | 0 | 0 |
| cube | 20 mm | 8 | 8 | 8 | 6 | 0 | 0 |
| sphere | 10 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| sphere | 15 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| sphere | 20 mm | 1 | 1 | 1 | 0 | 0 | 0 |
| capsule | 10 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| capsule | 15 mm | 2 | 0 | 0 | 0 | 0 | 0 |
| capsule | 20 mm | 2 | 2 | 2 | 2 | 0 | 0 |
| faceted | 10 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| faceted | 15 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| faceted | 20 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| quadruped | 10 mm | 25 | 11 | 11 | 10 | 10 | 2 |
| quadruped | 15 mm | 20 | 8 | 7 | 6 | 6 | 5 |
| quadruped | 20 mm | 30 | 7 | 7 | 6 | 6 | 6 |

Quadruped dart candidates by region (clusters built once at the first threshold, kept by their peak):

- radius 10 mm, ≥ 30°: elsewhere 7, leg roots 6, feet 4, tail root 3, chest 1, ears 1, muzzle tip 1, neck 1, tail 1; elsewhere at [(-21, 94, 98), (-16, 104, 128), (-6, 103, 136), (6, 103, 136), (8, 92, 88), (16, 104, 128)]
- radius 10 mm, ≥ 45°: feet 4, leg roots 2, ears 1, muzzle tip 1, neck 1, tail 1, tail root 1
- radius 10 mm, ≥ 60°: feet 4, leg roots 2, ears 1, muzzle tip 1, neck 1, tail 1, tail root 1
- radius 10 mm, ≥ 90°: feet 4, leg roots 2, ears 1, muzzle tip 1, neck 1, tail 1
- radius 10 mm, ≥ 120°: feet 4, leg roots 2, ears 1, muzzle tip 1, neck 1, tail 1
- radius 10 mm, ≥ 180°: ears 1, tail 1
- radius 15 mm, ≥ 30°: elsewhere 6, leg roots 4, chest 2, feet 2, rump 2, ears 1, muzzle tip 1, neck 1, tail 1; elsewhere at [(-27, 86, 126), (-18, 106, 122), (-6, 90, 87), (6, 90, 87), (18, 106, 122), (27, 86, 126)]
- radius 15 mm, ≥ 45°: feet 2, chest 1, ears 1, muzzle tip 1, neck 1, rump 1, tail 1
- radius 15 mm, ≥ 60°: feet 2, chest 1, ears 1, muzzle tip 1, neck 1, tail 1
- radius 15 mm, ≥ 90°: feet 2, ears 1, muzzle tip 1, neck 1, tail 1
- radius 15 mm, ≥ 120°: feet 2, ears 1, muzzle tip 1, neck 1, tail 1
- radius 15 mm, ≥ 180°: feet 2, ears 1, muzzle tip 1, tail 1
- radius 20 mm, ≥ 30°: elsewhere 20, feet 2, leg roots 2, chest 1, ears 1, muzzle fold 1, muzzle tip 1, neck 1, tail 1; elsewhere at [(-36, -10, 62), (-36, 10, 62), (-32, -6, 52), (-32, 6, 52), (-22, -10, 102), (-22, 10, 102)]
- radius 20 mm, ≥ 45°: feet 2, chest 1, ears 1, muzzle tip 1, neck 1, tail 1
- radius 20 mm, ≥ 60°: feet 2, chest 1, ears 1, muzzle tip 1, neck 1, tail 1
- radius 20 mm, ≥ 90°: feet 2, ears 1, muzzle tip 1, neck 1, tail 1
- radius 20 mm, ≥ 120°: feet 2, ears 1, muzzle tip 1, neck 1, tail 1
- radius 20 mm, ≥ 180°: feet 2, ears 1, muzzle tip 1, neck 1, tail 1

## Dart candidates with the absolute sum, research R1's wording (`--absolute`)

| Fixture | radius | ≥ 30° | ≥ 45° | ≥ 60° | ≥ 90° | ≥ 120° | ≥ 180° |
|---|---|---|---|---|---|---|---|
| cube | 10 mm | 8 | 8 | 8 | 6 | 0 | 0 |
| cube | 15 mm | 8 | 8 | 8 | 6 | 0 | 0 |
| cube | 20 mm | 8 | 8 | 8 | 6 | 0 | 0 |
| sphere | 10 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| sphere | 15 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| sphere | 20 mm | 1 | 1 | 1 | 0 | 0 | 0 |
| capsule | 10 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| capsule | 15 mm | 2 | 0 | 0 | 0 | 0 | 0 |
| capsule | 20 mm | 2 | 2 | 2 | 2 | 0 | 0 |
| faceted | 10 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| faceted | 15 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| faceted | 20 mm | 0 | 0 | 0 | 0 | 0 | 0 |
| quadruped | 10 mm | 15 | 5 | 5 | 5 | 5 | 5 |
| quadruped | 15 mm | 1 | 1 | 1 | 1 | 1 | 1 |
| quadruped | 20 mm | 1 | 1 | 1 | 1 | 1 | 1 |

