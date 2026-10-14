"""Boundary-pinned flattening (D-013) against a free-boundary baseline, for spike S7.

Non-shipping research code: a reference for the Rust flattener, not a design for it.

Pinned pipeline (research R3, Option A):
  1. Keep every boundary edge at its 3D length (exact seam parity).
  2. Choose boundary turning angles the way BFF does (Sawhney & Crane 2017) with the
     boundary scale factor fixed at u = 0: solve for the interior conformal factor
     that flattens the interior, and read the boundary curvature that results.
  3. That curve generally does not close. BFF would fix it by changing lengths, which
     would break parity, so lengths stay exact and only the turning angles move: the
     smallest change (least squares) that closes the polygon.
  4. Fill the interior with the boundary fixed (cotan-harmonic or Tutte), then
     minimise the symmetric Dirichlet energy (what SLIM minimises) with a line search
     that never lets a triangle flip.

Free baseline: Tutte onto a circle, then symmetric Dirichlet with every vertex free.
It shows how much distortion pinning costs, and how much seam-length error it avoids.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import scipy.sparse as sp
import scipy.sparse.linalg as spla
from scipy.optimize import minimize

# ---------------------------------------------------------------- topology


def euler_counts(faces: np.ndarray) -> tuple[int, int, int]:
    edges = {tuple(sorted((int(a), int(b)))) for t in faces for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
    return len(np.unique(faces)), len(edges), len(faces)


def boundary_loops(faces: np.ndarray) -> list[list[int]]:
    """Boundary loops in the faces' orientation (counter-clockwise for our charts)."""
    directed = {(int(a), int(b)) for t in faces for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0]))}
    nxt = {a: b for a, b in directed if (b, a) not in directed}
    loops, seen = [], set()
    for start in sorted(nxt):
        if start in seen:
            continue
        loop, cur = [start], nxt[start]
        seen.add(start)
        while cur != start:
            loop.append(cur)
            seen.add(cur)
            cur = nxt[cur]
        loops.append(loop)
    return loops


# ---------------------------------------------------------------- geometry


def _norm(v: np.ndarray) -> np.ndarray:
    return np.linalg.norm(v, axis=-1)


def _corner_angle(a: np.ndarray, b: np.ndarray, c: np.ndarray) -> np.ndarray:
    """Angle at a in triangle (a, b, c); atan2 form stays accurate near 0 and pi."""
    u, v = b - a, c - a
    return np.arctan2(_norm(np.cross(u, v)), np.einsum("ij,ij->i", u, v))


class Geometry:
    """Intrinsic quantities of a 3D chart that every flattening step needs."""

    def __init__(self, vertices: np.ndarray, faces: np.ndarray):
        self.V = np.asarray(vertices, float)
        self.F = np.asarray(faces, int)
        n = len(self.V)
        p0, p1, p2 = (self.V[self.F[:, k]] for k in range(3))

        # Rest shape of each triangle in its own 2D frame; columns are edges 0->1, 0->2.
        e1, e2 = p1 - p0, p2 - p0
        x_axis = e1 / _norm(e1)[:, None]
        normal = np.cross(e1, e2)
        y_axis = np.cross(normal, x_axis)
        y_axis /= _norm(y_axis)[:, None]
        rest = np.zeros((len(self.F), 2, 2))
        rest[:, 0, 0] = _norm(e1)
        rest[:, 0, 1] = np.einsum("ij,ij->i", e2, x_axis)
        rest[:, 1, 1] = np.einsum("ij,ij->i", e2, y_axis)
        self.rest_inv = np.linalg.inv(rest)
        self.area = 0.5 * _norm(normal)

        angles = np.stack([_corner_angle(p0, p1, p2), _corner_angle(p1, p2, p0), _corner_angle(p2, p0, p1)], 1)
        angle_sum = np.bincount(self.F.ravel(), angles.ravel(), n)

        self.loop = np.array(boundary_loops(self.F)[0])
        is_b = np.zeros(n, bool)
        is_b[self.loop] = True
        self.interior = np.flatnonzero(~is_b)
        self.kappa_boundary = np.pi - angle_sum[self.loop]  # ordered along the loop
        self.gauss_interior = 2 * np.pi - angle_sum[self.interior]
        self.boundary_lengths = _norm(self.V[np.roll(self.loop, -1)] - self.V[self.loop])

        # Cotan Laplacian, positive semidefinite. Corner k is opposite edge (k+1, k+2).
        rows, cols, vals = [], [], []
        cot = 1.0 / np.tan(angles)
        for k in range(3):
            i, j = self.F[:, (k + 1) % 3], self.F[:, (k + 2) % 3]
            w = 0.5 * cot[:, k]
            rows += [i, j, i, j]
            cols += [j, i, i, j]
            vals += [-w, -w, w, w]
        self.cotan = sp.csr_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))), shape=(n, n))

        # Uniform (Tutte) graph Laplacian.
        edges = np.unique(np.sort(np.concatenate([self.F[:, [0, 1]], self.F[:, [1, 2]], self.F[:, [2, 0]]]), 1), axis=0)
        adj = sp.csr_matrix((np.ones(2 * len(edges)), (np.r_[edges[:, 0], edges[:, 1]], np.r_[edges[:, 1], edges[:, 0]])), shape=(n, n))
        self.uniform = sp.diags(np.asarray(adj.sum(1)).ravel()) - adj


