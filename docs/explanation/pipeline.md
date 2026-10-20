# The pipeline, one model at a time

This page follows one model from its file to a printed pattern, stage by stage: which crate does the work, what it hands on, and what can stop it. Read it to know where a change belongs and which error a failure there raises. The type each stage guarantees, and how errors differ from warnings, are on the [architecture](architecture.md) page; what each crate exports is in [reference/crates.md](../reference/crates.md).

## The short answer

A model is read into a triangle soup (`openplush-io`), checked and repaired into a manifold mesh (`openplush-mesh`), cut along its seams into charts (`openplush-seams`), flattened chart by chart (`openplush-flatten`), turned into pattern pieces with seam allowance (`openplush-pattern`), laid out on fabric and pages (`openplush-layout`) and written as SVG or PDF (`openplush-export`). Each stage either hands on output that meets its promise or stops with a typed error naming where the problem is; nothing is quietly fixed. Today `openplush-pipeline`, the one public API, exposes validate, repair and chart; the later stages exist as crate functions that no pipeline stage calls yet ([status](../start/status.md)).

## How it works

The model followed here is the capsule in `fixtures/meshes/capsule.obj`: a closed pill shape of 192 triangles in millimetres, with the seam layout in `fixtures/seams/capsule.json` (two end caps and a tube slit lengthwise).

### Import: bytes to a triangle soup

`openplush-io` reads the file. `obj::read`, `stl::read` and `ply::read` each turn bytes into a `TriangleSoup`: positions, triangles, and for every triangle corner a `SourceCorner` recording which primitive and which vertex of the file it came from. Polygons are triangulated without moving or adding a vertex. The soup promises finite coordinates and in-range indices, and nothing about quality: holes, duplicates and non-manifold edges pass through to the next stage.

What can fail, as `IoError`: `Parse` (the file cannot be read, with the line or byte where the reader can say), `NonFiniteInput` (every vertex with a NaN or infinite coordinate, by its place in the file), and `Mesh` (the triangles do not fit a soup at all). `EngineError` has matching `Parse` and `NonFiniteInput` variants, but the pipeline has no import stage yet: a shell calls the reader directly. glTF and GLB have no reader yet ([status](../start/status.md)).

The capsule reads as one OBJ primitive with 98 vertices and 192 triangles.

### Validate: what is wrong, changing nothing

`openplush-pipeline::validate` runs `openplush-mesh`'s `validate` and returns a `ValidationReport`: every defect repair would meet (unwelded vertices, zero-area and duplicate faces, non-manifold edges and vertices, winding conflicts, inside-out shells, boundary loops, extra components, self-intersections), each located by face, vertex or vertex pair. It never fails and never changes the soup, so a shell can show the maker the defects before asking what to repair.

The capsule's report is clean: one component, no boundary loop, no non-manifold edge.

### Repair: a soup becomes a manifold mesh

`openplush-pipeline::repair` runs `openplush-mesh`'s `into_manifold` with a `RepairPlan`, the steps the maker allowed. It welds close vertices, drops faces that repeat an index or another face, pairs the faces on over-shared edges, orients the faces consistently, splits non-manifold vertices and components, builds the half-edge `ManifoldMesh`, and fills each hole the maker listed with a patch shaped to the surface around it, with a `RepairReport` of every change. This is the only way to get a `ManifoldMesh`, so unrepaired geometry cannot reach a later stage. An edge repair had to split becomes a mandatory seam, which the maker cannot remove (see [seams](seams.md)).

What can fail: `EngineError::Mesh`, carrying the mesh crate's refusal. Examples are `NoTriangles`, a defect whose repair step the plan turned off reaching the build (a `NonManifoldEdge` or `InconsistentOrientation`, say), and a listed hole the fill cannot close: too large (`HoleTooLarge`), running along a split edge's copy (`HoleOnSplit`), or with no patch that keeps every triangle sound (`HoleNotFillable`).

The capsule needs nothing: the mesh has 98 vertices, 288 edges and 192 faces.

### Seams and charts: where the fabric is cut

The maker's seams are a `SeamSet`, a kind on each seam edge; the mesh itself never changes when seams do. `openplush-pipeline::chart` hands the mesh and the seams to `openplush-seams`: it splits the faces into charts (the faces reachable from each other without crossing a seam), checks each one can flatten, cuts each open along its seams into a `ChartMesh`, the flat arrays the flattener reads, and builds the `SewingMap`, which records which chart edges sew to which. What a chart is, and why it must be a disk, is on the [seams](seams.md) page.

What can fail, first failure only and in this order: `Seams` for a seam on an edge the mesh does not have; then, for each chart in ID order, `FoldedChart` (two neighbouring faces lie back over each other, which no flattener can undo) before `ChartNotDisk` (the chart is closed, has more than one boundary, or has a handle), which carries one proposal of extra seams that would fix it when one is found; then `Seams` for a seam chain with nothing to sew to.

