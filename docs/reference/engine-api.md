# Engine API (`openplush-pipeline`)

What the public engine API is **today**, and the facts about it that rustdoc does not show: user-visible strings, ordering rules, wire forms, and stability. Signatures and per-item docs live in rustdoc, which is generated from the code and cannot drift from it:

```zsh
cargo doc -p openplush-pipeline --no-deps --open
```

The [engine API contract](../../specs/001-pattern-pipeline/contracts/engine-api.md) describes the full planned surface. Where this page and the contract disagree about something built, this page and the code are right.

## Surface today

| Item | Kind | Notes |
|---|---|---|
| `StageResult<T>` | type alias | `Result<Staged<T>, EngineError>`: what every stage will return |
| `Staged<T>` | struct | `value: T` plus `warnings: WarningSet`; warnings never turn `Ok` into `Err` |
| `EngineError` | enum, `#[non_exhaustive]` | A violated invariant; no output exists. Variants below |
| `ErrorLocations` | struct | What `EngineError::locations()` returns: the element IDs an error names, by category, sorted |
| `Warning`, `WarningSet`, `WarningCode`, `WarningTarget`, `Remedy`, `NonBlank` | types | The warning channel; see [Warnings](#warnings) |
| `SeamsError` | re-export, enum, `#[non_exhaustive]` | From `openplush-seams`: what its APIs (`ProposalId::from_hex`, `Proposal::new`, `SeamSet`, `SeamGraph::new`, `close_chart`, `mark_opening`) return. Variants below |
| `Format`, `SourceCorner`, `VertexId`, `EdgeId`, `FaceId`, `ComponentId`, `ChartId`, `SeamChainId`, `PieceId`, `Proposal`, `ProposalId`, `ProposalKind`, `SeamKind`, `SeamReason`, `ChartFailure` | re-exports | From `openplush-io`, `-mesh`, `-seams` and `-pattern`, so a shell can name everything an error or warning carries without a second dependency |

No stage function exists yet; each lands with the task that builds its stage.

## Errors

Each `EngineError` variant carries the location of the problem. The messages are part of the API: they are pinned by tests (`crates/openplush-pipeline/tests/error.rs`) and change only deliberately. IDs print as their index; lengths print in millimetres to three decimals.

| Variant | Located by | Message |
|---|---|---|
| `Parse` | `format`, plus the reader's `detail` | `could not read the {format name} file: {detail}` |
| `NonFiniteInput` | every non-finite source vertex (`SourceCorner`) | `{n} source vertices have a NaN or infinite coordinate` |
| `NotManifold` | `edges`, `vertices` | `the mesh is not manifold at {e} edges and {v} vertices` |
| `ChartNotDisk` | `chart`; carries `failure` and the `proposals` that fix it | `chart {chart} is not a topological disk` |
| `Flip` | `chart`, `faces` | `chart {chart} flattens with {n} inverted faces` |
| `BoundaryNotRealizable` | `chart`; carries `residual_mm` and dart `proposals` | `chart {chart}'s seam lengths cannot close flat (residual {r} mm)` |
| `SeamParity` | `chain`, both side lengths, the tolerance | `seam chain {chain}: sides {a} mm and {b} mm differ by more than {t} mm` |
| `OffsetFailed` | `piece` | `seam allowance for piece {piece} failed: {detail}` |
| `ProposalNotOffered` | `id` | `proposal {16 hex digits} is no longer offered` |
| `Cancelled` | nothing: the caller cancelled | `cancelled` |

Format names: `OBJ`, `glTF`, `GLB`, `STL`, `PLY`.

Three methods read an error the way the shell needs it. Each matches every variant explicitly, so a new variant does not compile until it has a code, locations and proposals.

- **`code() -> &'static str`**: the variant name (`"Parse"`, `"ChartNotDisk"`, …, `"Cancelled"`). It is stable on the wire: a released code never changes.
- **`locations() -> ErrorLocations`**: the element IDs in the "Located by" column, by category: `faces`, `edges`, `vertices`, `pieces`, `charts`, `chains`, `source_vertices` (`SourceCorner`s of the imported file) and `proposal` (an `Option<ProposalId>`). Each list is sorted ascending, without duplicates. `Parse` and `Cancelled` give empty locations. A proposal's own edges are not locations: they arrive with the proposal.
- **`proposals() -> &[Proposal]`**: the fixes `ChartNotDisk` and `BoundaryNotRealizable` carry; empty for every other variant.

`SeamsError` is the error of building a proposal, parsing its ID, changing a `SeamSet`, building a `SeamGraph`, closing a chart or marking an opening, not of a stage; its messages are pinned the same way, in `crates/openplush-pipeline/tests/seams_error.rs`:

| Variant | Carries | Message |
|---|---|---|
| `EmptyProposal` | nothing | `a proposal must add at least one seam edge` |
| `MandatorySeam` | the `edge`, printed in Rust `{:?}` (Debug) form: `EdgeId(4)` | `a mandatory seam cannot be removed, replaced, or placed over another seam (edge {edge:?})` |
| `UnknownEdge` | the `edge`, printed in Rust `{:?}` (Debug) form: `EdgeId(9)` | `a seam is on edge {edge:?}, which the mesh does not have` |
| `BadProposalId` | the rejected `text`, printed in Rust `{:?}` (Debug) form: double-quoted, with escapes | `{text:?} is not a proposal id (16 lowercase hex digits)` |
| `ChartNotClosed` | the `chart`, printed in Rust `{:?}` (Debug) form: `ChartId(3)` | `the cuts found for chart {chart:?} do not leave it one disk` |
| `NotASeam` | the `edge`, printed in Rust `{:?}` (Debug) form: `EdgeId(7)` | `edge {edge:?} has no seam to mark` |

## Warnings

A `Warning` has a code, a target, an explanation and a remedy, and cannot be built without all four:

- **Explanation** and **advisory text** are `NonBlank`: `NonBlank::new` returns `None` for empty or whitespace-only text.
- **Remedy** is `Actionable { proposal }` or `Advisory { what_to_change }`, never optional.
- **Targets**: `Face`, `Edge`, `Chain`, `Piece`, `Component`, or `Project`.

**Codes.** `WarningCode` ships with no variants yet; each stage adds the codes it raises. Every code has a kebab-case string (`WarningCode::as_str`) that never changes once released, because acknowledgements are stored by it. Released codes will be listed here.

**Order.** A `WarningSet` yields warnings in one total order, whatever order they were raised in:

1. code, by its **string** (not declaration order, so a new code never reorders a printed summary);
2. target: variant in the order listed above, then ID;
3. remedy: `Actionable` before `Advisory`; proposals by `ProposalId`, then by the rest of their content; advisories by text;
4. explanation.

Two warnings equal in all four are kept once.

## Wire forms

- **`ProposalId`** is a `u64`, sent as exactly 16 lowercase hex digits (`ProposalId::to_hex` / `from_hex`). A JSON number would lose precision above 2^53. `from_hex` refuses any other spelling with `SeamsError::BadProposalId`, so an ID has one form.

## Stability

The versioning policy is in the [contract, section 7](../../specs/001-pattern-pipeline/contracts/engine-api.md#7-stability). As built, every public enum in this API is `#[non_exhaustive]` except `Remedy`, whose two variants are a deliberate closed set, so adding an error variant, warning code or target is not a breaking change.
