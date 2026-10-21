# openplush-mesh

<!-- index: the triangle soup, validation, repair and hole filling, and the half-edge mesh they produce. -->

What `openplush-mesh` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

- `TriangleSoup` and `SourceCorner`
- `SourceMaterials` (`names`, `by_primitive`, `material(primitive)`) and `MaterialIndex`, which a soup carries with `extra_uv_sets` (UV sets the file has beyond the one read): `TriangleSoup::new` gives an empty table, `with_materials(materials, extra_uv_sets)` sets them, and repair moves both unchanged onto the mesh
- the half-edge `ManifoldMesh` (read-only accessors including `materials` and `extra_uv_sets`, `outgoing` returning `Outgoing`, `boundary_loops`, `edge_copies` (repair's copy groups as this mesh's edges, empty if it split none), `debug_validate`; no public constructor)
- IDs `VertexId`, `EdgeId`, `HalfEdgeId`, `FaceId`, `ComponentId`, `MeshElement`, and `id_newtype!`, which defines a `u32` ID with `new`, `index` and the shared derives
- `MeshError` with `CornerArray`, `Counted` and `MaterialFault`, where `InvalidMaterial` names the lowest primitive that a triangle uses but the table lacks (`Missing`), or whose material is not below the name count (`OutOfRange`)

### Validation

`validate(&TriangleSoup) -> ValidationReport`, which never fails and changes nothing. It welds used vertices within the weld distance (transitively, comparing only vertices in neighbouring weld-distance cells) and leaves out repeated-index faces and later copies wound the same way, keeping both layers of a double-sided sheet, then judges topology on what remains, as repair will see it; a vertex there is named by its weld group's smallest ID.

The report lists, each sorted:

- `unwelded` groups
- `zero_area` faces (area at or below the area tolerance, from the soup's own positions)
- `repeated_index` faces
- `duplicates` (`Duplicate`: the copy, the first face it repeats wound its way or, for `DuplicateOrientation::Opposite`, the other way, and `Same` or `Opposite`)
- `non_manifold_edges` (`NonManifoldEdge`: ends and every face)
- `non_manifold_vertices` (faces split into more than one fan joined through shared edges)
- `winding_conflicts` (two-face edges both faces walk the same way)
- `non_orientable` (two-face edges where the flips a breadth-first walk assigns contradict)
- `inside_out` (shells, faces joined across two-face edges, that are closed and have non-positive signed volume, so an inside-out shell touching the rest only at a vertex or a non-manifold edge is still caught; a shell made only of a double-sided sheet's layers is closed and flat but not listed, since repair leaves its winding as it is)
- `boundary_loops` (one-face edges chained in their faces' direction, split wherever a vertex repeats, each from its smallest vertex)
- `components` (vertex-connected, by faces; more than one is a defect)

A soup with no triangles or an extent tolerances refuse is `Unmeasurable` (`NoTriangles`, `Extent { diagonal }`) with nothing else judged.

`self_intersections` lists face pairs sharing no edge that cross along a segment longer than the weld distance, so a fold between faces sharing a vertex counts; faces that only touch (at a corner or edge-on), lie within the weld distance of one plane, or have no area are not listed, so shells overlapping only through coplanar faces go unreported. Candidates come from a bounding-box tree; the test is in floating point against the weld distance, since the report only warns. A vertex of valence k costs about k² pair tests.

### Repair

`RepairPlan` (five step toggles, all on by default, and `fill_holes`, the loops to fill, which are named by `LoopRef`, the lowest `SourceCorner` among the loop's vertices; a loop not listed stays an opening) and `RepairReport`, read through accessors that list each step's changes: welded groups, dropped repeated-index and duplicate faces (`DroppedCopy`), sheets, stitched caps and re-triangulated faces, paired edges (`PairedEdge`), added vertex copies (`VertexCopy`), flipped faces, orientation conflicts, components (`Component`), holes (`HoleReport`), mandatory seams and edge copies. Removals, flips and pairings name IDs of the soup given to repair, additions those of the repaired mesh.

`into_manifold(&soup, &plan)` returns `(ManifoldMesh, RepairReport)` and is the only way to get a mesh: weld, drop repeated-index faces and copies and stitch caps, pair the faces on over-shared edges, orient, split vertices (pairing- and conflict-aware), pair again, split components, compact unused vertices, then build (which cuts a conflict edge the vertex split left shared). A step the plan turns off is skipped and what it would have fixed reaches the build, which refuses it with its typed error.

### Hole filling

After the build every boundary loop is classified: an opening unless `fill_holes` lists it; a listed loop the mesh does not have is `UnknownHoleLoop`, the lowest named first. A listed loop is filled, in loop order, as Liepa 2003 does.

- It is triangulated by dynamic programming over its vertices for the lowest largest dihedral angle (1 - cos between unit normals, against the patch and the faces across the rim), then the lowest area, ties to the lowest apex, excluding a chord the mesh already has and a sliver.
- It is refined: a triangle is split at its centroid when the centroid is further than σ/√2 from each corner v, both for the triangle's σ (the mean of its corners') and for v's own (σ the mean edge length around a rim vertex, averaged for a new one), with Delaunay edge swaps after each round (an edge swaps when its opposite angles' cotangents sum below `-DELAUNAY_COT`).
- It is faired: the added vertices solve the combinatorial bilaplacian with the rim and its outer ring fixed, in one sparse Cholesky (`faer`).

The patch is appended after the mesh's own vertices, edges and faces, its corners with no `SourceCorner` and no UV, and `HoleReport` lists its faces. A loop of more than 512 vertices is `HoleTooLarge`, one along a copy of a split edge `HoleOnSplit`, and one with no triangulation, a patch that does not settle or solve, or a faired sliver `HoleNotFillable`.

### Repair errors, conflicts and edge copies

Errors beyond the build's: `NoTriangles`, `UnusableExtent`, `UnknownHoleLoop`, `HoleNotFillable`, `HoleOnSplit` and `HoleTooLarge`.

A non-orientable conflict edge (two faces no flips make agree) is cut into two boundary copies, both mandatory seams, listed in `orientation_conflicts` by the ends of the copy its lower face kept; with `fix_orientation` off it reaches the build and is refused as `InconsistentOrientation`.

`edge_copies` lists every edge repair split (a paired edge or a conflict cut) as the mesh's edges it became, each copy with two faces (a pair) or one (an unpaired face, or either side of a cut), ascending, the groups in order of their first copy; the join of a group's seam chains into one bundle is `SewingMap`'s.

Report faces are the input soup's IDs, except that vertex copies, conflicts, mandatory seams, hole loops and edge copies use the mesh's IDs.

## Depends on

`openplush-geom`, `faer`, `thiserror`
