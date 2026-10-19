# Quickstart: 001 pattern pipeline

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-13 | **Status**: Draft — describes the target state, not what exists today

Nothing in this file runs yet. It is the acceptance walkthrough: when every command here works and produces what it claims, feature 001 is done. Keep it honest — if a step here cannot be run, that is a gap in `tasks.md`, not a documentation problem.

## 0. Prerequisites

```sh
rustup toolchain install stable
rustup target add wasm32-unknown-unknown
corepack enable && corepack prepare pnpm@latest --activate   # Node LTS
```

Three documented commands get a new contributor to green (Constitution IX):

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm -r test
```

## 1. Engine, headless (Constitution II)

Every capability is reachable from the CLI. This is the path golden tests take, so it is the path that is guaranteed to work.

```sh
# Inspect a model before doing anything to it
openplush-cli validate fixtures/meshes/capsule.obj --json out/report.json

# Repair, with the plan written out so it is reproducible
openplush-cli repair fixtures/defects/non-manifold-edge.obj \
  --weld --fix-orientation --fill-holes auto \
  --plan-out out/repair-plan.json -o out/repaired.obj

# A full pattern from a prepared seam set
openplush-cli pattern fixtures/meshes/capsule.obj \
  --seams fixtures/seams/capsule.json \
  --fabric minky \
  --size-mm 300 \
  --page a4 \
  -o out/

# What landed in out/
#   pattern.svg          true scale, 1 user unit = 1 mm
#   pattern.pdf          cover page with the page map, then A4 tiles with registration marks; scale square on every page
#   pieces.json          geometry + metadata (FR-063)
#   metrics.json         per-piece distortion summary (FR-021)
#   warnings.json        everything unresolved, with its remedy (FR-064, FR-065)
```

### Check the things that matter

```sh
# Seam parity is a hard contract now (D-013). This must print nothing.
jq -e '.chains[] | select(.parity_error_mm > 1.0)' out/metrics.json && echo "PARITY VIOLATION" || echo "parity ok"

# The scale square must be exactly 50 mm in the file (SC-003)
jq -e '.scale_square_mm == 50.0' out/metrics.json

# Determinism: same input, byte-identical output (SC-008)
openplush-cli pattern fixtures/meshes/capsule.obj --seams fixtures/seams/capsule.json \
  --fabric minky --size-mm 300 --page a4 -o out2/
cmp out/pattern.svg out2/pattern.svg && echo "deterministic"
```

## 2. Determinism across native and WASM

The determinism CI job is this, for every fixture:

```sh
openplush-cli          pattern "$f" --seams "$seams" -o native/
node scripts/wasm-cli.mjs pattern "$f" --seams "$seams" -o wasm/
cmp native/pattern.svg wasm/pattern.svg
```

A mismatch is a release blocker, not a warning. Byte-identical output across platforms is Constitution V, and the usual culprits are hash-map iteration and unstable sorts on an output path.

## 3. Web app

```sh
pnpm --filter web dev          # http://localhost:5173
pnpm --filter web exec playwright test
```

Manual walkthrough matching US1:

1. Drop `fixtures/meshes/capsule.obj` on the window. The health report lists defects; nothing is modified yet.
2. Accept the repairs. A before/after summary appears (Constitution VII).
3. Set height to 300 mm. A human silhouette appears beside the model for scale.
4. Draw a seam by clicking two points. The path follows the surface.
5. Toggle the quality view. Distortion is a heatmap on both the model and the flat pieces.
6. Export for A4. Print at 100% and measure the scale square: 50 mm ± 0.5 mm.

Step 6 is the one that actually matters, and the one most likely to be quietly broken — a pattern that prints at 97% is a ruined project, and nothing on screen reveals it. Measure with a ruler, not by eye.

## 4. What "done" means for 001

| Check | Where |
|---|---|
| Seam parity within max(1.0 mm, 0.5%) on every fixture | SC-001, hard error (D-013) |
| ≥ 95% of surface area within ±5% stretch on the reference corpus | SC-002 |
| Scale square exactly 50.00 mm in-file, ±0.5 mm printed | SC-003 |
| First-time maker produces a printable pattern in ≤ 15 min (n ≥ 5) | SC-004 |
| 50k triangles: seam edit → preview ≤ 10 s | SC-005 |
| Zero network requests after first load | SC-006 |
| 100% of the defect corpus detected | SC-007 |
| Byte-identical exports across runs and platforms | SC-008 |

SC-004 needs recruited participants and a written protocol; it gates the release claim, not the build. Everything else is automatable and belongs in CI.

## 5. Fixtures

```
fixtures/        (+ LICENSES.md for every fixture: CC0/CC-BY only)
├── meshes/      capsule, cube, sphere, quadruped, teddy
├── defects/     one model per defect class — SC-007 measures against this
├── seams/       reference seam sets, so pattern tests need no UI
└── golden/      expected SVG + metrics JSON per fixture
```

`fixtures/` is a contributor lane with no code in it. A model that arrives with a known-good hand-made pattern becomes a regression oracle, and it is also the only way to measure feature 002's SC-001 — so these contributions are worth more than they look.

Never update a golden snapshot to make a test pass (`AGENTS.md`). If output legitimately changed, say so and leave the update to a human reviewer.