# ---------------------------------------------------------------- boundary


def bff_boundary_curvature(geo: Geometry) -> np.ndarray:
    """Flat-boundary turning angles with every boundary scale factor at zero (lengths kept).

    Discrete Yamabe relation with A positive semidefinite: K_new = K + A u. Requiring
    zero curvature inside gives A_II u_I = -K_I (u_B = 0); the boundary then turns by
    kappa_new = kappa + (A u)_B, which totals 2*pi by Gauss-Bonnet.
    """
    A, inner, bnd = geo.cotan, geo.interior, geo.loop
    u_i = spla.spsolve(A[inner][:, inner].tocsc(), -geo.gauss_interior)
    return geo.kappa_boundary + A[bnd][:, inner] @ u_i


def _headings(kappa: np.ndarray) -> np.ndarray:
    """Heading of edge i (vertex i to i+1) when vertex 0's edge points along +x."""
    return np.concatenate([[0.0], np.cumsum(kappa[1:])])


def polygon_points(lengths: np.ndarray, kappa: np.ndarray) -> np.ndarray:
    """Walk the boundary: n+1 points, the last equal to the first iff the curve closes."""
    phi = _headings(kappa)
    steps = lengths[:, None] * np.column_stack([np.cos(phi), np.sin(phi)])
    return np.vstack([[0.0, 0.0], np.cumsum(steps, axis=0)])


@dataclass
class ClosureResult:
    success: bool
    kappa: np.ndarray
    max_change: float  # radians, largest per-vertex turning-angle change


def close_polygon(lengths: np.ndarray, target: np.ndarray) -> ClosureResult:
    """Smallest change to the turning angles that closes the polygon. Lengths are never touched."""
    scale = lengths.sum()
    ell = lengths / scale

    def constraints(kappa):
        phi = _headings(kappa)
        return np.array([kappa.sum() - 2 * np.pi, (ell * np.cos(phi)).sum(), (ell * np.sin(phi)).sum()])

    def jacobian(kappa):
        phi = _headings(kappa)
        tail_sin = np.cumsum((ell * np.sin(phi))[::-1])[::-1]  # sum over i >= j
        tail_cos = np.cumsum((ell * np.cos(phi))[::-1])[::-1]
        jac = np.zeros((3, len(kappa)))
        jac[0] = 1.0
        jac[1, 1:] = -tail_sin[1:]
        jac[2, 1:] = tail_cos[1:]
        return jac

    res = minimize(
        lambda k: 0.5 * np.sum((k - target) ** 2),
        target.copy(),
        jac=lambda k: k - target,
        constraints=[{"type": "eq", "fun": constraints, "jac": jacobian}],
        method="SLSQP",
        options={"maxiter": 500, "ftol": 1e-15},
    )
    kappa = res.x
    # Polish onto the constraint surface with minimum-norm Gauss-Newton steps.
    for _ in range(20):
        c = constraints(kappa)
        if np.abs(c).max() < 1e-14:
            break
        jac = jacobian(kappa)
        kappa = kappa - jac.T @ np.linalg.solve(jac @ jac.T, c)
    ok = bool(np.abs(constraints(kappa)).max() < 1e-12)
    return ClosureResult(ok, kappa, float(np.abs(kappa - target).max()))


