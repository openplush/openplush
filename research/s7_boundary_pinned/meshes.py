"""Synthetic disk-topology charts for spike S7.

Non-shipping research code (see specs/001-pattern-pipeline/plan.md, `research/`).
Every chart is a consistently oriented (counter-clockwise) triangle mesh with one
boundary loop, built by mapping a planar grid onto an analytic surface. Units are
arbitrary: flattening is scale-equivariant, so all reported errors are relative.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


@dataclass(frozen=True)
class Chart:
    name: str
    vertices: np.ndarray  # (n, 3) float
    faces: np.ndarray  # (m, 3) int, counter-clockwise
    note: str


def _zip_rings(inner: list[int], outer: list[int], faces: list[tuple[int, int, int]]) -> None:
    """Triangulate the band between two concentric rings, walking both by angle."""
    n_in, n_out = len(inner), len(outer)
    i = j = 0
    while i < n_in or j < n_out:
        # Angle (as a fraction of a turn) of the next vertex on each ring.
        a_in = (i + 1) / n_in
        a_out = (j + 1) / n_out
        if j < n_out and (i >= n_in or a_out <= a_in):
            faces.append((inner[i % n_in], outer[j], outer[(j + 1) % n_out]))
            j += 1
        else:
            faces.append((inner[i % n_in], outer[j % n_out], inner[(i + 1) % n_in]))
            i += 1


def disk_grid(rings: int) -> tuple[np.ndarray, np.ndarray]:
    """Concentric-ring triangulation of the unit disk.

    Returns (polar, faces): polar[:, 0] is radius in [0, 1], polar[:, 1] the angle.
    Ring k has 6k vertices, so triangles stay close to equilateral.
    """
    polar = [(0.0, 0.0)]
    ring_ids: list[list[int]] = [[0]]
    for k in range(1, rings + 1):
        n = 6 * k
        ids = []
        for j in range(n):
            ids.append(len(polar))
            polar.append((k / rings, 2.0 * np.pi * j / n))
        ring_ids.append(ids)
    faces: list[tuple[int, int, int]] = []
    outer = ring_ids[1]
    for j in range(len(outer)):
        faces.append((0, outer[j], outer[(j + 1) % len(outer)]))
    for k in range(2, rings + 1):
        _zip_rings(ring_ids[k - 1], ring_ids[k], faces)
    return np.array(polar), np.array(faces, dtype=int)


def rect_grid(nu: int, nv: int) -> tuple[np.ndarray, np.ndarray]:
    """Structured grid on [-1, 1]^2 with two counter-clockwise triangles per quad."""
    u, v = np.meshgrid(np.linspace(-1, 1, nu), np.linspace(-1, 1, nv), indexing="xy")
    uv = np.column_stack([u.ravel(), v.ravel()])
    faces = []
    for r in range(nv - 1):
        for c in range(nu - 1):
            a = r * nu + c
            b, d, e = a + 1, a + nu, a + nu + 1
            faces.append((a, b, e))
            faces.append((a, e, d))
    return uv, np.array(faces, dtype=int)


def _xy(polar: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    return polar[:, 0] * np.cos(polar[:, 1]), polar[:, 0] * np.sin(polar[:, 1])


def flat_disk(rings: int = 16) -> Chart:
    polar, f = disk_grid(rings)
    x, y = _xy(polar)
    return Chart("flat-disk", np.column_stack([x, y, 0 * x]), f, "oracle: already flat")


def cylinder_patch(rings: int = 16, radius: float = 0.8) -> Chart:
    polar, f = disk_grid(rings)
    x, y = _xy(polar)
    v = np.column_stack([radius * np.sin(x / radius), y, radius * (1 - np.cos(x / radius))])
    return Chart("cylinder-patch", v, f, "oracle: developable (bent plane)")


def lobe_warp(polar: np.ndarray, terms) -> tuple[np.ndarray, np.ndarray]:
    """Conformal warp of the unit disk, w = z * (1 + sum(eps * e^{i*phase} * z^k)).

    Returns (rho, azimuth) of w. Each term adds k lobes of relative depth eps, and
    k = 1 or 2 terms break the symmetry. If sum((k + 1) * |eps|) < 1 then Re w' > 0
    on the disk, so the map is one-to-one (Noshiro-Warschawski) and, being
    conformal, keeps every small triangle's shape and orientation. A radial warp
    rho = r * R(phi) does not: its triangles shear by about lobe depth * lobe count
    relative to the ring spacing at every resolution, and invert once that passes 1.
    Raises ValueError when the bound fails.
    """
    bound = sum((k + 1) * abs(eps) for k, eps, _ in terms)
    if bound >= 1:
        raise ValueError(f"lobe terms sum((k+1)|eps|) = {bound:.2f} >= 1: the warp may fold")
    z = polar[:, 0] * np.exp(1j * polar[:, 1])
    w = z * (1 + sum(eps * np.exp(1j * ph) * z**k for k, eps, ph in terms))
    return np.abs(w), np.angle(w)


def sphere_cap(theta_max_deg: float, rings: int = 16, lobes=(), scale=(1.0, 1.0, 1.0),
               name: str | None = None, peak: bool = False) -> Chart:
    """Cap of a sphere (optionally an ellipsoid) around the north pole.

    The disk is warped by `lobe_warp(lobes)`, then mapped to the sphere with polar
    angle proportional to |w| (azimuthal equidistant), so the outline is wavy and
    non-convex while every triangle stays valid. theta_max_deg is the polar angle
    of the unwarped rim; with peak=True it is instead the rim's largest polar angle.
    Raises ValueError if the rim reaches 180 degrees: the cap would wrap past the
    south pole and fold back on itself.
    """
    polar, f = disk_grid(rings)
    rho, azimuth = lobe_warp(polar, lobes)
    label = name or f"cap-{theta_max_deg:g}"
    scale_rad = np.deg2rad(theta_max_deg) / (rho.max() if peak else 1.0)
    theta = scale_rad * rho
    if theta.max() >= np.pi:
        raise ValueError(f"{label}: peak polar angle {np.rad2deg(theta.max()):.0f} deg >= 180, surface folds")
    v = np.column_stack([np.sin(theta) * np.cos(azimuth), np.sin(theta) * np.sin(azimuth), np.cos(theta)])
    v = v * np.asarray(scale)
    return Chart(label, v, f, "positive curvature (stuffed dome)")


def saddle(rings: int = 16, a: float = 0.6) -> Chart:
    polar, f = disk_grid(rings)
    x, y = _xy(polar)
    return Chart("saddle", np.column_stack([x, y, a * (x * x - y * y)]), f, "negative curvature")


def bump(rings: int = 16, height: float = 0.5, width: float = 0.3, center=(0.3, -0.1)) -> Chart:
    """Flat disk with an off-centre Gaussian bump: positive curvature on top, negative around it."""
    polar, f = disk_grid(rings)
    x, y = _xy(polar)
    z = height * np.exp(-((x - center[0]) ** 2 + (y - center[1]) ** 2) / width**2)
    return Chart(f"bump-h{height:g}", np.column_stack([x, y, z]), f, "mixed curvature, asymmetric")


def sphere_gore(half_width_deg: float, half_height_deg: float = 80.0, nu: int = 9, nv: int = 41) -> Chart:
    """Beach-ball panel: the sphere between two meridians, clipped near the poles."""
    uv, f = rect_grid(nu, nv)
    lon = uv[:, 0] * np.deg2rad(half_width_deg)
    lat = uv[:, 1] * np.deg2rad(half_height_deg)
    v = np.column_stack([np.cos(lat) * np.cos(lon), np.cos(lat) * np.sin(lon), np.sin(lat)])
    return Chart(f"gore-{2 * half_width_deg:g}", v, f, "long thin panel, curved along its length")


def corpus() -> list[Chart]:
    """The synthetic stand-in for the fixture corpus, in order of increasing difficulty."""
    return [
        flat_disk(),
        cylinder_patch(),
        sphere_cap(45),
        sphere_cap(90),
        sphere_cap(135),
        sphere_cap(90, scale=(1.0, 0.6, 1.5), name="ellipsoid-cap-90"),
        sphere_cap(90, lobes=((3, 0.2, 0.0),), name="lobed-cap-90"),
        sphere_cap(120, lobes=((5, 0.13, 0.0),), name="lobed-cap-120"),
        saddle(),
        sphere_gore(45),
        sphere_gore(90),
        # Asymmetric: a symmetric turning sequence closes by symmetry alone, so these
        # are the charts that actually exercise closure.
        sphere_cap(100, lobes=((1, 0.25, 0.0), (2, 0.12, 1.0)), name="asym-cap-100"),
        sphere_cap(110, scale=(0.5, 1.0, 1.6), lobes=((1, 0.2, 0.5), (3, 0.05, 0.0)), name="ear"),
        bump(),
    ]


def stress() -> list[Chart]:
    """Harsher than a sensible seam layout would produce; looking for the failure edge.

    The caps peak at 175 degrees: almost a whole sphere in one piece, with deep
    lobes (each warp near the sum((k+1)|eps|) < 1 limit), but still a valid surface
    (see test_s7.Validity).
    """
    return [
        sphere_cap(175, peak=True, lobes=((4, 0.16, 0.0), (1, 0.05, 0.3)), name="lobed-cap-p175"),
        sphere_cap(175, peak=True, lobes=((1, 0.2, 0.0), (2, 0.1, 1.3), (5, 0.03, 0.0)), name="asym-cap-p175"),
        sphere_cap(175, peak=True, lobes=((6, 0.12, 0.0), (1, 0.03, 0.0)), name="star-cap-p175"),
        bump(height=1.2, width=0.25),
        sphere_gore(150, half_height_deg=85.0),
    ]
