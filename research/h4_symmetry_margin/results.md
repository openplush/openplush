# Spike H4: results

Run on 2026-10-19, Linux x86_64, standard library only. `python3 sweep.py` prints the two tables below; the output is byte-identical on CPython 3.11.17, 3.12.3 and 3.13.16 (the committed tables are the 3.13 run). `python3 -m unittest -v`: 14 tests, OK, on all three. `mypy --strict --python-version 3.11 mirror.py variants.py sweep.py test_h4.py` (mypy 2.3.1, `MYPYPATH=../fixtures_gen`): no issues.

Scores are RMS reflected-vertex-to-surface distance over the bounding-box diagonal (179 mm for the heads). The acceptance threshold is 0.01, the H002 oracle's bound. What the rows are and what they show is in [`README.md`](README.md).

Timings: one score 11 ms, one full candidate search 8 s, the whole sweep 3 min, on one core.

## Candidates

Score, normal, offset from the volume centroid along it. Gap is second minus best.

| Model | Distinct | Best | Second | Third | Gap |
|---|---|---|---|---|---|
| `near-symmetric.glb` | 3 | 0.0032 (-1.000, -0.015, -0.003) +0.00 mm | 0.0147 (+0.012, -1.000, -0.028) +1.50 mm | 0.0187 (-0.001, +0.199, +0.980) -0.50 mm | 0.0115 |
| `asymmetric.glb` | 4 | 0.0150 (+0.334, +0.911, +0.242) +0.00 mm | 0.0156 (+0.422, -0.777, +0.467) +1.00 mm | 0.0167 (+0.681, +0.081, +0.728) +1.12 mm | 0.0007 |
| upright ears, no lump, no snout | 3 | 0.0000 (+1.000, -0.000, +0.000) +0.00 mm | 0.0002 (+0.000, +1.000, -0.002) +0.00 mm | 0.0190 (+0.000, +0.007, -1.000) +0.75 mm | 0.0002 |
| lump 2 mm at +x, snout 0 mm at +y | 3 | 0.0002 (-0.000, +1.000, -0.002) +0.00 mm | 0.0015 (+1.000, -0.000, +0.001) +0.00 mm | 0.0189 (+0.002, +0.007, -1.000) +0.75 mm | 0.0012 |
| lump 2 mm at +x, snout 1 mm at +y | 3 | 0.0007 (-0.000, +1.000, +0.002) +0.00 mm | 0.0015 (+1.000, +0.000, +0.001) +0.00 mm | 0.0189 (+0.008, +0.004, -1.000) +0.62 mm | 0.0008 |
| lump 2 mm at +x, snout 2 mm at +y | 3 | 0.0014 (-0.000, +1.000, +0.006) +0.00 mm | 0.0015 (+1.000, +0.000, +0.001) +0.00 mm | 0.0188 (-0.011, -0.007, -1.000) +0.75 mm | 0.0001 |
| lump 2 mm at +x, snout 3 mm at +y | 3 | 0.0015 (+1.000, +0.000, +0.001) +0.00 mm | 0.0020 (-0.000, +1.000, +0.005) -0.12 mm | 0.0187 (-0.010, -0.008, -1.000) +0.75 mm | 0.0006 |
| lump 2 mm at +x, snout 5 mm at +y | 3 | 0.0015 (+1.000, +0.000, +0.001) +0.00 mm | 0.0033 (-0.000, +1.000, +0.008) -0.12 mm | 0.0186 (-0.010, -0.010, -1.000) +0.75 mm | 0.0018 |
| lump 2 mm at +x, snout 8 mm at +y | 3 | 0.0014 (+1.000, +0.000, +0.001) +0.00 mm | 0.0051 (+0.003, -1.000, -0.009) +0.25 mm | 0.0184 (+0.011, +0.007, +1.000) -0.75 mm | 0.0037 |
| lump 2 mm at +x, snout 12 mm at +y | 3 | 0.0014 (+1.000, +0.000, +0.001) +0.00 mm | 0.0073 (+0.002, -1.000, -0.015) +0.50 mm | 0.0181 (+0.011, +0.008, +1.000) -0.62 mm | 0.0059 |
| spheroid 45 × 55 × 45 mm | 3 | 0.0000 (+1.000, +0.000, +0.000) +0.00 mm | 0.0002 (-0.003, +0.002, +1.000) +0.00 mm | 0.0002 (-0.000, +1.000, +0.003) +0.00 mm | 0.0002 |
| spheroid 40 × 55 × 45 mm | 3 | 0.0000 (+1.000, -0.000, -0.000) +0.00 mm | 0.0002 (+0.000, -0.002, +1.000) +0.00 mm | 0.0002 (+0.000, +1.000, +0.003) +0.00 mm | 0.0002 |
| upright ears, jitter σ = 0.10 mm | 3 | 0.0008 (+1.000, +0.000, -0.001) +0.00 mm | 0.0008 (+0.000, +1.000, -0.003) +0.00 mm | 0.0191 (-0.014, -0.007, -1.000) +0.75 mm | 0.0000 |
| upright ears, jitter σ = 0.25 mm | 3 | 0.0020 (+1.000, -0.001, +0.000) +0.00 mm | 0.0020 (+0.001, +1.000, -0.003) +0.00 mm | 0.0191 (-0.013, -0.009, -1.000) +0.75 mm | 0.0000 |
| upright ears, jitter σ = 0.50 mm | 3 | 0.0038 (-1.000, -0.004, -0.003) +0.00 mm | 0.0039 (-0.011, +1.000, -0.003) +0.00 mm | 0.0191 (-0.024, -0.029, -0.999) +0.75 mm | 0.0001 |
| upright ears, jitter σ = 1.00 mm | 3 | 0.0071 (-1.000, -0.004, -0.009) -0.12 mm | 0.0072 (+0.015, -1.000, -0.002) +0.00 mm | 0.0192 (-0.022, -0.026, -0.999) +0.75 mm | 0.0001 |
| near-symmetric rebuilt, jitter σ = 0.50 mm | 3 | 0.0045 (+1.000, +0.017, +0.008) +0.00 mm | 0.0149 (+0.016, -1.000, -0.025) +1.50 mm | 0.0189 (+0.001, +0.211, +0.977) -0.50 mm | 0.0104 |
| near-symmetric rebuilt, jitter σ = 1.00 mm | 3 | 0.0071 (+1.000, +0.012, +0.010) +0.00 mm | 0.0155 (+0.012, -1.000, -0.023) +1.38 mm | 0.0190 (+0.085, +0.217, +0.972) -0.50 mm | 0.0084 |

