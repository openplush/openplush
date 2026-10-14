# Contract: `openplush-pipeline` public API

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-13 | **Status**: Draft contract

The single surface every shell depends on (Constitution I). `bindings/openplush-cli`, `bindings/openplush-wasm`, and later `bindings/openplush-py` may import `openplush-pipeline` and **nothing else** from `crates/`. The project document API is part of this surface as a re-export, `openplush_pipeline::project` (D-020), so saving and loading a project needs no second dependency. A CI check over `cargo metadata` enforces this.

Signatures here are the contract. Internal crate layout may change freely behind it; this file may not change without a version bump and an entry in the plan's Decision Ledger.

## 1. Shape of every stage

```rust
pub type StageResult<T> = Result<Staged<T>, EngineError>;

pub struct Staged<T> {
    pub value: T,
    pub warnings: WarningSet,   // may be empty; never blocks (FR-064)
}
```

Every stage is a pure function of its inputs. No stage touches the filesystem, the clock, the network, or threads — the shell owns all I/O. Given identical inputs and options, every stage returns byte-identical output on every platform (Constitution V).

**The split that matters**: `Err(EngineError)` means a declared invariant is violated and there is no output. `Ok(Staged { warnings })` means the output is valid but may sew badly. Nothing in this API lets a stage return partial or "fixed up" geometry — Constitution III, and the reason the project exists.

## 2. Stages

```rust
// ── Import ────────────────────────────────────────────────────────────────
pub fn import(bytes: &[u8], opts: &ImportOptions) -> StageResult<TriangleSoup>;
pub fn validate(soup: &TriangleSoup) -> Staged<ValidationReport>;   // never errors: reporting defects is its job
pub fn repair(soup: &TriangleSoup, plan: &RepairPlan) -> StageResult<(ManifoldMesh, RepairReport)>;

// ── Seams ─────────────────────────────────────────────────────────────────
pub fn chart(mesh: &ManifoldMesh, seams: &SeamSet) -> StageResult<(Vec<ChartMesh>, SewingMap)>;
pub fn propose_cuts(mesh: &ManifoldMesh, seams: &SeamSet, f: ChartFailure) -> StageResult<Vec<Proposal>>;

// ── Flatten ───────────────────────────────────────────────────────────────
pub fn flatten(charts: &[ChartMesh], opts: &FlattenOptions) -> StageResult<Vec<Parameterization>>;
pub fn metrics(charts: &[ChartMesh], params: &[Parameterization], fabric: &Fabric) -> Staged<Vec<ChartMetrics>>;
pub fn propose_darts(charts: &[ChartMesh], params: &[Parameterization], fabric: &Fabric) -> StageResult<Vec<Proposal>>;

// ── Orient ────────────────────────────────────────────────────────────────
pub fn direction_field(mesh: &ManifoldMesh, strokes: &[DirectionStroke]) -> StageResult<DirectionField>;
pub fn orient(charts: &[ChartMesh], params: &[Parameterization], field: &DirectionField, fabrics: &FabricAssignment) -> StageResult<Vec<GrainAngle>>;

// ── Pattern ───────────────────────────────────────────────────────────────
pub fn pattern(input: &PatternInput, opts: &PatternOptions) -> StageResult<Vec<PatternPiece>>;

// ── Hints (feature 002) ───────────────────────────────────────────────────
// Listed here because this file is the single engine surface; implemented by
// feature 002. Takes `scale` because hint thresholds are physical, which is
// why resizing the model changes the hints.
pub fn hints(mesh: &ManifoldMesh, scale: Scale, opts: &HintOptions) -> StageResult<HintReport>;

// ── Layout & export ───────────────────────────────────────────────────────
pub fn layout(pieces: &[PatternPiece], opts: &LayoutOptions) -> StageResult<Vec<Sheet>>;
pub fn export_svg(sheets: &[Sheet], warnings: &WarningSet, opts: &ExportOptions) -> StageResult<Vec<u8>>;
pub fn export_pdf(sheets: &[Sheet], warnings: &WarningSet, opts: &ExportOptions) -> StageResult<Vec<u8>>;
```