With no seams at all, the capsule is one closed chart and the stage refuses it: `chart 0 is not a topological disk`. With the fixture's 27 seam edges it returns three charts, the two caps and the slit tube, and three seam bundles joining their edges.

### Flatten: each chart laid flat

`openplush-flatten` gives every vertex of a chart a flat position. `FlattenSession::new` takes a `ChartMesh`'s arrays, lays the chart's boundary flat with every edge at its exact 3D length, then moves the interior a step at a time to keep each triangle as close to its 3D shape as it can without ever turning one over. `finish` hands out the layout only once it has settled. `metrics` and `summary` measure how far each face is stretched or compressed. Why the boundary is pinned is on the [flattening](flattening.md) page.

What can fail, as `FlattenError`: `EmptyChart` (a chart with no faces), `NotADisk` (a guard, since the chart stage should not let one through), `Degenerate` (a face with no area), `Unjudgeable` (a face whose flat coordinates the exact orientation test refuses), `SolveFailed` (a sparse solve was refused), `BoundaryNotRealizable` (the boundary cannot close at its 3D lengths; it carries how far it misses and a dart that would relieve it) and `Flip` (a face turned over). `EngineError` declares `Flip` and `BoundaryNotRealizable` for this stage, but no pipeline stage calls the flattener yet ([status](../start/status.md)).

The capsule's tube is a cylinder, which unrolls to a rectangle with no distortion. Each cap is a hemisphere, which cannot lie flat at its own size: with its rim pinned, the interior compresses, and its flat area comes out about half its 3D area. That compression is where fabric would gather, and what a dart would relieve.

### Pattern: charts become pieces

`openplush-pattern` turns flat outlines into pieces in millimetres. `scale::size` sets the real-world size: the factor from model units to millimetres and whether the result is within the supported 75 mm to 1000 mm. `Outline::new` accepts only a closed, simple, counter-clockwise polygon. `parity::check` confirms the two sides of a seam agree in length. `allowance::cut_line` grows the sewing line into the cut line, edge by edge (see [sewing concepts](sewing.md)). `PatternPiece` holds both lines, the cut count and the allowed rotations, and `Fabric` checks a fabric profile.

What can fail, as `PatternError`: a seam whose sides differ (`SeamParity`, which means a flattener bug), an outline that crosses or touches itself or winds the wrong way, allowances that split the piece or close off a gap, a cut count of zero or over 100, a fabric profile that contradicts itself, and a model length or scale that is zero, negative or not finite. `EngineError` declares `SeamParity` and `OffsetFailed` for this stage. The step from a flattened chart to its outline, and a pipeline stage for any of this, are not built yet ([status](../start/status.md)).

### Layout: pieces on fabric and pages

`openplush-layout` places the pieces. `shelf::pack` lays every copy of every piece across the fabric roll, by its cut line, into one `Sheet`: the fabric plan. `shelf::pack_templates` lays one template per piece for printing. `pages::tile` covers a sheet with the fewest Letter or A4 pages, overlapping 15 mm, each labelled by row and column, with registration marks on the shared edges.

What can fail, as `LayoutError`: no pieces, a roll width not greater than zero, a piece wider than the roll, a rotation that is not a multiple of 90 degrees, or more than 26 pages across or down.

### Export: bytes the maker prints

`openplush-export` writes what the maker prints. `svg::write` draws one sheet at true scale; `pdf::write` draws a tiled sheet as a PDF with a cover page (scale check, a map of the pages, and any unresolved warnings) and one page per tile, each with its label, registration marks and a scale-check square. Each piece shows a solid cut line and a dashed sewing line. The same input gives the same bytes.

What can fail, as `ExportError`: a placement whose piece was not given (`UnknownPiece`), a piece given twice (`DuplicatePiece`), and warning text the PDF fonts cannot print (`Unprintable`).

### Who calls the stages

The command-line tool and the WASM facade are the shells that will drive this chain. Neither calls a stage yet ([status](../start/status.md)); see [reference/cli.md](../reference/cli.md) and [reference/worker-protocol.md](../reference/worker-protocol.md).

## Why not the alternative

The obvious alternative is one call from file to PDF. The pipeline is staged instead because the maker loops: validation and repair happen once per model, while seams change again and again, and each seam edit needs only the stages after it. Because a seam set is flags on the mesh's edges, an edit leaves the repaired mesh, and every ID in it, untouched, and only charting onwards runs again.

## Decisions

[Constitution, Principles I, III and VII](https://github.com/openplush/openplush/blob/main/.specify/memory/constitution.md) · [pipeline architecture](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/plan.md#pipeline-architecture) · [engine API contract](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/contracts/engine-api.md) · [ADR 0012, export reads layout](../adr/0012-export-reads-layout.md)
