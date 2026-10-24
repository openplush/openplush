# Spike 001 T205: a reproducible stretch test for research R7's fabric presets

**Status (2026-10-23): written. Nothing has been measured yet. Under D-075 the presets take their numbers from supplier spec sheets or the Fabric Stretch Guide; this protocol is how a maker or the maintainer replaces one of those numbers with a measurement.**

Non-shipping research. Nothing in `crates/`, `apps/` or `packages/` depends on it.

## What it measures

Research R7 records two stretch figures per fabric: along the grain (lengthwise, parallel to the selvage) and across it (crosswise). Each is the extension at a fixed load, as a percentage of the unloaded length. The only consumer in 001 is FR-058's mismatch warning, which compares two fabrics per direction against D-046's 20-point threshold. A test therefore only has to separate fabrics that differ by 20 points, not resolve 1 point.

Bias is not tested. No piece is placed on the bias (D-046), so 001 uses none of R7's bias figures.

## Sources

| Ref | Source | What it supplies |
|---|---|---|
| [S1] | EN ISO 20932-1:2020, *Textiles — Determination of the elasticity of fabrics — Part 1: Strip tests* (identical to ISO 20932-1:2018, confirmed 2024; replaces EN 14704-1:2005), read through the Fabric Stretch Guide's "Test conditions assumed" (the maintainer's copy, 2026-10-23) | The two specimen forms, the load left to agreement, the 5 cycles and the 30 min rest |
| [S2] | Bureau Veritas CPS, "BS EN ISO 20932-1: textiles, determination of elasticity of fabrics", <https://www.cps.bureauveritas.com/newsroom/bs-en-iso-20932-1-textiles-determination-elasticity-fabrics>, read 2026-10-23 | Pretension values, a secondary summary of [S1] |
| [S3] | Fabric Stretch Guide, "Test conditions assumed" and "Stretch test your fabric" panels (the maintainer's copy, 2026-10-23) | The common-practice loads, each fabric row's method and load, and the 10 cm home ruler test |

The guide is an uncommitted copy with no stated licence, so the passage this protocol relies on is quoted here:

> Method A is a 50 mm flat strip; Method B is a 75 mm sewn loop. The standard allows either method for any fabric; using B for knits is a lab habit. The standard leaves the load to be agreed. Common practice is about 6 N/cm for wovens and 3–4 N/cm for knits, applied over 5 cycles. A different load or method gives different numbers.

Its per-fabric rows give each fabric's method and load in a "Test" column; the rows this protocol uses read:

| Guide row | Test |
|---|---|
| Quilting cotton | A, 6 N/cm |
| Felt (wool or acrylic, nonwoven) | A, 6 N/cm |
| Polar fleece | B, 3–4 N/cm |
| Minky (plush cuddle fleece) | B, 3–4 N/cm |
| Velboa / short plush | B, 3–4 N/cm |
| Faux fur (long-pile) | A, 6 N/cm |

The standard's full text could not be fetched from this environment. Every value below that [S1] does not give through the guide is marked `[verify]` against the full text and is not used until it is checked.

## Lab method (the standard)

| Parameter | Value | Source |
|---|---|---|
| Method A specimen | Flat strip, 50 mm wide | [S1] |
| Method B specimen | Sewn loop, 75 mm | [S1]. The standard allows either method for any fabric; B for knits is lab habit, not a rule |
| Load | Agreed between the parties [S1]. Common practice is about 6 N/cm of specimen width for wovens and 3–4 N/cm for knits [S3] | [S1], [S3] |
| Cycles | 5 load cycles | [S1] |
| Recovery | Measured after a rest of about 30 min following the fifth cycle | [S1] |
| Pretension, Method A | 1 cN/cm woven, 0.5 cN/cm knitted | [S2] `[verify]` |
| Pretension, Method B | 2 cN/cm woven, 1 cN/cm knitted | [S2] `[verify]` |
| Gauge length, extension rate | Not available from [S1] or [S2] | `[verify]` |
| Cycle whose extension is reported | Not available from [S1] or [S2]; record all five | `[verify]` |

A different load or method gives different numbers [S3], so the same load and method must be used on both fabrics of a comparison. The guide's per-fabric rows [S3] name Method B at 3–4 N/cm for fleece, minky and velboa, and Method A at 6 N/cm for quilting cotton, felt and long-pile faux fur; T008 records each row in R7.

## Home method (no tensile tester)

Use this to check or replace a preset. It reproduces the standard's load per centimetre on a 50 mm strip with a hanging weight. The weights are this protocol's arithmetic from the common-practice load [S3], not values from the standard.

| Fabric | Load [S3] | On a 50 mm strip | Hanging mass (÷ 9.81 m/s²) |
|---|---|---|---|
| Woven, felt, long-pile faux fur | 6 N/cm | 30 N | 3.1 kg |
| Knit, minky, velboa, fleece | 3–4 N/cm | 15–20 N | 1.5–2.0 kg; use 1.8 kg (3.5 N/cm) |

### Cutting and marking

1. Lay the fabric flat, wrong side up, with no tension. Find the grain from the selvage: lengthwise runs parallel to it, crosswise at 90°.
2. Cut two strips per direction, 50 mm wide and 250 mm long, at least 100 mm in from the selvage. Lengthwise strips run parallel to the selvage; crosswise strips run square to it. Cut pile fabrics from the back with the tips of the scissors, so the pile is not cut.
3. On each strip, mark two lines across its width 100 mm apart, centred along its length, with a fine fabric pen. The 75 mm either side holds the clamps.
4. Label each strip on the back with the fabric name, direction (L or C) and strip number.

The 100 mm gauge matches the ruler test's 10 cm [S3]. The 250 mm length, the 75 mm clamp ends and the 100 mm selvage margin are this protocol's choices, so the clamps sit clear of the marks.

### Running it

1. Clamp the top 75 mm of the strip, using two binder clips across its full width, and hang it straight.
2. Clip the weight to the bottom 75 mm across the full width. Lower it by hand so it doesn't drop.
3. Hold for 1 min (this protocol's choice), then measure between the two marks to the nearest millimetre. Lift the weight off.
4. Repeat for 5 cycles in total [S1] and record each.
5. Lay the strip flat for 30 min [S1] and measure the marks again for recovery.

Stretch % = (loaded length − 100 mm) ÷ 100 mm × 100. Recovery % = (loaded length − rested length) ÷ (loaded length − 100 mm) × 100.

### Quick ruler test [S3]

Mark 10 cm across the grain and stretch it gently by hand until it starts to resist, then measure. It is gentler than the lab load, so it reads somewhat lower [S3]. Use it only to tell whether a fabric is near a preset, never to replace one.

## Recording sheet

Copy one per fabric.

```
Fabric:                         Supplier / product code:
Category (woven/knit/pile/felt): Pile length (mm):
Date:                           Tester:
Method: home 50 mm strip / Method A / Method B      Load (N/cm):      Mass (kg):

Direction  Strip  Cycle1  Cycle2  Cycle3  Cycle4  Cycle5  Rested 30 min
                  (loaded length between marks, mm; unloaded = 100)
L          1
L          2
C          1
C          2

Stretch %, L (mean, cycle 5 until [S1] says which): Stretch %, C:
Recovery %, L:                        Recovery %, C:
Notes (curling, pile shedding, clamp slip):
```

A strip that slips in a clamp, tears, or curls so the marks can't be read is discarded and recut, not averaged in.

## Open

- Gauge length, extension rate, pretension and the reported cycle, all `[verify]` against the full text of [S1].
- The 1 min hold in the home method is this protocol's choice, not the standard's.
