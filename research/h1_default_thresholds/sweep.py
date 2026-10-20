"""The H1 sweep: hint counts per fixture over a grid of thresholds, printed as Markdown for results.md.

Run: python3 sweep.py   (from this directory; about a minute, standard library only)
Deterministic: no randomness, every table sorted. The quadruped's hints are also placed by region, so the
tables say not just how many hints a threshold yields but whether they land on the features the spec names.
"""

import sys
from pathlib import Path

from signals import Chain, Mesh, chains, clusters, integrated_curvature, read_obj, signed_folds

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "h4_symmetry_margin"))
from mirror import read_glb_mesh  # noqa: E402

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures" / "meshes"
PERMISSIVE = 10.0  # degrees; the most permissive fold at which chains are built once
SHARP = (20.0, 25.0, 30.0, 35.0, 40.0, 45.0)
CONCAVE = (15.0, 20.0, 25.0, 30.0, 35.0)
MIN_LENGTH = (0.0, 10.0, 20.0, 30.0)
RADII = (10.0, 15.0, 20.0)
CURVATURE = (30.0, 45.0, 60.0, 90.0, 120.0, 180.0)  # clusters are built once at the first, filtered by peak
ABSOLUTE = "--absolute" in sys.argv  # research R1's |K| integration; the default is the signed sum
Box = tuple[tuple[float, float], tuple[float, float], tuple[float, float]]
REGIONS: dict[str, Box] = {  # the quadruped's parts, by the solids that make it (source_quadruped.py)
    "neck": ((-20.0, 20.0), (44.0, 84.0), (78.0, 118.0)),
    "muzzle fold": ((-16.0, 16.0), (96.0, 116.0), (90.0, 120.0)),
    "muzzle tip": ((-14.0, 14.0), (116.0, 132.0), (88.0, 118.0)),
    "dome": ((-6.0, 6.0), (70.0, 100.0), (134.0, 160.0)),
    "ears": ((-22.0, 22.0), (62.0, 90.0), (124.0, 160.0)),
    "leg roots": ((-30.0, 30.0), (-58.0, 58.0), (34.0, 58.0)),
    "feet": ((-30.0, 30.0), (-58.0, 58.0), (-3.0, 14.0)),
    "tail": ((-12.0, 12.0), (-106.0, -80.0), (70.0, 110.0)),
    "tail root": ((-12.0, 12.0), (-80.0, -56.0), (70.0, 110.0)),
    "chest": ((-26.0, 26.0), (50.0, 76.0), (50.0, 100.0)),
    "rump": ((-26.0, 26.0), (-76.0, -50.0), (50.0, 100.0)),
}


def load() -> dict[str, Mesh]:
    out = {n: read_obj(FIXTURES / f"{n}.obj") for n in ("cube", "sphere", "capsule")}
    for n in ("faceted", "quadruped"):
        pos, faces = read_glb_mesh(FIXTURES / f"{n}.glb")
        out[n] = Mesh(pos, faces)
    return out


def region_of(p: tuple[float, float, float]) -> str:
    for name, box in REGIONS.items():
        if all(lo <= p[i] <= hi for i, (lo, hi) in enumerate(box)):
            return name
    return "elsewhere"


def chain_centre(m: Mesh, c: Chain) -> tuple[float, float, float]:
    xs = [m.pos[v][d] for e in c.edges for v in e for d in (0,)]
    ys = [m.pos[v][1] for e in c.edges for v in e]
    zs = [m.pos[v][2] for e in c.edges for v in e]
    return (sum(xs) / len(xs), sum(ys) / len(ys), sum(zs) / len(zs))


def by_region(names: list[str]) -> str:
    counts: dict[str, int] = {}
    for n in names:
        counts[n] = counts.get(n, 0) + 1
    return ", ".join(f"{n} {c}" for n, c in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))) or "—"


def fold_tables(meshes: dict[str, Mesh]) -> None:
    folds = {n: signed_folds(m) for n, m in meshes.items()}
    for sign, name, levels in ((1, "Sharp edges", SHARP), (-1, "Concave creases", CONCAVE)):
        built = {
            n: chains(m, folds[n], [e for e, a in folds[n].items() if sign * a >= PERMISSIVE])
            for n, m in meshes.items()
        }
        print(f"\n## {name}: chains built once at {PERMISSIVE:.0f}°, counted by mean fold and length\n")
        print("| Fixture | min length | " + " | ".join(f"≥ {t:.0f}°" for t in levels) + " |")
        print("|---|---|" + "---|" * len(levels))
        for n in meshes:
            for length in MIN_LENGTH:
                cells = [sum(c.mean_fold >= t and c.length_mm >= length for c in built[n]) for t in levels]
                print(f"| {n} | {length:.0f} mm | " + " | ".join(str(c) for c in cells) + " |")
        m = meshes["quadruped"]
        print(f"\nQuadruped {name.lower()} by region (min length 20 mm):\n")
        for t in levels:
            names = [region_of(chain_centre(m, c)) for c in built["quadruped"] if c.mean_fold >= t and c.length_mm >= 20.0]
            print(f"- ≥ {t:.0f}°: {by_region(names)}")


def curvature_tables(meshes: dict[str, Mesh]) -> None:
    print("\n## Dart candidates: clusters of the integrated angle defect over a geodesic disk\n")
    print("| Fixture | radius | " + " | ".join(f"≥ {t:.0f}°" for t in CURVATURE) + " |")
    print("|---|---|" + "---|" * len(CURVATURE))
    placed: list[str] = []
    for n, m in meshes.items():
        for r in RADII:
            signal = integrated_curvature(m, r, absolute=ABSOLUTE)
            built = clusters(m, signal, CURVATURE[0])
            cells = [sum(peak >= t for _, peak in built) for t in CURVATURE]
            print(f"| {n} | {r:.0f} mm | " + " | ".join(str(c) for c in cells) + " |")
            if n == "quadruped":
                for t in CURVATURE:
                    names = [region_of(m.pos[v]) for v, peak in built if peak >= t]
                    where = sorted(tuple(round(c) for c in m.pos[v]) for v, peak in built if peak >= t and region_of(m.pos[v]) == "elsewhere")
                    placed.append(f"- radius {r:.0f} mm, ≥ {t:.0f}°: {by_region(names)}" + (f"; elsewhere at {where[:6]}" if where else ""))
    print("\nQuadruped dart candidates by region (clusters built once at the first threshold, kept by their peak):\n")
    print("\n".join(placed))


def main() -> None:
    meshes = load()
    print("| Fixture | vertices | faces |")
    print("|---|---|---|")
    for n, m in meshes.items():
        print(f"| {n} | {len(m.pos)} | {len(m.faces)} |")
    fold_tables(meshes)
    curvature_tables(meshes)


if __name__ == "__main__":
    main()