def polygon_is_simple(points: np.ndarray) -> bool:
    """True if the closed polygon is counter-clockwise and no two non-adjacent edges meet.

    Touching counts as meeting: a vertex lying on another edge cannot be cut as one
    piece. Collinearity uses a tolerance scaled by the perimeter.
    """
    a, b = points, np.roll(points, -1, axis=0)
    n = len(points)
    signed_area = 0.5 * np.sum(a[:, 0] * b[:, 1] - b[:, 0] * a[:, 1])
    if signed_area <= 0:
        return False
    tol = 1e-12 * np.sum(_norm(b - a)) ** 2

    def orient(p, q, r):
        o = (q[..., 0] - p[..., 0]) * (r[..., 1] - p[..., 1]) - (q[..., 1] - p[..., 1]) * (r[..., 0] - p[..., 0])
        return np.where(np.abs(o) <= tol, 0.0, o)

    def on_segment(p, q, r):  # r collinear with p-q: is it within the segment's box?
        return ((np.minimum(p[..., 0], q[..., 0]) <= r[..., 0]) & (r[..., 0] <= np.maximum(p[..., 0], q[..., 0]))
                & (np.minimum(p[..., 1], q[..., 1]) <= r[..., 1]) & (r[..., 1] <= np.maximum(p[..., 1], q[..., 1])))

    i, j = np.triu_indices(n, 2)
    keep = ~((i == 0) & (j == n - 1))  # first and last edges share a vertex
    i, j = i[keep], j[keep]
    d1, d2 = orient(a[i], b[i], a[j]), orient(a[i], b[i], b[j])
    d3, d4 = orient(a[j], b[j], a[i]), orient(a[j], b[j], b[i])
    proper = (d1 * d2 < 0) & (d3 * d4 < 0)
    touch = (((d1 == 0) & on_segment(a[i], b[i], a[j])) | ((d2 == 0) & on_segment(a[i], b[i], b[j]))
             | ((d3 == 0) & on_segment(a[j], b[j], a[i])) | ((d4 == 0) & on_segment(a[j], b[j], b[i])))
    return not np.any(proper | touch)


# ---------------------------------------------------------------- interior


def harmonic_fill(geo: Geometry, boundary_xy: np.ndarray, weights: str) -> np.ndarray:
    L = geo.cotan if weights == "cotan" else geo.uniform
    inner, bnd = geo.interior, geo.loop
    x = np.zeros((len(geo.V), 2))
    x[bnd] = boundary_xy
    lii, lib = L[inner][:, inner].tocsc(), L[inner][:, bnd]
    x[inner] = np.column_stack([spla.spsolve(lii, -(lib @ boundary_xy[:, k])) for k in range(2)])
    return x


def tutte_free_init(geo: Geometry) -> np.ndarray:
    """Boundary on a circle at arc-length spacing (convex, so Tutte is injective)."""
    lengths = geo.boundary_lengths
    t = 2 * np.pi * np.concatenate([[0.0], np.cumsum(lengths[:-1])]) / lengths.sum()
    radius = lengths.sum() / (2 * np.pi)
    return harmonic_fill(geo, radius * np.column_stack([np.cos(t), np.sin(t)]), "uniform")


def _jacobians(geo: Geometry, x: np.ndarray) -> np.ndarray:
    f = geo.F
    d = np.stack([x[f[:, 1]] - x[f[:, 0]], x[f[:, 2]] - x[f[:, 0]]], axis=2)  # columns are edges
    return d @ geo.rest_inv


