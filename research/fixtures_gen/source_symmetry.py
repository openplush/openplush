"""Symmetry fixtures: a head with one mirror plane but a single tilted ear, and a lumpy head with no mirror plane.

Both are radial graphs over a latitude-longitude grid: each vertex is a unit direction scaled by an ellipsoid's radius
plus compactly supported bumps (a snout, ears, a lump). A radial graph is star-shaped, so it is closed, manifold and
free of self-intersections by construction. Written by generate.py; units are millimetres.
"""

import math
from dataclasses import dataclass

from generate import Face, Mesh, Vec
from source_seams import Primitive, glb_bytes

LONGITUDES = 32  # divisible by 4, so the mirror x -> -x maps grid longitudes onto grid longitudes
BANDS = 16
AXES = (40.0, 55.0, 45.0)  # ellipsoid semi-axes, all different, so the body alone has exactly three mirror planes
TILT = math.radians(5.0)  # the near-symmetric model's +x ear leans forward this much about x: small and local


@dataclass(frozen=True)
class Bump:
    """Raises the surface by up to `height` mm around `toward`, falling to zero at `reach` radians from it."""

    toward: Vec
    height: float
    reach: float

    def at(self, d: Vec) -> float:
        cos = max(-1.0, min(1.0, d[0] * self.toward[0] + d[1] * self.toward[1] + d[2] * self.toward[2]))
        t = math.acos(cos) / self.reach
        return self.height * (1.0 - t * t) ** 2 if t < 1.0 else 0.0


def unit(x: float, y: float, z: float) -> Vec:
    n = math.sqrt(x * x + y * y + z * z)
    return (x / n, y / n, z / n)


SNOUT = Bump(unit(0.0, 0.9, -0.25), 18.0, math.radians(35.0))
EAR = Bump(unit(0.55, -0.1, 0.83), 22.0, math.radians(22.0))  # the +x ear; its mirror is the -x ear


def mirrored(b: Bump) -> Bump:
    return Bump((-b.toward[0], b.toward[1], b.toward[2]), b.height, b.reach)


def tilted(b: Bump) -> Bump:
    """`b` rotated by TILT about the x axis, from +z towards +y."""
    x, y, z = b.toward
    c, s = math.cos(TILT), math.sin(TILT)
    return Bump((x, y * c + z * s, -y * s + z * c), b.height, b.reach)


def direction(phi: float, j: int) -> Vec:
    """The grid direction at polar angle `phi` and longitude `j`; the x < 0 half is the x > 0 half mirrored, bit for bit."""
    q = LONGITUDES // 4
    if q < j < 3 * q:
        x, y, z = direction(phi, (2 * q - j) % LONGITUDES)
        return (-x, y, z)
    t = 2 * math.pi * j / LONGITUDES
    return (0.0 if j in (q, 3 * q) else math.sin(phi) * math.cos(t), math.sin(phi) * math.sin(t), math.cos(phi))


def head(bumps: list[Bump]) -> Mesh:
    """The radial graph over the grid directions, triangulated so the x -> -x mirror maps the faces onto faces."""
    m = Mesh()

    def place(d: Vec) -> int:
        a, b, c = AXES
        r = 1.0 / math.sqrt((d[0] / a) ** 2 + (d[1] / b) ** 2 + (d[2] / c) ** 2) + sum(bump.at(d) for bump in bumps)
        return m.add((d[0] * r, d[1] * r, d[2] * r))

    north = place((0.0, 0.0, 1.0))
    rings = [[place(direction(math.pi * i / BANDS, j)) for j in range(LONGITUDES)] for i in range(1, BANDS)]
    south = place((0.0, 0.0, -1.0))
    q = LONGITUDES // 4
    for j in range(LONGITUDES):
        k = (j + 1) % LONGITUDES
        m.faces.append((north, rings[0][j], rings[0][k]))
        for up, down in zip(rings, rings[1:]):
            # The quad's diagonal flips across the plane x = 0, as a mirror flips it.
            quad: list[Face] = (
                [(up[j], down[j], down[k]), (up[j], down[k], up[k])]
                if j < q or j >= 3 * q
                else [(up[j], down[j], up[k]), (down[j], down[k], up[k])]
            )
            m.faces += quad
        m.faces.append((south, rings[-1][k], rings[-1][j]))
    return m


def symmetry_files() -> dict[str, bytes]:
    near = "near-symmetric: a head mirror-symmetric about x = 0 except its +x ear, tilted 5 degrees forward"
    asym = "asymmetric: a head with its snout turned aside, one ear and two broad lumps; no mirror plane"
    snout = Bump(unit(0.6, 0.75, -0.3), 25.0, math.radians(40.0))
    lumps = [Bump(unit(0.8, -0.3, 0.5), 25.0, math.radians(80.0)), Bump(unit(-0.4, -0.7, -0.6), 20.0, math.radians(70.0))]
    near_mesh = head([SNOUT, mirrored(EAR), tilted(EAR)])
    asym_mesh = head([snout, mirrored(EAR), *lumps])
    return {
        "meshes/near-symmetric.glb": glb_bytes("near-symmetric", near, near_mesh, [Primitive(near_mesh.faces)]),
        "meshes/asymmetric.glb": glb_bytes("asymmetric", asym, asym_mesh, [Primitive(asym_mesh.faces)]),
    }
