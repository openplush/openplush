# Spike H2: does monotonicity survive chaining and clustering? (H006)

**Status (2026-10-20): confirmed on the corpus.** Built once at the most permissive threshold and filtered per sensitivity, the crease chains and the dart clusters give a hint set that is a subset of the looser setting's in every one of 500 random ordered pairs per detector. Rebuilt per threshold, the same signals break the subset relation on the quadruped in 31 of 100 pairs for chains (20 by covered edges, not only by identity) and 80 of 100 for clusters. Nothing here ships; H012 is this check as a `proptest` over the Rust detectors, and H027 and H029 implement the build-once rule.

Non-shipping research code (research/AGENTS.md). Standard library only; `requirements.txt` is empty on purpose. The signals are imported from `../h1_default_thresholds/signals.py`.

```sh
python3 property.py          # the tables in results.md, about 4 s
python3 -m unittest -v       # 9 oracles, about 3 s
```

## The question

FR-006 promises that raising the sensitivity only adds hints and lowering it only removes them, per category, so a maker can slide the control without hints jumping around. Data-model section 3 derives a design rule from it: build the structure (crease chains, curvature clusters) once at the most permissive threshold and filter the result per sensitivity. H006 asks whether that rule is enough, and whether it is needed: does the build-once design actually hold on real meshes, and what goes wrong without it?

## Method

`monotone.py` turns the H1 signals into hints two ways. **Filtered**: chains built once at 10° (clusters at 30°), a hint kept at a setting when its length-weighted mean fold and its length (its peak curvature) reach the thresholds. **Rebuilt**: chains built from the edges that qualify at the setting's fold (clusters from the vertices that reach the setting's curvature), then filtered by length. A crease hint's identity is its sorted edge list; a dart hint's is its peak vertex.

`property.py` draws, with one seeded RNG, 100 ordered pairs of settings per fixture and detector, the first at least as strict as the second in every number, and counts the pairs where the strict hint set is not a subset of the loose one. For chains it also counts the pairs where the edges the strict hints cover are not a subset of the edges the loose hints cover: a weaker property a UI could live with if identity were all that broke.

`test_h2.py` holds the oracles: a 40°, 22°, 40° ridge whose middle edge drops out above 22°, a five-vertex strip whose 50, 70, 40, 80, 50 signal has a saddle at 40, and the corpus property itself at the seed `property.py` uses.

## Findings

- **Built once, filtered: monotone everywhere.** Zero violations in 500 pairs per detector. This is by construction: the hints are a fixed set and each filter is a monotone predicate of a fixed number per hint, so a hint passing the strict setting passes the loose one. The corpus run confirms the reference implementation has no hidden rebuild (a re-sort, a re-pairing at junctions) that would void the argument.
- **Rebuilt per threshold: not monotone, and not only by identity.** On the quadruped, 31 of 100 chain pairs fail by identity and 20 fail by coverage; the ridge oracle shows the mechanism: an edge between the two thresholds drops out, the chain splits, and the fragments are new hints that fail a length filter the whole chain passed. Even the smooth capsule and sphere, whose folds all sit between 10° and 25°, fail 5 to 7 pairs each. Clusters are worse: 80 of 100 pairs on the quadruped, because a stricter threshold cuts a cluster at a saddle and the second part's peak is a hint the looser setting never showed.
- **Coverage is not a usable fallback.** A UI that identified hints by the edges they cover would still see hints appear on tightening (20 of 100 pairs on the quadruped), so the design cannot relax to "the same edges, regrouped".
- **The cap must not truncate.** With build-once, SC-010's flood cap keeps monotonicity only if an over-cap category reports `flooded` and no hints, as data-model section 2 says; truncating to the first N would drop a hint on loosening when a new chain sorts ahead of it. Not measured here; stated so H023 tests it.

## What this means for H012 and the detectors

- H012's property test is well posed as written: random ordered sensitivity pairs, assert subset per category by hint identity. Identity must be stable across sensitivities, which build-once gives for free: the Rust `HintId` can be the chain's or cluster's index in the permissive structure.
- H027 and H029 must build at the permissive threshold and never rebuild on a sensitivity change; H033's "re-filter on sensitivity change only" is that rule at the pipeline level. Any later optimisation that rebuilds from a smaller edge set reintroduces the 31 % failure rate.
- The permissive thresholds (10° fold, 30° curvature) bound what any sensitivity can show. H016's mapping must not go below them, or hints would appear that no structure exists for.