`export_*` take the accumulated `WarningSet` because FR-064 requires unresolved warnings to print on the first page. Passing it is not optional — a caller cannot produce a pattern that hides its own known problems.

## 3. Sessions (long-running stages)

WASM is single-threaded in v0.1, so `flatten` must be able to yield.

```rust
pub struct FlattenSession { /* opaque */ }

impl FlattenSession {
    pub fn new(charts: &[ChartMesh], opts: &FlattenOptions) -> Result<Self, EngineError>;
    pub fn step(&mut self) -> Result<Progress, EngineError>;
    pub fn cancel(self);
    pub fn finish(self) -> StageResult<Vec<Parameterization>>;
}

pub enum Progress {
    Working { chart: usize, iteration: u32, max_distortion: f64 },
    Done,
}
```

`step()` performs a bounded amount of work and returns. The caller decides when to yield to the event loop. Cancellation is cooperative and leaves no partial output — there is no way to obtain a half-flattened `Parameterization`.

## 4. Incremental recompute

```rust
pub struct Pipeline { /* opaque; memoizes by content hash */ }

impl Pipeline {
    pub fn new(mesh: ManifoldMesh) -> Self;
    pub fn set_seams(&mut self, seams: SeamSet);       // invalidates affected charts only (FR-014)
    pub fn set_scale(&mut self, scale: Scale);         // invalidates `pattern` onward only
    pub fn set_shrink(&mut self, pct: f64);            // same (FR-056, uniform — D-012)
    pub fn set_fabrics(&mut self, a: FabricAssignment);
    pub fn pieces(&mut self) -> StageResult<&[PatternPiece]>;
}
```

Scale and shrink are uniform factors applied at the pattern stage, so neither re-runs flattening. That is the practical payoff of keeping shrink compensation uniform (D-012, D-014).

## 5. Errors

```rust
#[non_exhaustive]
pub enum EngineError {
    Parse        { format: Format, detail: String },
    NotManifold  { edges: Vec<EdgeId>, vertices: Vec<VertexId> },
    ChartNotDisk { chart: ChartId, failure: ChartFailure, proposals: Vec<Proposal> },
    Flip         { chart: ChartId, faces: Vec<FaceId> },
    BoundaryNotRealizable { chart: ChartId, residual_mm: f64 },
    SeamParity   { chain: SeamChainId, a_mm: f64, b_mm: f64, tolerance_mm: f64 },
    OffsetFailed { piece: PieceId, detail: String },
    Unsewable    { piece: PieceId, reason: UnsewableReason },
    Cancelled,
}

pub enum ChartFailure { Closed, MultipleBoundaries { count: u32 }, PositiveGenus { genus: u32 } }
```

Every variant **locates** the problem — an ID the UI can highlight in 3D. An error without a location is useless to a maker staring at a model, so "where" is part of the type.

- `ChartNotDisk` carries `proposals` so the UI can offer the minimal extra seams immediately (FR-013). The error *is* the suggestion.
- `BoundaryNotRealizable` is specific to the boundary-pinned solve (D-013): the prescribed lengths cannot close into a flat polygon. The remedy is a dart proposal; parity is never relaxed to escape it.
- `SeamParity` should be unreachable by construction. It exists as a last-line assertion because a silently mismatched seam is the exact failure mode Constitution III names as worse than a crash. If it ever fires, that is a bug in the flattener, not bad user input.

## 6. Headless parity (Constitution II)

Every function above is reachable from `openplush-cli` as file-in / file-out. A bug report is reproducible as a CLI invocation, and golden tests run through the CLI path rather than around it.

```sh
openplush-cli validate  model.glb --json report.json
openplush-cli pattern   model.glb --seams seams.json --fabric minky --size-mm 300 -o out/
openplush-cli hints     model.glb --size-mm 300 --json hints.json   # feature 002
```

## 7. Stability

Pre-1.0: breaking changes allowed on a minor bump, listed in the plan's Decision Ledger. Post-1.0: semver, with `#[non_exhaustive]` on every public enum so adding a variant is not a breaking change.
