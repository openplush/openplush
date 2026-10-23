# Quickstart: 002 seam-placement hints

**Feature**: `002-seam-placement-hints` | **Date**: 2026-10-13 | **Status**: Draft — describes the target state

Nothing here runs yet, and it cannot until 001's flatten and pattern stages work (plan D-008). This is the acceptance walkthrough: when every step produces what it claims, feature 002 is done.

## 0. Prerequisites

Same toolchain as 001. Additionally, these 001 tasks must be green:

| Needed from 001 | Why |
|---|---|
| 001 T069–T077 seams and charts | Accepting a hint creates a seam |
| 001 T078–T083 flatten | FR-031's preview is a flat-pattern preview |
| 001 T084–T087 pattern | Same |
| 001 T107 proposal preview-then-accept | Hints reuse it rather than duplicating it |
| 001 T144 curvature | The dart-candidate signal |
| 001 T035 fixtures | Nothing below is measurable without models |

## 1. Headless (Constitution II, FR-050)

```sh
# Every category at default sensitivity
openplush-cli hints fixtures/meshes/quadruped.glb --size-mm 300 --json out/hints.json

# One category, turned up
openplush-cli hints fixtures/meshes/quadruped.glb --size-mm 300 \
  --only concave-crease --sensitivity 0.8 --json out/concave.json
```

`out/hints.json` carries every hint with its category, location, strength, explanation, and decision state, plus the `absent`, `flooded`, and `orphaned_dismissals` lists.

### The oracles, as runnable checks

```sh
# SC-004: a smooth sphere has no creases. Must be zero, and must be REPORTED as absent
# rather than merely missing - an empty panel and a broken detector look identical.
openplush-cli hints fixtures/meshes/sphere.glb --size-mm 300 --json out/sphere.json
jq -e '[.hints[] | select(.category=="SharpEdge" or .category=="ConcaveCrease")] | length == 0' out/sphere.json
jq -e '.absent | index("SharpEdge")' out/sphere.json

# A cube has exactly 12 crease lines - no more, no fewer
openplush-cli hints fixtures/meshes/cube.glb --size-mm 300 --json out/cube.json
jq -e '[.hints[] | select(.category=="SharpEdge")] | length == 12' out/cube.json

# SC-005: symmetry found on a symmetric model, refused on an asymmetric one
jq -e '.hints[] | select(.category=="Symmetry") | .kind.match_score < 0.02' out/hints.json
openplush-cli hints fixtures/meshes/asymmetric.glb --size-mm 300 --json out/asym.json
jq -e '[.hints[] | select(.category=="Symmetry")] | length == 0' out/asym.json

# SC-007: every hint explains itself. Must print nothing.
jq -e '.hints[] | select(.explanation.why == null or .explanation.effect == null)' out/hints.json \
  && echo "UNEXPLAINED HINT" || echo "all hints explained"

# SC-008: determinism
openplush-cli hints fixtures/meshes/quadruped.glb --size-mm 300 --json out2/hints.json
cmp out/hints.json out2/hints.json && echo "deterministic"
```

### Monotonicity (FR-006)

The guarantee that raising sensitivity only ever adds hints. Checkable from the shell, and property-tested in `tests/monotonicity.rs`:

```sh
for s in 0.2 0.5 0.8; do
  openplush-cli hints fixtures/meshes/quadruped.glb --size-mm 300 \
    --only sharp-edge --sensitivity "$s" --json "out/s$s.json"
done
# every hint id at 0.2 must still be present at 0.5, and at 0.8
jq -e --slurpfile hi out/s0.5.json '[.hints[].id] - [$hi[0].hints[].id] | length == 0' out/s0.2.json
jq -e --slurpfile hi out/s0.8.json '[.hints[].id] - [$hi[0].hints[].id] | length == 0' out/s0.5.json
```

If either assertion fails, a threshold somewhere is adaptive rather than absolute — see plan D-003, which exists specifically to prevent this.

### Physical thresholds (FR-007, FR-040)

The same model at two sizes should give different hints, because three of the five thresholds are in millimetres:

```sh
openplush-cli hints fixtures/meshes/quadruped.glb --size-mm 80  --json out/small.json
openplush-cli hints fixtures/meshes/quadruped.glb --size-mm 600 --json out/large.json
# The 80 mm toy should show fewer crease hints: more of them fall under the length filter.
```

A tool where these two files are identical has scale-independent thresholds and FR-007 is not implemented, however plausible the output looks.

## 2. Web app

```sh
pnpm --filter web dev
pnpm --filter web exec playwright test hints
```

Manual walkthrough matching US1 and US2:

1. Import `fixtures/meshes/quadruped.glb`, accept repairs, set height 300 mm.
2. Open the hints panel. Four categories with counts; the overlay marks the muzzle fold, the neck crease, the head dome.
3. Click a list entry. The view moves to it and the overlay element is distinguished.
4. Read the explanation. It says why it was flagged and what a seam there would do — in that order, in plain language.
5. Turn concave-crease sensitivity down. Hints disappear. Turn it back up: **the same hints return, plus more.** None of the originals is missing.
6. Accept the neck crease. A preview shows the flat-pattern effect. Keep it — it becomes an ordinary seam, editable like any hand-drawn one.
7. Dismiss a hint. It leaves the list. Save, reload: still dismissed.
8. Change the size to 150 mm. Hints recompute, and a summary says what changed.
9. Load `fixtures/meshes/sphere.glb`. The crease categories say they found nothing, rather than looking broken.

Step 5 is the one most likely to be quietly wrong, and the one a maker will notice fastest — a sensitivity slider that loses the hint you were examining feels broken even when every individual hint is correct.

## 3. What "done" means

| Check | Where |
|---|---|
| 70% of hand-placed seam length within 5 mm of a hint | SC-001 — **blocked on fixtures with hand-made patterns** |
| First seam placed 30% faster with hints on (n ≥ 5) | SC-002 — needs recruiting |
| Hints within 5 s at 50k triangles; re-filter within 1 s | SC-003 |
| Smooth sphere: zero crease hints at default sensitivity | SC-004 |
| Symmetry correct on every fixture, no false planes | SC-005 |
| Accepting any hint yields a valid seam or dart, 100% | SC-006 |
| Every hint carries an explanation | SC-007 |
| Byte-identical hint report across runs and platforms | SC-008 |
| Zero network requests | SC-009 |
| No category over 50 hints on faceted fixtures at default | SC-010 |

**SC-001 is the honest hole.** It is the feature's headline claim and it cannot be measured until `fixtures/` contains models shipping with known-good hand-made patterns. That is fixture-lane work, not code. Until then the feature can be correct, deterministic, and fully tested while its central premise — that these hints match where experienced makers actually cut — remains unverified. Worth saying out loud rather than discovering at release.

## 4. Fixtures this feature needs

```
fixtures/meshes/
├── quadruped.glb       symmetric animal: muzzle fold, neck crease, head dome
├── asymmetric.glb      deliberately no mirror plane - proves refusal works
├── near-symmetric.glb  one ear tilted - proves the match score is meaningful
├── faceted.glb         icosphere or low-poly - proves the flood cap works
└── sphere.glb          smooth - proves zero is reportable (shared with 001)
```

Plus, for SC-001, any model that arrives with a known-good hand-made pattern. Those are worth more than the rest combined: they are the only way to measure whether this feature does what it claims.
