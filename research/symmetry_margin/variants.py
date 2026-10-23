"""In-memory models for the margin sweep, built with the 002 T002 fixture generator so they share its head and ears.

None of these is a committed fixture: they exist only to place a second candidate plane at a chosen distance
from the first, or to lift an exact plane's score with tessellation-like noise, which the two committed heads
cannot do on their own.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "fixtures_gen"))

import source_symmetry as src  # noqa: E402
from source_symmetry import EAR, SNOUT, Bump, head, mirrored, tilted, unit  # noqa: E402

Vec = tuple[float, float, float]
Face = tuple[int, int, int]
Model = tuple[list[Vec], list[Face]]

EARS = [Bump(unit(0.55, 0.0, 0.83), 22.0, math.radians(22.0)), Bump(unit(-0.55, 0.0, 0.83), 22.0, math.radians(22.0))]
REACH = math.radians(30.0)


def two_planes(lump_mm: float, snout_mm: float) -> Model:
    """Upright ears at ±x, so x = 0 and y = 0 both mirror the head exactly, then a lump at +x of the first height
    and a snout at +y of the second: the lump breaks only x = 0 and the snout only y = 0, each by as much as its
    height says, so the two candidate planes can be placed at any distance apart or tied."""
    bumps = EARS + [Bump((1.0, 0.0, 0.0), lump_mm, REACH), Bump((0.0, 1.0, 0.0), snout_mm, REACH)]
    m = head([b for b in bumps if b.height > 0])
    return m.pos, m.faces


def spheroid(axes: tuple[float, float, float]) -> Model:
    """The bare ellipsoid: three exact planes, and a continuum of them when two semi-axes are equal."""
    saved = src.AXES
    src.AXES = axes
    try:
        m = head([])
    finally:
        src.AXES = saved
    return m.pos, m.faces


def near_symmetric_fixture() -> Model:
    """The committed near-symmetric head rebuilt in memory (float64, so not bit-identical to the GLB's float32)."""
    m = head([SNOUT, mirrored(EAR), tilted(EAR)])
    return m.pos, m.faces


def jittered(model: Model, sigma_mm: float, seed: int) -> Model:
    """Every vertex moved by an isotropic Gaussian of the given deviation, seeded, so a symmetric shape with an
    asymmetric tessellation or scan noise can be imitated at a chosen amplitude."""
    rng = random.Random(seed)

    def moved(p: Vec) -> Vec:
        return (p[0] + rng.gauss(0.0, sigma_mm), p[1] + rng.gauss(0.0, sigma_mm), p[2] + rng.gauss(0.0, sigma_mm))

    pos = [moved(p) for p in model[0]]
    return pos, list(model[1])
