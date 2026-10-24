# Meshes: from triangle soup to half-edges

<!-- index: how a model is stored from import to repair, and the half-edge facts that catch people out. -->

This page explains how OpenPlush stores a 3D model while it works on it, and the facts about that storage that catch people out. Read it before changing `openplush-mesh`, or any code that walks a mesh.

## The short answer

A model arrives as a [triangle soup](../reference/glossary.md#geometry): triangles that list their corners' positions, with nothing yet saying which triangles share an edge. Repair welds corners at the same place into shared vertices and builds a half-edge `ManifoldMesh`, the only form later stages accept. In it every edge is a pair of half-edges with neighbouring indices, the surface's edge is made of explicit half-edges with no face, and anything that can differ between the corners at one vertex, such as a UV coordinate, is stored per corner rather than per vertex.

## How it works

**Weld first.** A file does not promise that two triangles meeting at a point use the same vertex. STL stores every triangle's three corners separately, and exporters of other formats split a vertex wherever its UV coordinate or normal differs on either side, so a smooth sphere can arrive as many disconnected strips. Until the corners at one place are joined, the triangles have no shared edges, and every question about topology (which faces are neighbours, where the boundary is, whether the surface is closed) has the wrong answer. Repair's weld step joins vertices within the weld distance, a tolerance scaled to the model's size, and validation welds the same way before it judges topology.

**Per-corner, not per-vertex.** Welding gives one vertex where a UV seam meets, but that vertex still has a different UV coordinate in each triangle around it. So the values that can differ by triangle live on the half-edges, one per corner: the UV coordinate, and the provenance that names the file's original corner (a `SourceCorner`). Storing them per vertex would keep one of the values and lose the rest.

**Half-edges come in pairs.** Each edge is two half-edges running in opposite directions, and they are allocated together at indices `2k` and `2k + 1`. So the twin of half-edge `h` is `h ^ 1`, and the edge it belongs to is `h >> 1`: bit operations, not lookups, and never out of step with each other. Code that creates half-edges must create them in pairs to keep this true.

**The boundary is explicit.** Where the surface has an edge, as at an opening, the half-edge on the outer side exists and has no face (`face = None`). Every half-edge therefore has a twin, and walking around a boundary loop is the same `next` step as walking around a triangle. A vertex on the boundary always points at its outgoing boundary half-edge, so `is_boundary_vertex` is one lookup and walking around a vertex starts at the same place every time, which keeps the output deterministic.

**Only repair makes one.** `ManifoldMesh` has no public constructor: repair is the only code that produces it, after checking that the surface is consistently oriented and manifold with no degenerate face. Unrepaired data cannot reach the stages after it.

**Seams don't change the mesh.** A seam is a flag on an edge, so drawing or erasing one never rebuilds the mesh; [seams](seams.md) explains how the flags cut the surface into charts and why each chart must be a disk. What the flattener requires of a chart, that every triangle stays the right way up and that physical size is applied only after flattening, is in [flattening](flattening.md).

## Why not the alternative

The obvious alternative is a face-vertex list, triangles holding three vertex indices, which is what files store and what most mesh libraries hand you. It answers "which vertices does this face use" but not "which face is across this edge" or "is this vertex on the boundary" without a search, and those are the questions repair, seam drawing and chart cutting ask all the time. The soup keeps that form only until repair; everything after it gets half-edges.

## Decisions

[Constitution, Principle VII: explicit repair, never silent mutation](https://github.com/openplush/openplush/blob/main/CONSTITUTION.md)
