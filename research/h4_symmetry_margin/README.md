# Spike H4: the symmetry ambiguity margin (H-006)

**Status (2026-10-19): the recommendation below was accepted as H-010 in the 002 plan's Decision Ledger the same day.** Nothing here ships; H030 implements the rule.

Non-shipping research code (research/AGENTS.md). Standard library only; `requirements.txt` is empty on purpose.

```sh
python3 sweep.py          # the candidate and decision tables in results.md, about three minutes
python3 -m unittest -v    # 14 oracles, about 20 s
```

## The question

Research R1 and H-006 say the symmetry hint reports at most one plane, and **none** when "two or more candidates score within a small margin of each other", because picking one plane on a nearly radially symmetric animal would mislead. The spike asks what that margin is: how close must two candidate planes score before reporting none beats reporting the best?

## Method

- **Score**: the H002 oracle's definition (`research/fixtures_gen/test_symmetry.py`): RMS distance from each vertex's reflection in the plane to the surface, over the bounding-box diagonal. Zero for an exact mirror plane; the committed `near-symmetric.glb` scores 0.0035 about x = 0 and `asymmetric.glb`'s best plane 0.015, so the numbers here and in the fixture oracle compare directly.
- **Candidates**: the three area-weighted PCA planes (the design's starting set) plus the best three of a 60-normal Fibonacci sweep, each refined by a bounded pattern search over the normal **and the plane's offset**, then merged when their normals lie within 10° (as lines, so n and −n are one plane). "Second" below always means the second *distinct* plane.
- **Models**: the two committed heads; the H002 generator's head with upright ears at ±x, so x = 0 and y = 0 both mirror it exactly, plus a 2 mm lump at +x (breaks x = 0 only) and a snout of 0–12 mm at +y (breaks y = 0 only, by as much as its height says), which places the two candidate planes at any distance apart or tied; two bare spheroids; and seeded isotropic vertex jitter at 0.1–1 mm on the exact-plane head and on a rebuild of `near-symmetric`, to see what tessellation asymmetry or scan noise does to the scores.
- **Decision**: `mirror.decide`: none when the best candidate scores above the acceptance threshold; none ("ambiguous") when a second distinct plane both fits within the threshold and scores within the margin of the best; else the best. A margin equal to the threshold reduces the rule to "ambiguous whenever two distinct planes fit". The tables sweep the margin from 0 to the threshold with the threshold at 0.01, the H002 oracle's bound; H1 (H005) owns the threshold itself, and the recommendation is stated relative to it.

## Findings

Numbers in [`results.md`](results.md).

1. **The gap between two fitting planes is not a noise question.** Isotropic jitter of σ raises an exact plane's score by about 1.4 σ / diagonal (0.0008 at 0.1 mm, 0.0071 at 1 mm on a 179 mm head) and raises *every* fitting plane by the same amount: the two exact planes of the upright head stay within 0.0001 of each other at every σ. So noise does not reorder fitting candidates, and a margin chosen to absorb noise would be about zero. [solid on these models]
2. **A second fitting plane is a second valid mirror, and the score cannot say which is the sewing plane.** With a 2 mm lump at +x and a 5 mm snout at +y both planes fit (0.0015 and 0.0033) and the score prefers x = 0, which happens to be the plane an animal's snout lies in; with the snout at 2 mm they tie at 0.0014 / 0.0015 and no margin short of 0.0001 can tell them apart, yet neither answer is wrong as a mirror. The score ranks how exactly a plane mirrors the shape, not which of two mirrors the maker sews by. [solid]
3. **Exact ties exist in practice**: a spheroid, an egg, any body with two or three exact planes. On this tessellation an exact plane scores up to 0.0002, not 0, because the plane's offset is estimated from a centroid that moves with the triangulation (0.01 mm here), so a margin of exactly 0 would report a plane on a bare spheroid. The margin must be at least that floor. [solid]
4. **Where only one plane fits, no margin matters.** `near-symmetric.glb`: best 0.0032 about x = 0 (the refined plane sits 0.9° off x = 0 and beats the exact plane's 0.0035, because the tilted ear shifts the best fit), second 0.0147, above the threshold. Jittered to 1 mm it is still 0.0071 against 0.0155. `asymmetric.glb`: best 0.0150, nothing fits. Every margin from 0 to 0.01 gives the same verdicts on both. [solid]

## Recommendation [likely]

**Report none ("ambiguous: two planes fit") whenever a second distinct candidate plane also scores within the acceptance threshold; no separate margin constant.** Equivalently, the margin is the acceptance threshold itself.

- It is the only rule under which a tie and a near-tie are treated the same, which finding 2 says they must be, and it needs no number that H1's threshold tuning could later contradict.
- Its cost is confined to bodies with two fitting planes, which are the near-radially-symmetric ones H-006 already names as the case to refuse; a quadruped, a head with a snout, anything with a front, has one fitting plane and is unaffected (finding 4).
- The report should still list every fitting candidate with its score (data-model: `symmetry: Vec<SymmetryCandidate>`, best first), so a maker who knows which plane they want can pick it; the rule decides only what the tool offers unasked.

If the maintainer prefers a fixed margin below the threshold, the tables support **0.003** (a 5° ear tilt's worth of asymmetry on the committed fixture): it separates a 2 mm lump from an 8 mm snout and still calls the 5 mm snout ambiguous. Anything below 0.001 is within the tessellation floor and would call the bare spheroid's exact planes distinct.

## Limits

- Six candidate starts, so a continuum of exact planes (the 45 × 55 × 45 spheroid) shows as three distinct planes, not many; enough to be ambiguous, not a map of the continuum.
- The offset search's finest step is 0.125 mm, above the 0.01 mm centroid drift, so exact planes keep their 0.0002 floor; the floor is reported, not removed.
- The threshold 0.01 is the fixture oracle's bound, not a tuned value; H1 sets the real one, and the recommended rule is stated relative to it so it survives that tuning.
- Three synthetic heads and two spheroids; no real plush model yet (H001, H004).
