"""The H2 property check: random threshold pairs on the corpus, counting where "stricter ⊆ looser" fails.

Run: python3 property.py   (from this directory; about 20 s, standard library only)
Deterministic: one seeded RNG, every table sorted. For each fixture and each of N random pairs of threshold
settings, ordered so the first is at least as strict as the second in every number, the hints at the strict
setting must be a subset of the hints at the loose one. Counted for the design the data model prescribes
(build once, filter) and for the alternative (rebuild per threshold), by hint identity and, for creases, by
the edges the hints cover.
"""

import random
import sys
from pathlib import Path

from monotone import CreaseHints, DartHints, covered

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "h1_default_thresholds"))
from sweep import load  # noqa: E402

SEED = 2026_10_20
PAIRS = 100
FOLD = (10.0, 60.0)  # degrees; the sampled range of the mean-fold threshold
LENGTH = (0.0, 60.0)  # mm; the sampled range of the minimum chain length
CURVATURE = (30.0, 240.0)  # degrees; the sampled range of the dart threshold
RADIUS_MM = 15.0  # H-011's integration radius
Counts = dict[str, int]


def ordered(rng: random.Random, lo: float, hi: float) -> tuple[float, float]:
    """Two draws, strict first (the higher threshold yields fewer hints)."""
    a, b = rng.uniform(lo, hi), rng.uniform(lo, hi)
    return max(a, b), min(a, b)


def crease_counts(hints: CreaseHints, rng: random.Random) -> Counts:
    counts: Counts = {"filtered identity": 0, "rebuilt identity": 0, "rebuilt coverage": 0}
    for _ in range(PAIRS):
        (strict_fold, loose_fold), (strict_len, loose_len) = ordered(rng, *FOLD), ordered(rng, *LENGTH)
        if not hints.filtered(strict_fold, strict_len) <= hints.filtered(loose_fold, loose_len):
            counts["filtered identity"] += 1
        strict, loose = hints.rebuilt(strict_fold, strict_len), hints.rebuilt(loose_fold, loose_len)
        if not strict <= loose:
            counts["rebuilt identity"] += 1
        if not covered(strict) <= covered(loose):
            counts["rebuilt coverage"] += 1
    return counts


def dart_counts(hints: DartHints, rng: random.Random) -> Counts:
    counts: Counts = {"filtered identity": 0, "rebuilt identity": 0}
    for _ in range(PAIRS):
        strict, loose = ordered(rng, *CURVATURE)
        if not hints.filtered(strict) <= hints.filtered(loose):
            counts["filtered identity"] += 1
        if not hints.rebuilt(strict) <= hints.rebuilt(loose):
            counts["rebuilt identity"] += 1
    return counts


def table(title: str, rows: dict[str, Counts]) -> None:
    columns = list(next(iter(rows.values())))
    print(f"\n### {title}: violations in {PAIRS} ordered pairs\n")
    print("| fixture | " + " | ".join(columns) + " |")
    print("|---|" + "---|" * len(columns))
    for name in sorted(rows):
        print(f"| {name} | " + " | ".join(str(rows[name][c]) for c in columns) + " |")


def main() -> None:
    rng = random.Random(SEED)
    meshes = load()
    creases = {name: crease_counts(CreaseHints(m), rng) for name, m in sorted(meshes.items())}
    darts = {name: dart_counts(DartHints(m, RADIUS_MM), rng) for name, m in sorted(meshes.items())}
    table("Crease chains, (mean fold, minimum length) pairs", creases)
    table(f"Dart clusters at {RADIUS_MM:g} mm, curvature pairs", darts)


if __name__ == "__main__":
    main()