def flips(geo: Geometry, x: np.ndarray) -> int:
    return int(np.sum(np.linalg.det(_jacobians(geo, x)) <= 0))


def sd_energy(geo: Geometry, x: np.ndarray, with_grad: bool = False):
    """Area-weighted mean symmetric Dirichlet energy, ||J||^2 + ||J^-1||^2. Isometry gives 4."""
    J = _jacobians(geo, x)
    det = J[:, 0, 0] * J[:, 1, 1] - J[:, 0, 1] * J[:, 1, 0]
    if np.any(det <= 0):
        return np.inf, None
    fro = np.einsum("tij,tij->t", J, J)
    w = geo.area / geo.area.sum()
    energy = float(np.sum(w * (fro + fro / det**2)))
    if not with_grad:
        return energy, None
    cof = np.stack([np.stack([J[:, 1, 1], -J[:, 1, 0]], 1), np.stack([-J[:, 0, 1], J[:, 0, 0]], 1)], 1)
    dE_dJ = 2 * J + 2 * J / det[:, None, None] ** 2 - 2 * (fro / det**3)[:, None, None] * cof
    dE_dD = (w[:, None, None] * dE_dJ) @ np.transpose(geo.rest_inv, (0, 2, 1))
    grad = np.zeros_like(x)
    np.add.at(grad, geo.F[:, 1], dE_dD[:, :, 0])
    np.add.at(grad, geo.F[:, 2], dE_dD[:, :, 1])
    np.add.at(grad, geo.F[:, 0], -dE_dD[:, :, 0] - dE_dD[:, :, 1])
    return energy, grad


def smallest_positive_root(a: np.ndarray, b: np.ndarray, c: np.ndarray) -> np.ndarray:
    """Smallest real root > 0 of a t^2 + b t + c, elementwise; inf where there is none.

    Uses q = -(b + sign(b) sqrt(disc)) / 2, roots q / a and c / q, which avoids the
    cancellation that makes the textbook formula lose the small root when |a| is
    tiny, and needs no separate linear branch (c / q is the root as a -> 0).
    """
    with np.errstate(divide="ignore", invalid="ignore"):
        disc = b * b - 4 * a * c
        real = disc >= 0
        q = -0.5 * (b + np.copysign(np.sqrt(np.where(real, disc, 0.0)), b))
        r1 = np.where(a != 0, q / a, np.inf)
        r2 = np.where(q != 0, c / q, np.inf)
    r1 = np.where(real & np.isfinite(r1) & (r1 > 0), r1, np.inf)
    r2 = np.where(real & np.isfinite(r2) & (r2 > 0), r2, np.inf)
    return np.minimum(r1, r2)


def _max_injective_step(geo: Geometry, x: np.ndarray, p: np.ndarray) -> float:
    """Largest t with every det(J(x + t p)) > 0 (Smith & Schaefer 2015).

    The energy's barrier (infinite at det = 0) is the backstop if this bound is
    ever slightly optimistic: the line search rejects any step that flips a face.
    """
    f = geo.F
    d = np.stack([x[f[:, 1]] - x[f[:, 0]], x[f[:, 2]] - x[f[:, 0]]], axis=2)
    e = np.stack([p[f[:, 1]] - p[f[:, 0]], p[f[:, 2]] - p[f[:, 0]]], axis=2)
    c = d[:, 0, 0] * d[:, 1, 1] - d[:, 0, 1] * d[:, 1, 0]
    b = d[:, 0, 0] * e[:, 1, 1] + e[:, 0, 0] * d[:, 1, 1] - d[:, 0, 1] * e[:, 1, 0] - e[:, 0, 1] * d[:, 1, 0]
    a = e[:, 0, 0] * e[:, 1, 1] - e[:, 0, 1] * e[:, 1, 0]
    return float(smallest_positive_root(a, b, c).min())


