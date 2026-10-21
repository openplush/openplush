# Engine API (`openplush-pipeline`)

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
| `FlattenError` | re-export, enum, `#[non_exhaustive]` | From `openplush-flatten`: the refusal `EngineError::Flatten` carries. Its variants are the flattener's own |
| `DVec2` | re-export | From `openplush-geom`: the type of a flat position in `Parameterization::uv` |
| `TriangleSoup`, `ValidationReport`, `RepairPlan`, `LoopRef`, `RepairReport`, `ManifoldMesh`, `SeamSet`, `ChartMesh`, `SewingMap`, `SeamBundle`, `ChainSide` | re-exports | What the stages take and return, from `openplush-mesh` and `-seams` |
| `MeshError`, `MeshElement`, `HalfEdgeId`, `CornerArray`, `Counted`, `MaterialFault` | re-exports | From `openplush-mesh`: the refusal `EngineError::Mesh` carries and the types its variants name. Variants below |
| `Warning`, `WarningSet`, `WarningCode`, `WarningTarget`, `Remedy`, `NonBlank` | types | The warning channel; see [Warnings](#warnings) |
| `SeamsError` | re-export, enum, `#[non_exhaustive]` | From `openplush-seams`: what its APIs (`ProposalId::from_hex`, `Proposal::new`, `SeamSet`, `SeamGraph::new`, `close_chart`, `mark_opening`, `check_fold`, `SewingMap::new`, `shortest_path`, `mark_dart`) return, and what `chart` wraps in `EngineError::Seams`. Variants below |
| `Format`, `SourceCorner`, `VertexId`, `EdgeId`, `FaceId`, `ComponentId`, `ChartId`, `SeamChainId`, `PieceId`, `Proposal`, `ProposalId`, `ProposalKind`, `SeamKind`, `SeamReason`, `ChartFailure` | re-exports | From `openplush-io`, `-mesh`, `-seams` and `-pattern`, so a shell can name everything an error or warning carries without a second dependency |

## Stages

`import`, `validate`, `repair`, `chart` and `flatten` exist; each later stage lands with the task that builds it.

**`import(&ImportFiles)`** reads the primary file with the reader its extension names: the extension of the name's final `/`-separated segment, as `std::path::Path::extension` reads it, compared ASCII case-insensitively. `obj` is OBJ, `stl` STL, `ply` PLY, and `gltf` and `glb` glTF; a `.gltf` file that is GLB inside is read as GLB. Any other name, one with no extension (`model`, `.obj`, `model.`), and `fbx` until its reader exists, is `UnknownFormat`. The bytes are never sniffed for the reader. A reader's refusal is `Parse`, naming that reader, `MissingResource`, `NonFiniteInput`, or `Mesh`. A glTF buffer `uri` is percent-decoded and resolved against the primary's directory, and the file of exactly that name in `others` is read. A `uri` with a scheme, an absolute path, a bad escape, or `..` above the primary's directory is `Parse`. One with no matching file is `MissingResource`. Other files are ignored. What the glTF reader does not apply is one `import-ignored` warning on the project: skins (read in the bind pose), morph targets (read as the base mesh), animations, and point and line primitives. Otherwise its warnings are empty.

**`validate(&TriangleSoup)`** is the mesh crate's `validate`: a `ValidationReport` of every defect, never an error and never a change. Its warnings are empty.

**`repair(&TriangleSoup, &RepairPlan)`** is the mesh crate's `into_manifold` and nothing else: the repaired mesh, which carries repair's copy groups, and the report of every change. A refusal is `Mesh`, in the mesh crate's order. Its warnings are empty.

**`chart(&ManifoldMesh, &SeamSet)`** splits the mesh along the seams into charts, each cut open along its seams for flattening, with the `SewingMap` of their seam sides. A mesh from `into_manifold` carries repair's copy groups (`ManifoldMesh::edge_copies`), which the map uses to sew a split edge's copies; the stage takes no others. It returns the first failure and no partial output, checking in this order: a seam on an edge the mesh lacks (`Seams`); then each chart in ID order, folded (`FoldedChart`) before not a disk (`ChartNotDisk`, with the one `CloseChart` proposal found, or none when no cut was found that leaves a disk); then a seam chain with no partner, or a copy group that does not line up (`Seams`). Its warnings are empty.

**`flatten(&ManifoldMesh, &[ChartMesh])`** lays each chart flat, in the order given, stepping its `FlattenSession` until it settles, and returns one `Parameterization` per chart: `chart()` and `uv()`, one flat position per chart vertex, indexed like `ChartMesh::xyz` and in the model's units. The mesh, the one the charts were cut from, is read only to name a dart's edges. It returns the first chart's refusal and no layout for any chart: `Flip`; `BoundaryNotRealizable`, whose dart becomes one `Dart` proposal of the mesh edges it walks, each found through a chart face so a split edge's copy inside the chart is the one named (a dart with no edge offers none); or `Flatten` carrying any other `FlattenError`. Its warnings are empty.

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
| `OffsetFailed` | `piece` | `seam allowance for piece {piece} failed: {detail}` |
| `ProposalNotOffered` | `id` | `proposal {16 hex digits} is no longer offered` |
| `Cancelled` | nothing: the caller cancelled | `cancelled` |

Format names: `OBJ`, `glTF`, `GLB`, `STL`, `PLY`.

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
| `import-ignored` (`ImportIgnored`) | `import`, for glTF | `Project` | `The file has parts the pattern does not follow: {parts}. The pattern is made from the triangles as modelled.`, each part one of `the skin on node {n}, read in its bind pose`, `the morph targets of mesh {m} (node {n}), read as the base mesh`, `animation {a}, not played`, `primitive {p} of mesh {m} (node {n}), which is points or lines`, joined by `; ` in walk order, animations last. Advisory: apply the pose or morph in the 3D tool and export again |

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