## Decisions, accept = 0.01

| Model | margin 0.0 | margin 0.001 | margin 0.002 | margin 0.003 | margin 0.005 | margin 0.01 |
|---|---|---|---|---|---|---|
| `near-symmetric.glb` | plane | plane | plane | plane | plane | plane |
| `asymmetric.glb` | none: no plane fits | none: no plane fits | none: no plane fits | none: no plane fits | none: no plane fits | none: no plane fits |
| upright ears, no lump, no snout | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 0 mm at +y | plane | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 1 mm at +y | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 2 mm at +y | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 3 mm at +y | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 5 mm at +y | plane | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 8 mm at +y | plane | plane | plane | plane | none: ambiguous | none: ambiguous |
| lump 2 mm at +x, snout 12 mm at +y | plane | plane | plane | plane | plane | none: ambiguous |
| spheroid 45 × 55 × 45 mm | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| spheroid 40 × 55 × 45 mm | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| upright ears, jitter σ = 0.10 mm | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| upright ears, jitter σ = 0.25 mm | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| upright ears, jitter σ = 0.50 mm | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| upright ears, jitter σ = 1.00 mm | plane | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous | none: ambiguous |
| near-symmetric rebuilt, jitter σ = 0.50 mm | plane | plane | plane | plane | plane | plane |
| near-symmetric rebuilt, jitter σ = 1.00 mm | plane | plane | plane | plane | plane | plane |