def minimize_sd(geo: Geometry, x0: np.ndarray, free: np.ndarray, iters: int = 4000) -> tuple[np.ndarray, int, str]:
    """L-BFGS on the free vertices; every accepted step keeps all triangles unflipped.

    Returns (x, iterations, reason). reason is "gtol" (gradient small: converged),
    "stalled" (energy stopped changing at machine precision), "maxiter", "linesearch"
    (no acceptable step), or "infeasible-start" (the start already has a flip).
    Only gtol and stalled count as converged.
    """
    x = x0.copy()
    energy, grad = sd_energy(geo, x, with_grad=True)
    if not np.isfinite(energy):
        return x, 0, "infeasible-start"
    mask = np.zeros_like(x, bool)
    mask[free] = True
    s_hist, y_hist = [], []
    g = np.where(mask, grad, 0).ravel()
    n_iter, reason = 0, "maxiter"
    for _ in range(iters):
        n_iter += 1
        q = g.copy()
        alphas = []
        for s, y in reversed(list(zip(s_hist, y_hist, strict=True))):
            a = s @ q / (y @ s)
            alphas.append(a)
            q -= a * y
        if y_hist:
            q *= (s_hist[-1] @ y_hist[-1]) / (y_hist[-1] @ y_hist[-1])
        for (s, y), a in zip(zip(s_hist, y_hist, strict=True), reversed(alphas), strict=True):
            q += (a - (y @ q) / (y @ s)) * s
        p = -q.reshape(x.shape)
        if g @ p.ravel() >= 0:  # not a descent direction: restart from steepest descent
            s_hist, y_hist = [], []
            p = -g.reshape(x.shape)
        t = min(1.0, 0.8 * _max_injective_step(geo, x, p))
        while t > 1e-16:
            e_new, g_new = sd_energy(geo, x + t * p, with_grad=True)
            if e_new <= energy + 1e-4 * t * (g @ p.ravel()):
                break
            t *= 0.5
        else:
            reason = "linesearch"
            break
        g_new = np.where(mask, g_new, 0).ravel()
        step, dg = t * p.ravel(), g_new - g
        # Keep the pair only if it satisfies the curvature condition; otherwise the
        # two-loop recursion stops producing a positive-definite inverse Hessian.
        if step @ dg > 1e-12 * np.linalg.norm(step) * np.linalg.norm(dg):
            s_hist.append(step)
            y_hist.append(dg)
            if len(s_hist) > 10:
                s_hist.pop(0)
                y_hist.pop(0)
        x = x + t * p
        stalled = abs(energy - e_new) <= 1e-15 * energy
        energy, g = e_new, g_new
        # Gradient test first: near the minimum the energy is quadratic, so an energy
        # tolerance of eps would leave ~sqrt(eps) error in the positions.
        if np.abs(g).max() < 1e-10:
            reason = "gtol"
            break
        if stalled:
            reason = "stalled"
            break
    return x, n_iter, reason


# ---------------------------------------------------------------- reporting


@dataclass
class Stats:
    status: str
    init: str
    init_flips: int
    flips: int
    injective: bool
    sd_mean: float  # 4.0 is isometric
    stretch_max: float  # largest singular value (1.0 = no stretch)
    compress_min: float  # smallest singular value (1.0 = no compression)
    boundary_len_err_max: float  # max |l_2D / l_3D - 1| over boundary edges
    perimeter_err: float  # |P_2D / P_3D - 1|
    iterations: int
    termination: str  # minimize_sd reason, or "-" if it was not run


def _stats(geo: Geometry, x: np.ndarray, status: str, init: str, init_flips: int, iterations: int,
           termination: str = "-") -> Stats:
    J = _jacobians(geo, x)
    det = np.linalg.det(J)
    fro = np.einsum("tij,tij->t", J, J)
    root_p = np.sqrt(np.maximum(fro + 2 * np.abs(det), 0))
    root_m = np.sqrt(np.maximum(fro - 2 * np.abs(det), 0))
    s1, s2 = (root_p + root_m) / 2, (root_p - root_m) / 2
    energy, _ = sd_energy(geo, x)
    lengths_2d = _norm(x[np.roll(geo.loop, -1)] - x[geo.loop])
    n_flips = int(np.sum(det <= 0))
    return Stats(
        status=status,
        init=init,
        init_flips=init_flips,
        flips=n_flips,
        injective=n_flips == 0,
        sd_mean=energy,
        stretch_max=float(s1.max()),
        compress_min=float(s2.min()),
        boundary_len_err_max=float(np.abs(lengths_2d / geo.boundary_lengths - 1).max()),
        perimeter_err=float(abs(lengths_2d.sum() / geo.boundary_lengths.sum() - 1)),
        iterations=iterations,
        termination=termination,
    )


