"""The H4 sweep: candidate-plane scores on the committed heads and on models built to tie, printed as Markdown.

Run: python3 sweep.py   (from this directory; about four minutes, standard library only)
Output is deterministic: no randomness but the seeded jitter, and every table is sorted.
"""

import platform
from pathlib import Path

import variants
from mirror import Mirror, Plane, Vec, decide, read_glb_mesh

FIXTURES = Path(__file__).resolve().parents[2] / "fixtures" / "meshes"
ACCEPT = 0.01  # the H002 oracle's bound: near-symmetric scores under half of it, asymmetric's best above it
MARGINS = (0.0, 0.001, 0.002, 0.003, 0.005, ACCEPT)
LUMP_MM = 2.0  # breaks x = 0 by about a third of the fixture's ear tilt
SNOUTS_MM = (0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 12.0)  # breaks y = 0 from not at all to past the acceptance bound
SPHEROIDS = ((45.0, 55.0, 45.0), (40.0, 55.0, 45.0))
JITTERS = (0.1, 0.25, 0.5, 1.0)
SEED = 7


def fmt_normal(n: Vec) -> str:
    return "(" + ", ".join(f"{c:+.3f}" for c in n) + ")"


def fmt_plane(p: Plane) -> str:
    return f"{p[0]:.4f} {fmt_normal(p[1])} {p[2]:+.2f} mm"


def models() -> list[tuple[str, Mirror]]:
    out: list[tuple[str, Mirror]] = []
    for name in ("near-symmetric", "asymmetric"):
        out.append((f"`{name}.glb`", Mirror(*read_glb_mesh(FIXTURES / f"{name}.glb"))))
    out.append(("upright ears, no lump, no snout", Mirror(*variants.two_planes(0.0, 0.0))))
    for snout in SNOUTS_MM:
        name = f"lump {LUMP_MM:.0f} mm at +x, snout {snout:.0f} mm at +y"
        out.append((name, Mirror(*variants.two_planes(LUMP_MM, snout))))
    for axes in SPHEROIDS:
        out.append((f"spheroid {axes[0]:.0f} × {axes[1]:.0f} × {axes[2]:.0f} mm", Mirror(*variants.spheroid(axes))))
    exact, near = variants.two_planes(0.0, 0.0), variants.near_symmetric_fixture()
    for sigma in JITTERS:
        out.append((f"upright ears, jitter σ = {sigma:.2f} mm", Mirror(*variants.jittered(exact, sigma, SEED))))
    for sigma in JITTERS[2:]:
        name = f"near-symmetric rebuilt, jitter σ = {sigma:.2f} mm"
        out.append((name, Mirror(*variants.jittered(near, sigma, SEED))))
    return out


def main() -> None:
    print(f"Python {platform.python_version()} on {platform.system()} {platform.machine()}\n")
    rows: list[tuple[str, list[Plane]]] = [(name, m.candidates()) for name, m in models()]

    print("## Candidates\n")
    print("Score, normal, offset from the volume centroid along it. Gap is second minus best.\n")
    print("| Model | Distinct | Best | Second | Third | Gap |")
    print("|---|---|---|---|---|---|")
    for name, cands in rows:
        cells = [fmt_plane(p) for p in cands[:3]] + ["—"] * (3 - len(cands[:3]))
        gap = f"{cands[1][0] - cands[0][0]:.4f}" if len(cands) > 1 else "—"
        print(f"| {name} | {len(cands)} | {' | '.join(cells)} | {gap} |")

    print(f"\n## Decisions, accept = {ACCEPT}\n")
    print("| Model | " + " | ".join(f"margin {m}" for m in MARGINS) + " |")
    print("|---|" + "---|" * len(MARGINS))
    for name, cands in rows:
        verdicts = [decide(cands, ACCEPT, m)[0] for m in MARGINS]
        print(f"| {name} | " + " | ".join(verdicts) + " |")


if __name__ == "__main__":
    main()
