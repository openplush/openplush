"""H2: does FR-006's monotonicity survive chaining and clustering?

FR-006 promises that the hints at a lower sensitivity are a subset of the hints at a higher one, in every
category. Data-model section 3 makes that a design rule: build the structure (crease chains, curvature
clusters) once at the most permissive threshold and filter it per sensitivity, never rebuild it per threshold.
This module gives both ways of turning the H1 signals into hints, so property.py can count, on the corpus,
how often "stricter ⊆ looser" fails under each.

Standard library only; the signals are research/h1_default_thresholds/signals.py (the same definitions H028
and H029 implement in Rust).
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "h1_default_thresholds"))
from signals import Chain, Edge, Mesh, chains, clusters, integrated_curvature, signed_folds  # noqa: E402

PERMISSIVE_FOLD = 10.0  # degrees; H1's most permissive fold, where chains are built once
PERMISSIVE_CURVATURE = 30.0  # degrees; H1's most permissive summed angle defect, where clusters are built once
ChainHint = tuple[Edge, ...]  # a hint's identity: its edges, sorted


def ident(chain: Chain) -> ChainHint:
    return tuple(sorted(chain.edges))


def covered(hints: set[ChainHint]) -> set[Edge]:
    return {e for h in hints for e in h}


class CreaseHints:
    """Crease hints from the fold signal, at a (mean fold, minimum length) threshold pair."""

    def __init__(self, m: Mesh) -> None:
        self.m = m
        self.folds = signed_folds(m)
        self.once = chains(m, self.folds, self.qualifying(PERMISSIVE_FOLD))

    def qualifying(self, fold: float) -> list[Edge]:
        return [e for e, f in self.folds.items() if abs(f) >= fold]

    def filtered(self, fold: float, length: float) -> set[ChainHint]:
        """Data-model section 3: the chains built once, kept when both filters hold."""
        return {ident(c) for c in self.once if c.mean_fold >= fold and c.length_mm >= length}

    def rebuilt(self, fold: float, length: float) -> set[ChainHint]:
        """The alternative: chains rebuilt from the edges that qualify at this fold, then filtered by length."""
        return {ident(c) for c in chains(self.m, self.folds, self.qualifying(fold)) if c.length_mm >= length}


class DartHints:
    """Dart candidates from the integrated curvature, at a curvature threshold; a hint is its cluster's peak vertex."""

    def __init__(self, m: Mesh, radius_mm: float) -> None:
        self.m = m
        self.signal = integrated_curvature(m, radius_mm)
        self.once = clusters(m, self.signal, PERMISSIVE_CURVATURE)

    def filtered(self, threshold: float) -> set[int]:
        """Data-model section 3: the clusters built once, kept when the peak reaches the threshold."""
        return {peak for peak, value in self.once if value >= threshold}

    def rebuilt(self, threshold: float) -> set[int]:
        """The alternative: clusters rebuilt over the vertices that reach this threshold."""
        return {peak for peak, _ in clusters(self.m, self.signal, threshold)}
