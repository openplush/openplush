# Data Model: Import Seams and Fabric Regions from the Source Model

**Feature**: `003-source-seam-import` | **Date**: 2026-10-16

Extends 001's data model (`specs/001-pattern-pipeline/data-model.md`); section references below (§n) are to that file. Field lists are design intent; names may change in implementation, invariants may not.

## 1. Source materials on the soup and the mesh (extends 001 data-model §2)

```rust
pub struct TriangleSoup {
    // existing: positions, triangles, corner_src, corner_uv
    materials: SourceMaterials,
    extra_uv_sets: u32,              // UV sets the importer saw but did not read (FR-004)
}

pub struct SourceMaterials {
    names: Vec<String>,              // MaterialIndex -> name as written in the file
    by_primitive: Vec<Option<MaterialIndex>>,   // SourceCorner::primitive -> material; None = no material
}
```

- **Invariant (new)**: every source primitive has at most one material. The importer owns primitive numbering (`SourceCorner` docs), so it splits primitives at each material change. glTF already does this; OBJ needs spike Q3.
- **Validation**: `TriangleSoup::with_materials` (not `new`, decision S-019) refuses, with a typed `MeshError`, a `by_primitive` shorter than the highest primitive index used, and a `MaterialIndex` not below `names.len()`. As built: [docs/reference/crates.md](../../docs/reference/crates.md). A material is a function of provenance, so no per-face array is needed: a face's material is its first corner's primitive's material.
- **Carried through repair**: `SourceMaterials` and `extra_uv_sets` are file-level, not per-element, so repair moves them unchanged from the soup onto `ManifoldMesh`. `ManifoldMesh` gains the same two fields and read accessors, and repair's output type is unchanged. This changes 001's `ManifoldMesh` (decision S-011).
- **Faces added by repair** (hole fill, T066) have no source primitive: their `corner_src` entries are `None` in the merged `ManifoldMesh` (`Vec<Option<SourceCorner>>`). That is distinct from a source face whose primitive has no material, so a face's material state has three values: a material, no material in the file, or repair-filled. Spec FR-024 assigns repair-filled faces to a region (§3).
- `MaterialIndex` is a `u32` newtype from `id_newtype!` in `openplush-mesh`.

## 2. Reading the surface

| Concept | Defined as | Computed by |
|---|---|---|
| UV cut | Interior edge, UVs on both sides, endpoint UVs differ by more than `UV_CUT` at either end (research R2) | `openplush-seams::source` |
| Material boundary | Interior edge whose two faces both come from the source file (neither is repair-filled) and differ in `Option<MaterialIndex>`. A source face with no material next to one with a material is a boundary; a repair-filled face never is (spec FR-024) | `openplush-seams::source` |
| Projection point | Interior vertex with at least `PROJECTION_MIN_FACES = 5` incident faces, all with UVs, whose corner UVs at that vertex are pairwise more than `UV_CUT` apart (research R2, spec FR-006). Reported only; changes no proposal | `openplush-seams::source` |
| Dead-end tip | Interior vertex of seam-valence 1 in the graph over UV cuts ∪ material boundaries ∪ mandatory seams, excluding seams already in the project (research R3, spec FR-013) | 001's seam graph (T070) |

`UV_CUT = 1e-5` lives in `openplush_geom::epsilon` beside the other tolerances, as the first tolerance that is neither model-relative nor physical. Its doc comment says why it is absolute.

## 3. The import proposal

