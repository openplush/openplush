"""Quadruped fixture: a symmetric four-legged animal with a muzzle fold, a neck crease and a head dome.

The shape is a union of signed-distance primitives: an ellipsoid body and belly, capsule neck, sphere head and
capsule muzzle joined hard (pointwise minimum), so the neck and the muzzle fold are true concave creases, and two
ear capsules, four leg capsules and a tail blended in smoothly, so the only creases on the model are the two the
hints are tuned on. A smooth blend would round the neck off; a hard join at every limb would add creases nobody
asked for and, on a grid mesher, zigzags along them.
The surface is meshed by dual contouring (Ju, Losasso, Schaefer, Warren 2002) on a grid symmetric about x = 0:
one vertex per cell the surface crosses, placed where the tangent planes at the cell's edge crossings meet, so a
vertex lands on a crease when one passes through the cell and the joins stay as sharp as the solids make them;
each cell at x < 0 takes the mirror image of its twin's vertex, so the model is mirror-symmetric bit for bit.
Marching tetrahedra, tried first, left two thousand sliver folds over 40 degrees on a smooth surface, and plain
surface nets spread every join over two edges. Written by generate.py; units are millimetres, z up, snout to +y.
"""

import math
from dataclasses import dataclass

from generate import Mesh, Vec
from source_seams import Primitive, glb_bytes

CELL = 4.0  # mm; well under the thinnest feature's radius (ears, 7.5 mm), so no cell holds two sheets
X_CELLS, Y_RANGE, Z_RANGE = 12, (-108.0, 136.0), (-16.0, 164.0)  # cells each side of x = 0; y and z extents
ROUND = 6  # decimals kept, as for every generated fixture
CUBE_EDGES = tuple((a, b) for a in range(8) for b in range(a + 1, 8) if bin(a ^ b).count("1") == 1)
GRADIENT_STEP = 0.01  # mm; central differences of the field, far below the cell and above float noise
MASS_WEIGHT = 0.1  # pull towards the crossings' mean, so a cell with only parallel tangent planes still has one answer
MARGIN = 0.1  # of a cell; a vertex stays this far inside its cell, so two cells' vertices never nearly coincide
BLEND = 4.0  # mm; the smooth-union radius for limbs, ears and tail
CREASE_DOT = math.cos(math.radians(25.0))  # normals farther apart than this mark a cell that holds a crease

Index = tuple[int, int, int]


