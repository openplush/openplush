"""Faceted fixture: a frequency-2 icosphere, the coarsest icosphere with more hard edges than the hint panel lists.

An icosahedron has 30 edges, under the 50-hint cap; one midpoint subdivision gives 120, each folded 18 to 22.5 degrees
and 33 to 37 mm long at this radius. Written by generate.py; units are millimetres.
"""

import math

from generate import Edge, Face, Mesh, Vec
from source_seams import Primitive, glb_bytes

RADIUS = 60.0
PHI = (1 + math.sqrt(5)) / 2
ICOSAHEDRON: list[Face] = [
    (0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11),
    (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
    (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9),
    (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1),
]  # fmt: skip


def on_sphere(p: Vec) -> Vec:
    n = math.sqrt(p[0] ** 2 + p[1] ** 2 + p[2] ** 2)
    return (RADIUS * p[0] / n, RADIUS * p[1] / n, RADIUS * p[2] / n)


def icosphere() -> Mesh:
    """Each icosahedron face split into four at its edge midpoints, every vertex pushed out to RADIUS."""
    m = Mesh()
    for x, y in ((-1.0, PHI), (1.0, PHI), (-1.0, -PHI), (1.0, -PHI)):
        m.add(on_sphere((x, y, 0.0)))
    for x, y in ((-1.0, PHI), (1.0, PHI), (-1.0, -PHI), (1.0, -PHI)):
        m.add(on_sphere((0.0, x, y)))
    for x, y in ((-1.0, PHI), (1.0, PHI), (-1.0, -PHI), (1.0, -PHI)):
        m.add(on_sphere((y, 0.0, x)))
    mid: dict[Edge, int] = {}

    def midpoint(a: int, b: int) -> int:
        key = (min(a, b), max(a, b))
        if key not in mid:
            p, q = m.pos[a], m.pos[b]
            mid[key] = m.add(on_sphere(((p[0] + q[0]) / 2, (p[1] + q[1]) / 2, (p[2] + q[2]) / 2)))
        return mid[key]

    for a, b, c in ICOSAHEDRON:
        ab, bc, ca = midpoint(a, b), midpoint(b, c), midpoint(c, a)
        m.faces += [(a, ab, ca), (b, bc, ab), (c, ca, bc), (ab, bc, ca)]
    return m


def faceted_files() -> dict[str, bytes]:
    note = "faceted: a frequency-2 icosphere of radius 60 mm; all 120 edges fold 18 to 22.5 degrees"
    m = icosphere()
    return {"meshes/faceted.glb": glb_bytes("faceted", note, m, [Primitive(m.faces)])}