Who owns what, following the crate DAG: edge detection and the dead-end check are in `openplush-seams` (`source.rs`), because they read the mesh and the seam graph. Regions and name matching are in `openplush-pattern` (`regions.rs`, 001 T137's module), because `Fabric` lives there. `openplush-pipeline` composes them into the types below, which are part of the public API.

```rust
pub struct SourceImport {
    seams: Option<Proposal>,          // ProposalKind::Seam { reason: SeamReason::SourceModel }; Cut and FabricBoundary edges
    darts: Vec<Proposal>,             // one per dead-end chain: ProposalKind::Seam { reason: SeamReason::SourceDart }, Dart edges
    closing: Vec<Proposal>,           // CloseChart proposals from propose_cuts over (existing ∪ seams ∪ darts)
    regions: Vec<SourceRegion>,       // empty if no face has a material (FR-005); else one per material, plus "No material" if any source face lacks one
    new_fabrics: Vec<Fabric>,         // profiles created for unmatched names: minky preset values (FR-023, S-007)
    report: SourceReport,
}

pub struct SourceRegion {
    material: Option<MaterialIndex>,  // None = "No material"
    fabric: FabricId,                 // matched or newly created
    faces: Vec<FaceId>,               // ascending; repair-filled faces included per FR-024
}

pub struct SourceReport {
    uv_set_read: Option<u32>,         // None = model had no UVs
    extra_uv_sets: u32,               // > 0: the report presents FR-004's warning, derived from this count (S-016)
    uv_cut_edges: u32,
    material_boundary_edges: u32,
    shared_edges: u32,                // edges that are both, counted once (FR-021)
    faces_without_uv: u32,
    projection_points: Vec<VertexId>, // ascending; FR-006's hint is shown when non-empty
    filled_areas: Vec<(ComponentId, FabricId)>,  // FR-024: which region each repair-filled area joined
    matched: Vec<(MaterialIndex, FabricId)>,
    created: Vec<(MaterialIndex, FabricId)>,
}
```

**Invariants**
- `seams` holds no edge already in the project's `SeamSet` (`Proposal::new` already refuses that). An edge that is both a UV cut and a material boundary is added once, as `FabricBoundary`: that kind is the more specific one, and both kinds are ordinary seams to the flattener.
- No `Dart` or `Opening` kind ever appears in `seams`, and no `Opening` in `darts`; every edge in `darts` is `Dart` (FR-010, FR-011). Checked by a debug assertion and a property test.
- `seams` and `darts` share no edge: a dead-end chain is in exactly one dart proposal and nowhere else (FR-012). `darts` is ordered by tip, ascending.
- Every face belongs to exactly one `SourceRegion` (SC-005).
- `closing` is empty exactly when every chart under existing ∪ `seams` ∪ `darts` is a disk (US2 scenario 4). That is the set "accept all" applies. Accepting less is partial acceptance, and 001's validation then reports what remains (US2 scenario 3).
- `seams` is `None` when nothing would be added (a model with no UVs and one material), because `Proposal::new` refuses an empty edit.

**FR-024 tie-break**: a repair-filled area (the faces that filled one loop) joins the region bordering the largest share of its perimeter, measured by length. Ties go to the lowest `MaterialIndex`, with "No material" last.

## 3a. Unreviewed fabric warning (FR-025)

A new `WarningCode::UnreviewedFabric` (001 data-model §13), target `Project`, `Remedy::Advisory` ("Pick a preset for, or edit, the fabric '<name>'"). Raised for every fabric whose `fromMaterial` flag is set. **Schema addition**: optional `fabrics[].fromMaterial: boolean` (default `false`), set on profiles created by import and cleared by the first edit. Additive, so v1 files stay valid.

## 4. Dead ends become dart proposals (FR-011, FR-012)

There is no dead-end error. Each chain from a dead-end tip (data-model §2) to the first junction or surface boundary is one `Proposal` in `SourceImport::darts`:
- `ProposalKind::Seam { reason: SeamReason::SourceDart }`, with every chain edge added as `SeamKind::Dart`.
- Its explanation names the tip's location and says the cut stops partway, so it will be sewn as a dart; decline it and fix the cut in the modelling tool if that was not meant.
- `SeamReason::SourceDart` is new, beside `SourceModel`.

On 2026-10-16 this section defined `EngineError::DeadEndCut { tips, chains }`. It was withdrawn on 2026-10-17 (spec Clarifications) before any code existed.

## 5. Re-import (FR-050 to FR-052)

```rust
pub enum OnExisting { Replace, Merge }

pub struct ReimportDiff {
    added_seams: u32,
    placed_seams: u32,                // both modes; old seam edges re-placed, one per old edge however many pieces; 0 with old seams present raises FR-051's hint (S-018)
    removed_seams: u32,               // Replace only
    removed_hand_drawn: u32,          // FR-051's callout; seams not from a prior import
    unplaceable_seams: Vec<ProvenanceEdge>,  // Merge only; located on the OLD mesh (research R5)
    unplaceable_faces: Vec<ProvenanceFace>,
}
```

- Old seams and faces are re-placed by **position** against the new mesh within the weld tolerance (research R5), never by provenance key and never by nearest-edge snapping. An old seam edge also re-places as a path of new edges whose interior vertices lie on it within the tolerance (S-017); a split face does not. The path is walked from the old edge's first endpoint: each step takes the neighbour on the segment that is strictly closer to the far endpoint and nearest the current vertex, ties to the lower `VertexId`, so the result is deterministic. The difference is computed before the maker chooses, so `placed_seams` and the hint are the same under Replace and Merge.
- "Hand-drawn" needs the project to know where a seam came from. **Schema addition**: an optional `origin: "drawn" | "imported" | "proposed"` on each `seams[]` item in `project.schema.json`, defaulting to `drawn`. It is additive, so v1 files stay valid with no migration.

## 6. Persistence

Accepted imports are ordinary `seams[]`, `fabrics[]` and `fabricRegions[]` entries (FR-043): nothing marks them as imported except the optional `origin` above. The import proposal itself is never stored. It is recomputed from the stored source mesh on demand, like every other derived value (§12).
