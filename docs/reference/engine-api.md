# Engine API (`openplush-pipeline`)

<!-- index: the public `openplush-pipeline` surface as built, error messages, warning order, wire forms. -->

What the public engine API is **today**, and the facts about it that rustdoc does not show: user-visible strings, ordering rules, wire forms, and stability. Signatures and per-item docs live in rustdoc, which is generated from the code and cannot drift from it:

```zsh
cargo doc -p openplush-pipeline --no-deps --open
```

The [engine API contract](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/contracts/engine-api.md) describes the full planned surface. Where this page and the contract disagree about something built, this page and the code are right.

## Surface today

| Item | Kind | Notes |
|---|---|---|
| `StageResult<T>` | type alias | `Result<Staged<T>, EngineError>`: what every stage will return |
| `Staged<T>` | struct | `value: T` plus `warnings: WarningSet`; warnings never turn `Ok` into `Err` |
| `EngineError` | enum, `#[non_exhaustive]` | A violated invariant; no output exists. Variants below |
| `ErrorLocations` | struct | What `EngineError::locations()` returns: the element IDs an error names, by category, sorted |
| `import(files)` | function | `StageResult<TriangleSoup>`: the primary file read by the reader its extension names; see [Stages](#stages) |
| `ImportFiles`, `ImportFile` | structs | What `import` takes: the `primary` file and the `others` beside it, each a `name` (a relative path with `/` separators) and its `bytes` |
| `validate(soup)` | function | `Staged<ValidationReport>`: every defect repair would meet, changing nothing; see [Stages](#stages) |
| `repair(soup, plan)` | function | `StageResult<(ManifoldMesh, RepairReport)>`; see [Stages](#stages) |
| `chart(mesh, seams)` | function | `StageResult<(Vec<ChartMesh>, SewingMap)>`; see [Stages](#stages) |
| `flatten(mesh, charts)` | function | `StageResult<Vec<Parameterization>>`; see [Stages](#stages) |
| `Parameterization` | struct | One chart laid flat: `chart()` and `uv()`, one flat position per chart vertex |
| `pattern(input, opts)` | function | `StageResult<Vec<PatternPiece>>`; see [Stages](#stages) |
| `PatternInput` | struct | What `pattern` reads, all borrowed from one run: `mesh`, `charts`, `sewing`, `layouts` |
| `PatternOptions` | struct | What `pattern` applies: `scale` (a `Scale`) and `fabric` (a `Fabric`) |
| `size(mesh, target)` | function | `StageResult<Scale>`: the scale for the maker's finished size; see [Stages](#stages) |
| `layout(pieces, opts)` | function | `StageResult<Vec<Sheet>>`: the print, one template per piece; see [Stages](#stages) |
| `export_svg(sheet, pieces, warnings)`, `export_pdf(sheet, pieces, paper, warnings)` | functions | `StageResult<Vec<u8>>`: the sheet as SVG, or tiled as PDF, with every warning printed; see [Stages](#stages) |
| `LayoutOptions`, `Sheet`, `Placement`, `Paper` | re-exports | From `openplush-layout`: the roll width, gap and mirroring (`LayoutOptions::new`), what `layout` returns and where it put each piece, and Letter or A4 |
| `LayoutError`, `ExportError` | re-exports, enums, `#[non_exhaustive]` | From `openplush-layout` and `-export`: the refusals `EngineError::Layout` and `Export` carry. Their variants are those crates' own |
| `SizeTarget` | re-export, enum | From `openplush-pattern`: `LongestDimension(Mm)`, or `Reference { model_length, finished }`, one measured distance in model units and what it finishes at |
| `Scale`, `Mm` | re-exports | From `openplush-geom`: millimetres per model unit, `Scale::mm_per_model_unit`, and a checked length in millimetres |
| `Fabric`, `FabricSpec`, `FabricCategory`, `FabricId`, `StretchDirection` | re-exports | From `openplush-pattern`: a fabric profile, built by `Fabric::new` from a `FabricSpec`, which refuses an unusable one |
| `PatternPiece`, `Outline`, `CutCount` | re-exports | From `openplush-pattern`: what `pattern` returns, a piece's sewing and cut lines in millimetres, and how many copies to cut |
| `PatternError` | re-export, enum, `#[non_exhaustive]` | From `openplush-pattern`: the refusal `EngineError::Pattern` carries. Its variants are the pattern crate's own |
| `Pipeline` | struct | The session: one repaired mesh, the charts its current seams make, and each chart's layout, stepped and remembered; see [Session](#session) |
| `Progress` | enum, `#[non_exhaustive]` | What `Pipeline::flatten_step` left behind: `Working { chart, iteration, energy }`, `ChartDone { chart }` or `Done` |
| `FlattenError` | re-export, enum, `#[non_exhaustive]` | From `openplush-flatten`: the refusal `EngineError::Flatten` carries. Its variants are the flattener's own |
| `DVec2` | re-export | From `openplush-geom`: the type of a flat position in `Parameterization::uv` |
| `TriangleSoup`, `ValidationReport`, `RepairPlan`, `LoopRef`, `RepairReport`, `ManifoldMesh`, `SeamSet`, `ChartMesh`, `SewingMap`, `SeamBundle`, `ChainSide` | re-exports | What the stages take and return, from `openplush-mesh` and `-seams` |
| `DuplicateOrientation`, `Unmeasurable` | re-exports | From `openplush-mesh`: the enums `ValidationReport`'s duplicates and `unmeasurable()` carry |
| `MeshError`, `MeshElement`, `HalfEdgeId`, `CornerArray`, `Counted`, `MaterialFault` | re-exports | From `openplush-mesh`: the refusal `EngineError::Mesh` carries and the types its variants name. Variants below |
| `Warning`, `WarningSet`, `WarningCode`, `WarningTarget`, `Remedy`, `NonBlank` | types | The warning channel; see [Warnings](#warnings) |
| `SeamsError` | re-export, enum, `#[non_exhaustive]` | From `openplush-seams`: what its APIs (`ProposalId::from_hex`, `Proposal::new`, `SeamSet`, `SeamGraph::new`, `close_chart`, `mark_opening`, `check_fold`, `SewingMap::new`, `shortest_path`, `mark_dart`) return, and what `chart` wraps in `EngineError::Seams`. Variants below |
| `Format`, `SourceCorner`, `VertexId`, `EdgeId`, `FaceId`, `ComponentId`, `ChartId`, `SeamChainId`, `PieceId`, `Proposal`, `ProposalId`, `ProposalKind`, `SeamKind`, `SeamReason`, `ChartFailure` | re-exports | From `openplush-io`, `-mesh`, `-seams` and `-pattern`, so a shell can name everything an error or warning carries without a second dependency |

## Stages

`import`, `validate`, `repair`, `chart`, `flatten`, `size`, `pattern`, `layout`, `export_svg` and `export_pdf` exist; each later stage lands with the task that builds it.

**`import(&ImportFiles)`** reads the primary file with the reader its extension names: the extension of the name's final `/`-separated segment, as `std::path::Path::extension` reads it, compared ASCII case-insensitively. `obj` is OBJ, `stl` STL, `ply` PLY, `gltf` and `glb` glTF, and `fbx` FBX; a `.gltf` file that is GLB inside is read as GLB. Any other name, and one with no extension (`model`, `.obj`, `model.`), is `UnknownFormat`. The bytes are never sniffed for the reader. A reader's refusal is `Parse`, naming that reader, `MissingResource`, `NonFiniteInput`, or `Mesh`. A glTF buffer `uri` is percent-decoded and resolved against the primary's directory, and the file of exactly that name in `others` is read. A `uri` with a scheme, an absolute path, a bad escape, or `..` above the primary's directory is `Parse`. One with no matching file is `MissingResource`. Other files are ignored. A glTF node `rotation` `[x, y, z, w]` whose squared length differs from 1 by more than `UNIT_QUATERNION_TOLERANCE` (1e-6) is `Parse`, with the detail `node {n}: the rotation [x, y, z, w] is not a unit quaternion (its squared length is {s})`; it is never normalised. A node `matrix` is read as written, scale included. FBX is read only as binary FBX 7.x: an ASCII FBX file is `Parse`, with the detail `the file is ASCII FBX, which is not read: export it as binary FBX`, and so is another binary version. Its models under the document's root are drawn with FBX's full transform stack and each model's geometric transform, in the file's axis system turned to Y up; its units stay the file's. What the glTF or FBX reader does not apply is one `import-ignored` warning on the project: skins (read in the bind pose), morph targets (read as the base mesh), animations, point and line primitives, and, for FBX, deformers (read in the rest pose), animation stacks and geometry that is not a polygon mesh. Otherwise its warnings are empty.

**`validate(&TriangleSoup)`** is the mesh crate's `validate`: a `ValidationReport` of every defect, never an error and never a change. Its warnings are empty.

**`repair(&TriangleSoup, &RepairPlan)`** is the mesh crate's `into_manifold` and nothing else: the repaired mesh, which carries repair's copy groups, and the report of every change. A refusal is `Mesh`, in the mesh crate's order. Its warnings are empty.

**`chart(&ManifoldMesh, &SeamSet)`** splits the mesh along the seams into charts, each cut open along its seams for flattening, with the `SewingMap` of their seam sides. A mesh from `into_manifold` carries repair's copy groups (`ManifoldMesh::edge_copies`), which the map uses to sew a split edge's copies; the stage takes no others. It returns the first failure and no partial output, checking in this order: a seam on an edge the mesh lacks (`Seams`); then each chart in ID order, folded (`FoldedChart`) before not a disk (`ChartNotDisk`, with the one `CloseChart` proposal found, or none when no cut was found that leaves a disk); then a seam chain with no partner, or a copy group that does not line up (`Seams`). Its warnings are empty.

**`flatten(&ManifoldMesh, &[ChartMesh])`** lays each chart flat, in the order given, stepping its `FlattenSession` until it settles, and returns one `Parameterization` per chart: `chart()` and `uv()`, one flat position per chart vertex, indexed like `ChartMesh::xyz` and in the model's units. The mesh, the one the charts were cut from, is read only to name a dart's edges. It returns the first chart's refusal and no layout for any chart: `Flip`; `BoundaryNotRealizable`, whose dart becomes one `Dart` proposal of the mesh edges it walks, each found through a chart face so a split edge's copy inside the chart is the one named (a dart with no edge offers none); or `Flatten` carrying any other `FlattenError`. Each chart's `FlattenSession::finish` also says why its flatten stopped, a `StoppedBy` from `openplush-flatten`: `IterationCap` (the 200-iteration cap, also when that last iteration gained too little to go on), `Settled` (an iteration lowered the energy by less than 1e-9 of its value) or `NoDescent` (no step tried lowered it, or the chart has no interior vertex). A chart that stopped at the cap raises one `flatten-iteration-cap` warning on its piece; no other stop warns. Each layout keeps its area-weighted mean compression deficit (`Summary::compression_deficit`) for `pattern` to judge against the fabric; a layout its `Summary` cannot measure is refused as `Flatten`.

**`size(&ManifoldMesh, SizeTarget)`** returns the scale, in millimetres per model unit, that makes the model the size the maker asked for. The model's longest dimension is the longest side of the mesh's axis-aligned bounding box, in the file's axes. `LongestDimension(mm)` makes that side `mm` long. `Reference { model_length, finished }` makes a distance of `model_length` units measured on the model `finished` long. When the longest dimension finishes under 75 mm or over 1000 mm (both ends supported), it raises one `size-out-of-range` warning and still returns the scale. A reference length that is zero, negative or not finite is `Pattern { piece: None, BadModelLength }`; a scale or finished size that is zero or not finite is `Pattern { piece: None, BadScale }`. Both locate nothing.

**`pattern(&PatternInput, &PatternOptions)`** makes one `PatternPiece` per chart, in chart order, piece `n` from chart `n`. The input is one run's mesh, charts, sewing map and layouts. The sewing line is the chart's one boundary loop, in millimetres: each flat position goes through `Scale::point_to_mm`, the one conversion of a flat position, and the loop runs with the interior on its left, so counter-clockwise, from the first boundary side, in face order, of the lowest chart face that has one. The cut line is the sewing line grown by the fabric's `default_allowance` on every edge. Each piece is cut once as printed, and may turn by the fabric's `rotations_deg()`. It returns the first failure and no pieces, checking in this order: inputs that do not belong together (`StageInputMismatch`: a layout that is not its chart's, at its index, with one position per chart vertex, or a sewing side outside the charts given); then every bundle in lead-chain order, each side's flat length against its first side's, summed in model units and converted once with `Scale::to_mm` (`SeamParity`, a flattener bug); then per chart, the sewing line and then the cut line (`Pattern`, naming the piece). It raises one `piece-compressed` warning on each piece whose layout needs more stretch than the fabric gives; see [Warnings](#warnings).

**`layout(&[PatternPiece], &LayoutOptions)`** is the print: `shelf::pack_templates`, one template of every piece on a roll `LayoutOptions`' width, in one `Sheet` as wide as its pieces reach. A refusal is `Layout`, naming the piece for `PieceTooWide` and `UnsupportedRotation` and nothing for `NoPieces` and `BadRollWidth`. Its warnings are empty.

**`export_svg(&Sheet, &[PatternPiece], &WarningSet)`** is `svg::write`: the sheet at true scale, 1 user unit to 1 mm, with every warning in the set printed under it as summary rows. **`export_pdf(&Sheet, &[PatternPiece], Paper, &WarningSet)`** tiles the sheet on `paper` with `pages::tile` and calls `pdf::write`: a cover with the scale check, the page map and the summary rows, then one page per tile. The pieces are the ones the sheet places; a `Sheet` holds placements, not outlines. A sheet needing more than 26 pages across or down is `Layout` (`TooManyPages`), naming nothing. A placement whose piece was not given, or a piece given twice, is `Export`, naming the piece. Their warnings are empty.

**Summary rows.** Both exports print every warning in the set, in its order, acknowledged or not: "unresolved" means not fixed. A row is the code's string; the target in words: `face {n}`, `edge {n}`, `seam chain {n}`, `piece {n}`, `component {n}`, or `the whole pattern`; the explanation; and the remedy's text, an `Actionable` proposal's explanation or an `Advisory`'s `what_to_change`. Each prints as `{n}. {code}: {target}`, then the explanation, then `Fix: {remedy}`. Every engine warning's text is printable ASCII, so `ExportError::Unprintable` is a bug if it ever appears.

## Session

`Pipeline` holds what a shell edits between stage calls, so a seam edit flattens again only the charts it changed. It owns one repaired mesh and its current seam set.

- **`Pipeline::new(ManifoldMesh)`** starts with no seams: `charts()` is `chart(&mesh, &SeamSet::new())`.
- **`set_seams(&SeamSet) -> Vec<ChartId>`** runs the `chart` stage again and returns, in ID order, each chart whose content no chart of the previous seams had; with the same seams again it returns none, and when the chart stage refuses, none. It drops the flatten in progress, any refusal kept, and the layout of every chart the new seams no longer make; a chart they leave as it was keeps its layout.
- **`charts() -> &StageResult<(Vec<ChartMesh>, SewingMap)>`** is the `chart` stage's result for the current seams, a refusal included.
- **`flatten_step() -> Result<Progress, EngineError>`** takes one `FlattenSession` step on the lowest-ID chart without a layout, starting its session on the first step. It returns `Working { chart, iteration, energy }` while that chart's flatten may take more steps (`energy` is the area-weighted mean symmetric Dirichlet energy, 4 for an isometry), `ChartDone { chart }` on the step that lays a chart flat while another still has no layout, and `Done` once every chart has one, then on every later call. When the chart stage refused, it returns that refusal. A chart's refusal is mapped as `flatten` maps it (`Flip`, `BoundaryNotRealizable` or `Flatten`) and kept: every later `flatten_step` and `parameterizations` returns the same error, steps no other chart and retries nothing, until the next `set_seams`.
- **`parameterizations() -> StageResult<Vec<Parameterization>>`** steps the rest to the end and returns every chart's layout in chart order, byte-identical to what `flatten` returns for the same charts, with the same warnings: a chart laid from the memo raises `flatten-iteration-cap` as it did when it was flattened. It raises no `piece-compressed`, which needs the fabric: `pattern` raises it. `flatten_step` and its `Progress` say nothing of why a chart stopped.
- **`proposals() -> Vec<Proposal>`** lists what the current state offers, each `ProposalId` once, the first seen kept: the cuts the chart stage's `ChartNotDisk` carries, the darts of a kept `BoundaryNotRealizable`, then each `Actionable` remedy of the chart stage's warnings. A session whose charts all flatten offers nothing.
- **`preview(ProposalId) -> Result<SeamSet, EngineError>`** is the current seam set with the proposal's edges added; an edge that already has a seam keeps its kind. The session is unchanged.
- **`apply_proposal(ProposalId) -> Result<Vec<ChartId>, EngineError>`** sets `preview`'s seam set as `set_seams` does and returns what `set_seams` returns. An ID `proposals` does not offer, one just applied included, is `ProposalNotOffered` from both methods, and nothing changes.

A layout is remembered under a key, 64-bit FNV-1a (offset basis `0xcbf29ce484222325`, prime `0x100000001b3`) over the chart's `ChartMesh`: each face's `FaceId` and three local corners as 4 bytes little-endian, then each local position's `x`, `y` and `z` bits as 8 bytes little-endian. The key only finds a candidate: a layout is used only for a `ChartMesh` equal to the one it was laid out for, so two charts whose keys collide each get their own layout. Each layout is kept with the `StoppedBy` its session finished with. Dropping a `Pipeline` drops the flatten in progress; no half-flattened layout is ever reachable.

## Errors

Each `EngineError` variant carries the location of the problem. The messages are part of the API: they are pinned by tests (`crates/openplush-pipeline/tests/error.rs`) and change only deliberately. IDs print as their index, except in `Mesh` and `Seams`, which print their inner error's message; lengths print to three decimals, in millimetres, except `BoundaryNotRealizable`'s residual, which is in model units because flattening runs before the scale.

| Variant | Located by | Message |
|---|---|---|
| `Parse` | `format`, plus the reader's `detail` | `could not read the {format name} file: {detail}` |
| `UnknownFormat` | nothing; carries the primary file's `name` | `cannot read "{name}": no reader for this file type` |
| `MissingResource` | nothing; carries the `uri` as the file writes it | `the file refers to "{uri}", which was not supplied beside it` |
| `NonFiniteInput` | every non-finite source vertex (`SourceCorner`) | `{n} source vertices have a NaN or infinite coordinate` |
| `Mesh` | whatever the inner `MeshError` names, in the soup given to repair (no mesh exists when repair refuses; only `BrokenInvariant`, a bug-class check of debug builds, names the half-built mesh's IDs): its faces, its vertices, a hole loop by its `SourceCorner` in `source_vertices`, a broken invariant's element (a half-edge as its edge); a refusal about the soup as a whole names none | the inner error's message, unchanged, as in the `MeshError` table below |
| `ChartNotDisk` | `chart`; carries `failure` and the `proposals` that fix it | `chart {chart} is not a topological disk` |
| `FoldedChart` | `chart`, `edge` (the lowest edge it folds across) | `chart {chart} lies back over itself across edge {edge}` |
| `Seams` | whatever the inner `SeamsError` names: its `edge`, `chart`, `chain` or `vertex` IDs; a refusal about a whole proposal or dart (`EmptyProposal`, `EmptyDart`, `BadProposalId`) names none | the inner error's message, unchanged, with IDs in Debug form as in the `SeamsError` table below |
| `Flatten` | the inner `FlattenError`'s `chart` and, where it has one, its `face` | the inner error's message, unchanged |
| `Flip` | `chart`, `faces` | `chart {chart} flattens with {n} inverted faces` |
| `BoundaryNotRealizable` | `chart`; carries `residual` (model units) and dart `proposals` | `chart {chart}'s seam lengths cannot close flat (residual {r} in model units)` |
| `SeamParity` | `chain`, both side lengths, the tolerance | `seam chain {chain}: sides {a} mm and {b} mm differ by more than {t} mm` |
| `Pattern` | `piece`, when the refusal is about one; carries the `PatternError` whole | the inner error's message, unchanged |
| `StageInputMismatch` | `chart`, the first whose inputs do not match | `chart {chart}: the layouts or sewing map given do not belong to the charts given` |
| `Layout` | the inner `LayoutError`'s `piece`, for `PieceTooWide` and `UnsupportedRotation`; nothing for `NoPieces`, `BadRollWidth` and `TooManyPages` | the inner error's message, unchanged |
| `Export` | the inner `ExportError`'s `piece`, for `UnknownPiece` and `DuplicatePiece`; nothing for `Unprintable` | the inner error's message, unchanged |
| `ProposalNotOffered` | `id` | `proposal {16 hex digits} is no longer offered` |
| `Cancelled` | nothing: the caller cancelled | `cancelled` |

Format names: `OBJ`, `glTF`, `GLB`, `STL`, `PLY`, `FBX`.

Three methods read an error the way the shell needs it. Each matches every variant explicitly, so a new variant does not compile until it has a code, locations and proposals.

- **`code() -> &'static str`**: the variant name (`"Parse"`, `"ChartNotDisk"`, …, `"Cancelled"`). It is stable on the wire: a released code never changes.
- **`locations() -> ErrorLocations`**: the element IDs in the "Located by" column, by category: `faces`, `edges`, `vertices`, `pieces`, `charts`, `chains`, `source_vertices` (`SourceCorner`s of the imported file) and `proposal` (an `Option<ProposalId>`). Each list is sorted ascending, without duplicates. `Parse`, `UnknownFormat`, `MissingResource` and `Cancelled` give empty locations. A proposal's own edges are not locations: they arrive with the proposal.
- **`proposals() -> &[Proposal]`**: the fixes `ChartNotDisk` and `BoundaryNotRealizable` carry; empty for every other variant.

`MeshError` is the error of building a soup or repairing one; `repair` returns it inside `Mesh`. Its messages are pinned in `crates/openplush-pipeline/tests/mesh_error.rs`. IDs and enums print in Rust `{:?}` (Debug) form: `VertexId(1)`, `FaceId(7)`, `Uvs`.

| Variant | Carries | Message |
|---|---|---|
| `NonFiniteVertex` | the `vertex` | `vertex {vertex:?} has a non-finite coordinate` |
| `IndexOutOfRange` | the `triangle`, the `index`, the `vertex_count` | `triangle {triangle:?} uses vertex index {index}, but there are only {vertex_count} vertices` |
| `CornerCountMismatch` | the `array` (`Sources` or `Uvs`), `expected`, `found` | `expected {expected} per-corner {array:?} (three per triangle), found {found}` |
| `TooLarge` | `what` was counted, the `count` | `{count} {what:?} exceed the u32 index space` |
| `InvalidMaterial` | the lowest `primitive`, the `fault` (`Missing` or `OutOfRange`) | `source primitive {primitive}'s material entry is invalid: {fault:?}` |
| `DegenerateTriangle` | the `triangle` | `triangle {triangle:?} repeats a vertex` |
| `ZeroAreaTriangle` | the `triangle` | `triangle {triangle:?} has (near-)zero area` |
| `DuplicateTriangle` | the later `triangle` and the `first` | `triangle {triangle:?} repeats the vertices of triangle {first:?}` |
| `IsolatedVertex` | the `vertex` | `vertex {vertex:?} is not used by any triangle` |
| `NonManifoldEdge` | the edge's ends `a` and `b` | `edge between {a:?} and {b:?} is shared by more than two triangles` |
| `InconsistentOrientation` | the edge's ends `a` and `b` | `triangles sharing the edge between {a:?} and {b:?} have opposite orientations` |
| `NoTriangles` | nothing | `the soup has no triangles` |
| `UnusableExtent` | nothing | `the soup's extent is zero or too large to derive tolerances from` |
| `UnknownHoleLoop` | the `loop_ref` | `the plan fills the loop {loop_ref:?}, which the mesh does not have` |
| `AmbiguousHoleLoop` | the `loop_ref` | `the plan fills the loop {loop_ref:?}, but more than one hole has that name, so none was filled` |
| `HoleNotFillable` | the `loop_ref` | `the loop {loop_ref:?} has no fill: every triangulation of it reuses an edge, has no area, or does not solve` |
| `HoleOnSplit` | the `loop_ref` | `the loop {loop_ref:?} runs along a split edge, so it is one side of a seam, not a hole` |
| `HoleTooLarge` | the `loop_ref`; carries `vertices` | `the loop {loop_ref:?} has {vertices} vertices, more than the 512 a fill takes` |
| `PairingMismatch` | the `face` and the edge's ends `a`, `b` | `a pairing on the edge between {a:?} and {b:?} names triangle {face:?} wrongly` |
| `NonManifoldVertex` | the `vertex` | `vertex {vertex:?} is non-manifold (its triangles form more than one fan)` |
| `BrokenInvariant` | the `invariant` (1-5, or 0) and the `element` | `half-edge invariant {invariant} is broken at {element:?}` |

`SeamsError` is the error of building a proposal, parsing its ID, changing a `SeamSet`, building a `SeamGraph`, closing a chart or marking an opening; the `chart` stage returns it inside `Seams`. Its messages are pinned the same way, in `crates/openplush-pipeline/tests/seams_error.rs`:

| Variant | Carries | Message |
|---|---|---|
| `EmptyProposal` | nothing | `a proposal must add at least one seam edge` |
| `MandatorySeam` | the `edge`, printed in Rust `{:?}` (Debug) form: `EdgeId(4)` | `a mandatory seam cannot be removed, replaced, or placed over another seam (edge {edge:?})` |
| `UnknownEdge` | the `edge`, printed in Rust `{:?}` (Debug) form: `EdgeId(9)` | `a seam is on edge {edge:?}, which the mesh does not have` |
| `BadProposalId` | the rejected `text`, printed in Rust `{:?}` (Debug) form: double-quoted, with escapes | `{text:?} is not a proposal id (16 lowercase hex digits)` |
| `ChartNotClosed` | the `chart`, printed in Rust `{:?}` (Debug) form: `ChartId(3)` | `the cuts found for chart {chart:?} do not leave it one disk` |
| `NotASeam` | the `edge`, printed in Rust `{:?}` (Debug) form: `EdgeId(7)` | `edge {edge:?} has no seam to mark` |
| `FoldedChart` | the `chart` and the lowest `edge` it folds across, in Debug form: `ChartId(2)`, `EdgeId(17)` | `chart {chart:?} lies back over itself across edge {edge:?}` |
| `UnpairedBoundarySeam` | the `chain`, printed in Rust `{:?}` (Debug) form: `SeamChainId(5)` | `seam chain {chain:?} runs along the mesh boundary with no recorded partner` |
| `UnknownVertex` | the `vertex`, in Debug form: `VertexId(8)` | `vertex {vertex:?} is not one of the mesh's` |
| `NoPath` | `from` and `to`, in Debug form | `no path of edges joins vertex {from:?} to vertex {to:?}` |
| `EmptyDart` | nothing | `a dart needs at least one edge` |
| `NotAWalk` | the `edge`, in Debug form | `edge {edge:?} does not continue the walk` |
| `DartNotAnchored` | the walk's first `vertex`, in Debug form | `a dart starts on a seam or the mesh boundary; neither end does (vertex {vertex:?})` |
| `DartTouchesSeam` | the first such `vertex`, in Debug form | `a dart's tip and inner vertices touch no seam or boundary; vertex {vertex:?} does` |
| `CopyGroupMismatch` | the lead `chain`, in Debug form | `the copies of seam chain {chain:?} do not form chains that sew together` |

## Warnings

A `Warning` has a code, a target, an explanation and a remedy, and cannot be built without all four:

- **Explanation** and **advisory text** are `NonBlank`: `NonBlank::new` returns `None` for empty or whitespace-only text.
- **Remedy** is `Actionable { proposal }` or `Advisory { what_to_change }`, never optional.
- **Targets**: `Face`, `Edge`, `Chain`, `Piece`, `Component`, or `Project`.

**Codes.** Each stage adds the codes it raises. Every code has a kebab-case string (`WarningCode::as_str`) that never changes once released, because acknowledgements are stored by it.

| Code | Raised by | Target | Says |
|---|---|---|---|
| `flatten-iteration-cap` (`FlattenIterationCap`) | `flatten`, `Pipeline::parameterizations` | `Piece`, numbered as the chart | `This piece stopped flattening at the iteration limit before it settled, so the distortion shown may be more than the fabric must take.` Advisory: `Add a seam or a dart in this piece, or keep it as it is: the layout is valid, only less relaxed than it could be.` |
| `import-ignored` (`ImportIgnored`) | `import`, for glTF and FBX | `Project` | `The file has parts the pattern does not follow: {parts}. The pattern is made from the triangles as modelled.`, each part one of `the skin on node {n}, read in its bind pose`, `the morph targets of mesh {m} (node {n}), read as the base mesh`, `animation {a}, not played`, `primitive {p} of mesh {m} (node {n}), which is points or lines`, and for FBX `the deformer "{d}" on model "{m}", read in its rest pose`, `the geometry "{g}" on model "{m}", which is not a polygon mesh`, `animation stack "{s}", not played`, joined by `; ` in walk order, animations and animation stacks last. Advisory: apply the pose or morph in the 3D tool and export again |
| `piece-compressed` (`PieceCompressed`) | `pattern` | `Piece`, numbered as the chart | `This piece must stretch about {n}% to fill its shape, more than this fabric's {m}%, so it will sew up flatter than the model.`, raised when the stretch the layout needs, `1 / (1 − d) − 1` for its area-weighted mean compression deficit `d = Σ A·max(0, 1 − σ₂) / ΣA` (`Summary::compression_deficit`), is over the stretch the fabric gives, `max(min(stretch_grain_pct, stretch_cross_pct) / 100, 0.05)`; `{n}` and `{m}` are those two as whole percents. Advisory: `Add a dart or a seam in this piece.` |
| `size-out-of-range` (`SizeOutOfRange`) | `size` | `Project` | `The pattern's longest dimension finishes at {mm} mm, outside the supported 75 mm to 1000 mm.`, `{mm}` to one decimal, or to as many more as it takes to show the size is outside the range (`1000.04`, never `1000.0`). Advisory: `Resize the model so its longest dimension is between 75 mm and 1000 mm, or keep this size: the pattern is still made, but untested at this size.` |

**Order.** A `WarningSet` yields warnings in one total order, whatever order they were raised in:

1. code, by its **string** (not declaration order, so a new code never reorders a printed summary);
2. target: variant in the order listed above, then ID;
3. remedy: `Actionable` before `Advisory`; proposals by `ProposalId`, then by the rest of their content; advisories by text;
4. explanation.

Two warnings equal in all four are kept once.

## Wire forms

- **`ProposalId`** is a `u64`, sent as exactly 16 lowercase hex digits (`ProposalId::to_hex` / `from_hex`). A JSON number would lose precision above 2^53. `from_hex` refuses any other spelling with `SeamsError::BadProposalId`, so an ID has one form.

## Stability

The versioning policy is in the [contract, section 7](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/contracts/engine-api.md#7-stability). As built, every public enum in this API is `#[non_exhaustive]` except `Remedy`, whose two variants are a deliberate closed set, so adding an error variant, warning code or target is not a breaking change.