@dataclass(frozen=True)
class Capsule:
    a: Vec
    b: Vec
    r: float

    def at(self, p: Vec) -> float:
        ab = (self.b[0] - self.a[0], self.b[1] - self.a[1], self.b[2] - self.a[2])
        ap = (p[0] - self.a[0], p[1] - self.a[1], p[2] - self.a[2])
        along = (ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / (ab[0] ** 2 + ab[1] ** 2 + ab[2] ** 2)
        t = max(0.0, min(1.0, along))
        d = (ap[0] - t * ab[0], ap[1] - t * ab[1], ap[2] - t * ab[2])
        return math.sqrt(d[0] ** 2 + d[1] ** 2 + d[2] ** 2) - self.r


@dataclass(frozen=True)
class Ellipsoid:
    """Negative inside; an approximate distance (exact for a sphere), which the iso-surface does not depend on."""

    c: Vec
    axes: Vec

    def at(self, p: Vec) -> float:
        k = math.sqrt(sum(((p[i] - self.c[i]) / self.axes[i]) ** 2 for i in range(3)))
        return (k - 1.0) * min(self.axes)


Solid = Capsule | Ellipsoid

BODY = Ellipsoid((0.0, 0.0, 72.5), (34.5, 71.5, 38.5))
BELLY = Ellipsoid((0.0, 0.0, 62.5), (36.5, 62.5, 24.5))  # flattens the underside, so each leg joins it at a wide angle
NECK = Capsule((0.0, 58.0, 92.0), (0.0, 76.0, 106.0), 15.0)
HEAD = Ellipsoid((0.0, 84.5, 114.5), (29.0, 29.0, 29.0))
MUZZLE = Capsule((0.0, 100.0, 106.0), (0.0, 118.0, 103.0), 12.5)
TAIL = Capsule((0.0, -66.0, 84.0), (0.0, -96.0, 100.0), 7.5)


def mirrored_pair(x: float, a: Vec, b: Vec, r: float) -> list[Solid]:
    return [Capsule((x, a[1], a[2]), (x, b[1], b[2]), r), Capsule((-x, a[1], a[2]), (-x, b[1], b[2]), r)]


CORE: list[Solid] = [BODY, BELLY, NECK, HEAD, MUZZLE]


def limbs() -> list[Solid]:
    ears = mirrored_pair(13.0, (0.0, 78.0, 132.0), (0.0, 74.0, 150.0), 7.5)
    legs = [s for y in (41.0, -41.0) for s in mirrored_pair(18.0, (0.0, y, 46.0), (0.0, y, 9.0), 10.0)]
    return [TAIL, *ears, *legs]


@dataclass(frozen=True)
class Animal:
    """The core joined hard, then the limbs blended in by a log-sum-exp smooth minimum of radius BLEND."""

    core: list[Solid]
    limbs: list[Solid]

    def at(self, p: Vec) -> float:
        values = [min(s.at(p) for s in self.core)] + [s.at(p) for s in self.limbs]
        low = min(values)
        return low - BLEND * math.log(sum(math.exp((low - v) / BLEND) for v in values))


def field(p: Vec, animal: Animal) -> float:
    """Negative inside the animal."""
    return animal.at(p)


def gradient(p: Vec, animal: Animal) -> Vec:
    """Unit outward normal of the field at `p`; on a crease it is one side's, which is what a crease vertex needs."""
    g = []
    for d in range(3):
        lo = list(p)
        hi = list(p)
        lo[d] -= GRADIENT_STEP
        hi[d] += GRADIENT_STEP
        g.append(field((hi[0], hi[1], hi[2]), animal) - field((lo[0], lo[1], lo[2]), animal))
    n = math.sqrt(g[0] ** 2 + g[1] ** 2 + g[2] ** 2)
    return (g[0] / n, g[1] / n, g[2] / n)


def solve3(a: list[list[float]], b: list[float]) -> Vec:
    """Gaussian elimination with partial pivoting on a 3 x 3 system; deterministic, no library."""
    m = [a[i] + [b[i]] for i in range(3)]
    for col in range(3):
        pivot = max(range(col, 3), key=lambda r: abs(m[r][col]))
        m[col], m[pivot] = m[pivot], m[col]
        for r in range(3):
            if r != col:
                f = m[r][col] / m[col][col]
                m[r] = [x - f * y for x, y in zip(m[r], m[col])]
    return (m[0][3] / m[0][0], m[1][3] / m[1][1], m[2][3] / m[2][2])


def dual_vertex(hits: list[Vec], normals: list[Vec], lo: Vec, hi: Vec) -> Vec:
    """The crossings' mean where the cell is smooth, else the point nearest every crossing's tangent plane, pulled
    gently towards that mean and kept a margin inside the cell: dual contouring's vertex (its quadratic error
    function with the mass point as regulariser), used only where it buys a crease."""
    mean = (sum(h[0] for h in hits), sum(h[1] for h in hits), sum(h[2] for h in hits))
    mean = (mean[0] / len(hits), mean[1] / len(hits), mean[2] / len(hits))
    smooth = all(n[0] * m[0] + n[1] * m[1] + n[2] * m[2] >= CREASE_DOT for n in normals for m in normals)
    return clamped(mean if smooth else qef_minimum(hits, normals, mean), lo, hi)


def clamped(x: Vec, lo: Vec, hi: Vec) -> Vec:
    m = MARGIN * CELL
    inner = [(min(max(x[d], lo[d] + m), hi[d] - m)) for d in range(3)]
    return (inner[0], inner[1], inner[2])


def qef_minimum(hits: list[Vec], normals: list[Vec], mean: Vec) -> Vec:
    a = [[MASS_WEIGHT * (i == j) for j in range(3)] for i in range(3)]
    b = [MASS_WEIGHT * mean[d] for d in range(3)]
    for h, n in zip(hits, normals):
        nh = n[0] * h[0] + n[1] * h[1] + n[2] * h[2]
        for i in range(3):
            b[i] += n[i] * nh
            for j in range(3):
                a[i][j] += n[i] * n[j]
    return solve3(a, b)


class Grid:
    """Field samples on a grid symmetric about x = 0, meshed by dual contouring: one vertex per cell the surface
    crosses and one quad per crossing grid edge, joining the four cells around it."""

    def __init__(self, animal: Animal) -> None:
        self.animal = animal
        self.ny = round((Y_RANGE[1] - Y_RANGE[0]) / CELL)
        self.nz = round((Z_RANGE[1] - Z_RANGE[0]) / CELL)
        self.values: dict[Index, float] = {}
        for i in range(-X_CELLS, X_CELLS + 1):
            for j in range(self.ny + 1):
                for k in range(self.nz + 1):
                    v = field(self.point((abs(i), j, k)), animal)  # the x < 0 half samples its mirror: bit-identical
                    if v == 0.0:
                        raise ValueError(f"grid vertex {(i, j, k)} lies on the surface; move a part")
                    if v < 0.0 and (abs(i) == X_CELLS or j in (0, self.ny) or k in (0, self.nz)):
                        raise ValueError(f"the animal reaches the grid boundary at {(i, j, k)}")
                    self.values[(i, j, k)] = v
        self.mesh = Mesh()
        self.cell_vertex: dict[Index, int] = {}

    def point(self, g: Index) -> Vec:
        return (g[0] * CELL, Y_RANGE[0] + g[1] * CELL, Z_RANGE[0] + g[2] * CELL)

    def crossing(self, a: Index, b: Index) -> Vec | None:
        fa, fb = self.values[a], self.values[b]
        if (fa < 0.0) == (fb < 0.0):
            return None
        t = fa / (fa - fb)
        pa, pb = self.point(a), self.point(b)
        return (pa[0] + t * (pb[0] - pa[0]), pa[1] + t * (pb[1] - pa[1]), pa[2] + t * (pb[2] - pa[2]))

    def vertex(self, cell: Index) -> int:
        """The cell's vertex, made on first use; a cell at x < 0 mirrors its twin."""
        if cell not in self.cell_vertex:
            i, j, k = cell
            if i < 0:
                p = self.mesh.pos[self.vertex((-i - 1, j, k))]
                self.cell_vertex[cell] = self.mesh.add((-p[0], p[1], p[2]))
            else:
                corners = [(i + (b & 1), j + (b >> 1 & 1), k + (b >> 2)) for b in range(8)]
                crossings = (self.crossing(corners[a], corners[b]) for a, b in CUBE_EDGES)
                hits = [c for c in crossings if c is not None]
                normals = [gradient(h, self.animal) for h in hits]
                v = dual_vertex(hits, normals, self.point(corners[0]), self.point(corners[7]))
                rounded = tuple(round(c, ROUND) + 0.0 for c in v)
                self.cell_vertex[cell] = self.mesh.add((rounded[0], rounded[1], rounded[2]))
        return self.cell_vertex[cell]

    def quad(self, a: Index, axis: int) -> None:
        """The quad around the grid edge from `a` along `axis`, when the field changes sign across it, split along
        its shorter diagonal and wound outward."""
        b = (a[0] + (axis == 0), a[1] + (axis == 1), a[2] + (axis == 2))
        if (self.values[a] < 0.0) == (self.values[b] < 0.0):
            return
        u, w = (axis + 1) % 3, (axis + 2) % 3
        ring: list[int] = []
        for du, dw in ((-1, -1), (0, -1), (0, 0), (-1, 0)):
            c = list(a)
            c[u] += du
            c[w] += dw
            ring.append(self.vertex((c[0], c[1], c[2])))
        if self.values[a] >= 0.0:
            ring.reverse()
        p = [self.mesh.pos[v] for v in ring]
        if math.dist(p[0], p[2]) <= math.dist(p[1], p[3]):
            self.mesh.faces += [(ring[0], ring[1], ring[2]), (ring[0], ring[2], ring[3])]
        else:
            self.mesh.faces += [(ring[1], ring[2], ring[3]), (ring[1], ring[3], ring[0])]

    def march(self) -> Mesh:
        limits = (X_CELLS, self.ny, self.nz)
        for i in range(-X_CELLS, X_CELLS + 1):
            for j in range(self.ny + 1):
                for k in range(self.nz + 1):
                    for axis in range(3):
                        if (i, j, k)[axis] < limits[axis]:
                            self.quad((i, j, k), axis)
        return self.mesh


def quadruped() -> Mesh:
    return Grid(Animal(CORE, limbs())).march()


def quadruped_files() -> dict[str, bytes]:
    note = "quadruped: a symmetric animal with a muzzle fold, neck crease and head dome; surface nets over a hard union"
    m = quadruped()
    return {"meshes/quadruped.glb": glb_bytes("quadruped", note, m, [Primitive(m.faces)])}