@dataclass
class ChartResult:
    name: str
    triangles: int
    total_gauss_deg: float  # how far from developable
    natural_gap: float  # BFF curve's closure gap / perimeter, before any fix
    closure_ok: bool
    closure_max_change_deg: float
    boundary_simple: bool
    reflex_corners: int  # boundary vertices turning inward in the pinned outline
    pinned: Stats
    free: Stats


def run_chart(chart) -> ChartResult:
    geo = Geometry(chart.vertices, chart.faces)
    lengths = geo.boundary_lengths

    target = bff_boundary_curvature(geo)
    natural = polygon_points(lengths, target)
    gap = float(np.linalg.norm(natural[-1] - natural[0]) / lengths.sum())

    closure = close_polygon(lengths, target)
    boundary = polygon_points(lengths, closure.kappa)[:-1]
    simple = polygon_is_simple(boundary)

    # A triangle with all three corners on the boundary is fixed by the boundary
    # alone; if it comes out inverted (e.g. at a reflex corner) no interior solve
    # can repair it. The chart needs a split or a dart, whatever the start.
    on_loop = np.zeros(len(geo.V), bool)
    on_loop[geo.loop] = True
    rim_only = np.flatnonzero(on_loop[geo.F].all(axis=1))
    placed = np.zeros((len(geo.V), 2))
    placed[geo.loop] = boundary
    rim_inverted = int(np.sum(np.linalg.det(_jacobians(geo, placed)[rim_only]) <= 0)) if len(rim_only) else 0

    candidates = [(w, harmonic_fill(geo, boundary, w)) for w in ("cotan", "uniform")]
    counts = {w: flips(geo, x) for w, x in candidates}
    usable = [(w, x) for w, x in candidates if counts[w] == 0]
    if not closure.success:
        pinned = _stats(geo, candidates[0][1], "closure failed", "-", min(counts.values()), 0)
    elif not simple:
        pinned = _stats(geo, candidates[0][1], "boundary self-intersects", "-", min(counts.values()), 0)
    elif rim_inverted:
        pinned = _stats(geo, candidates[0][1], f"{rim_inverted} all-boundary triangle(s) inverted", "-",
                        min(counts.values()), 0)
    elif not usable:
        best = min(candidates, key=lambda c: counts[c[0]])
        pinned = _stats(geo, best[1], "no injective initialisation", best[0], counts[best[0]], 0)
    else:
        w, x0 = usable[0]
        x, n_it, why = minimize_sd(geo, x0, geo.interior)
        status = "ok" if why in ("gtol", "stalled") else f"did not converge ({why})"
        pinned = _stats(geo, x, status, w, 0, n_it, why)

    x0 = tutte_free_init(geo)
    x, n_it, why = minimize_sd(geo, x0, np.arange(len(geo.V)))
    status = "ok" if why in ("gtol", "stalled") else f"did not converge ({why})"
    free = _stats(geo, x, status, "tutte-circle", flips(geo, x0), n_it, why)

    return ChartResult(
        name=chart.name,
        triangles=len(geo.F),
        total_gauss_deg=float(np.rad2deg(geo.gauss_interior.sum())),
        natural_gap=gap,
        closure_ok=closure.success,
        closure_max_change_deg=float(np.rad2deg(closure.max_change)),
        boundary_simple=simple,
        reflex_corners=int(np.sum(closure.kappa < 0)),
        pinned=pinned,
        free=free,
    )
