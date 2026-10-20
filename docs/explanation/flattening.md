# Flattening and exact seam lengths

This page explains what flattening does to a chart, and why OpenPlush pins every seam edge to its exact 3D length instead of letting the flattener trade a little length for less distortion. Read it before changing `openplush-flatten`, or when a flattened piece looks more compressed than you expected.

## The short answer

Flattening gives every vertex of one chart a position in the plane, so the chart can be cut from flat fabric. A curved surface cannot lie flat without some stretch or compression, so the question is where that error goes. OpenPlush puts all of it inside the piece: each chart's boundary is laid flat with every edge at exactly its 3D length, and only the interior moves to stay as close to its 3D shape as it can. Both sides of a seam come from the same 3D edges, so they come out the same length with no coordination between pieces, and the two pieces a maker sews together always match.

## How it works

A chart arrives from the seams stage as a `ChartMesh`: model-space positions and faces, already cut open along its seams and checked to be a disk (see [seams](seams.md)). `FlattenSession::new` takes those arrays and does the work in two parts.

**The boundary.** The chart's one boundary loop is laid flat first. Its edge lengths are the 3D lengths and never change. What can change is the angle the boundary turns at each vertex. The flattener first works out the turning the interior asks for, the angles at which the inside would lie flat with the boundary unscaled (the curvature step of Boundary First Flattening, Sawhney and Crane 2017). Those angles rarely close the polygon exactly, so it then adjusts only the turning angles, by as little as it can, until the polygon closes. Lengths are never touched.

**The interior.** With the boundary fixed, the interior starts from a harmonic fill (cotangent weights, or uniform weights if the cotangent fill turns a face over) and is then improved one step at a time by SLIM, which lowers the symmetric Dirichlet energy, a measure of how far each triangle is from its 3D shape (Rabinovich et al. 2017). Each step's line search stops short of turning any triangle over, so every intermediate layout is valid. The session settles when a step lowers the energy by less than one part in a billion, when no step helps, or after 200 steps, which still leaves a valid layout.

`step()` runs one iteration, so a single-threaded worker can report progress and yield between steps; `finish()` hands out the layout only once it has settled, and dropping the session cancels it. Whether a triangle is turned over is decided by `injectivity::verify`, an exact orientation test: every triangle must be strictly counter-clockwise, with no tolerance. The starting fill must pass it, and no later step can break it.

The flattener works in the chart's own units. Flattening a chart and then scaling it gives the same shape as scaling first, so physical size is applied after flattening and never baked in here.

**Where the distortion shows.** A developable chart, such as a cylinder slit lengthwise, flattens with no distortion at all. A curved chart pays inside: a hemisphere cap pinned at its rim compresses toward the middle, with a mean compression deficit of 35 to 38% in [ADR 0005](../adr/0005-boundary-pinned-flattening.md)'s measurements. `metrics::distortion` reports each face's stretch and compression (the singular values of the map from its 3D shape to its flat one), and `summary::summarize` sums a chart up by area. Neither applies a threshold: how much compression is too much depends on the fabric, and the remedy is a dart, which the maker chooses.

**When it fails.** Two failures come from the shape itself, and both are typed errors, never a quiet compromise. The others, such as an empty or degenerate chart, are guards; the [pipeline](pipeline.md#flatten-each-chart-laid-flat) page lists them all.

- The boundary cannot close at its lengths: `BoundaryNotRealizable`, with how far the polygon misses and a dart (the shortest edge path from the boundary to the interior vertex with the most angle defect, the most curved point) that would relieve it.
- A triangle comes out turned over: `Flip`, listing every such face.

In both cases the remedy is another cut, never a length change. On every fixture the research measurements tried, neither happened.

As a last guard, the pattern stage's `parity::check` compares the two sides of a seam in millimetres and refuses a difference over max(1.0 mm, 0.5% of the longer side). With the boundary pinned, a failure there is a bug in the flattener, not a property of the model.

## Why not the alternative

The obvious alternative is to flatten each chart with a free boundary, or with a strong pull toward the 3D lengths rather than an exact pin, and accept a small mismatch between the sides of a seam as ease. That gives lower interior distortion, which is why it is tempting. It was rejected because the mismatch is not small where it matters: on a hemisphere, the free solve drifted seam lengths by 41 to 47%, and on a sphere gore by 5 to 8%, far past what a sewist can ease in. A weighted pull shrinks the gap but cannot close it. A length mismatch is found only when the maker sews and the edges do not meet, after the fabric is cut. Interior compression is shown before cutting, and a dart fixes it.

Angle-preserving (conformal) flattening was not chosen for the interior either: it keeps angles but lets area drift, and area drift is fabric puckering or straining.

## Decisions

[Seam parity is exact and hard (plan, Decision Ledger, D-013)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/plan.md#decision-ledger) · [flattening algorithm (research R3)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/research.md#r3-flattening-algorithm) · [ADR 0005, boundary-pinned flattening](../adr/0005-boundary-pinned-flattening.md)
