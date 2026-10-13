# Data Model & Critical Edge Cases

**Feature**: `001-pattern-pipeline` | **Date**: 2026-10-12 | **Status**: Draft

Covers: representations per stage, the half-edge structure, non-manifold handling, chart topology, seam optimization, grainline/nap mapping, allowance, notches, decals, persistence. Confidence tags as in [research.md](./research.md).

## 1. Representations by Stage

| Stage | Type | Accepts | Guarantees |
|---|---|---|---|
| Import | `TriangleSoup` | Anything parseable | Finite coordinates; triangles only; provenance per corner |
| Validate | `ValidationReport` | `TriangleSoup` | Every defect located (face/edge/vertex IDs) |
| Repair | `ManifoldMesh` + `RepairReport` | `TriangleSoup` + `RepairPlan` | Constitution III post-repair invariants |
| Seams | `SeamSet` | `ManifoldMesh` edges | Typed edge flags; no geometry change |
| Chart | `ChartMesh` ×N + `SewingMap` | mesh + seams | Each chart a topological disk, or a typed error with proposed cuts |
| Flatten | `Parameterization` | `ChartMesh` | det J > 0 on every face |
| Pattern | `PatternPiece` ×N | param + fabric + scale | Simple polygons in mm; seam parity within tolerance |
| Layout | `Sheet` ×N | pieces + layout options | No overlaps; rotation sets honored |

**Typestate**: `ManifoldMesh` has no public constructor. The only way to get one is `repair::into_manifold(soup, &plan) -> Result<(ManifoldMesh, RepairReport), RepairError>`. Downstream stages take `&ManifoldMesh`, so unrepaired data cannot reach the flattener by construction.

## 2. Half-Edge Structure

Index-based arenas with newtype IDs — no pointers, no `Rc<RefCell<…>>`. Borrow-checker friendly, cache friendly, trivially serializable, and transferable to the worker as flat buffers.

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HalfEdgeId(u32);
pub struct EdgeId(u32);   // = HalfEdgeId >> 1
pub struct VertexId(u32);
pub struct FaceId(u32);

impl HalfEdgeId {
    #[inline] pub fn twin(self) -> Self { Self(self.0 ^ 1) }   // twins allocated as (2k, 2k+1)
    #[inline] pub fn edge(self) -> EdgeId { EdgeId(self.0 >> 1) }
}

/// Struct-of-arrays layout.
pub struct ManifoldMesh {
    // per vertex
    pos: Vec<DVec3>,
    v_out: Vec<HalfEdgeId>,          // boundary vertices: MUST point to the outgoing boundary half-edge
    // per half-edge
    he_next: Vec<HalfEdgeId>,
    he_prev: Vec<HalfEdgeId>,
    he_origin: Vec<VertexId>,
    he_face: Vec<Option<FaceId>>,    // None ⇒ boundary half-edge
    // per face
    f_he: Vec<HalfEdgeId>,
    // per corner (= per interior half-edge): source attributes
    corner_uv: Vec<Option<DVec2>>,   // original texture coords, preserved through repair
    corner_src: Vec<SourceCorner>,   // provenance: (source primitive, source vertex index)
}
```

**Conventions**
- **Explicit boundary half-edges** (`face = None`): `twin` is always valid, boundary loops are traversable via `next`, no special-casing in iterators.
- **Paired-index twins** remove the `twin` array and give `EdgeId` for free.
- **Boundary vertex rule**: `v_out[v]` is the boundary half-edge if one exists → O(1) `is_boundary(v)` and a deterministic start for one-ring iteration.
- **Per-corner attributes on half-edges**, not vertices. A vertex on a UV seam in the source model has several UVs; one per corner is the only lossless representation.
- `prev` is stored (not recomputed) because cutting and boundary traversal hit it constantly; it costs 4 bytes per half-edge.

**Invariants** (checked by `debug_validate()` in tests and debug builds)
1. `next(prev(h)) == h`, `twin(twin(h)) == h`, `origin(next(h)) == origin(twin(h))`.
2. Interior face loops have length 3.
3. Each vertex's one-ring is a single fan (disk or half-disk) — the manifold condition.
4. Adjacent faces have opposite half-edge directions on their shared edge — consistent orientation.
5. No face has zero area below `ε_area` (see §11).

## 3. Non-Manifold Handling (repair pipeline)

Order matters; each step records entries in `RepairReport`.

| # | Step | Detects | Resolution | Seam consequence |
|---|---|---|---|---|
| 0 | Normalize | n-gons, NaN/∞ | Quads split on shorter diagonal; n-gons ear-clipped on best-fit plane; non-finite faces dropped | — |
| 1 | **Weld** | Coincident vertices | Merge within `ε_weld = 1e-6 × bbox diagonal` via sorted spatial hash (deterministic tie-break by source index). Corner UVs untouched | **Critical for glTF**: exporters split vertices at UV/normal discontinuities; unwelded, the model is topologically shredded into UV islands [solid] |
| 2 | Degenerates & duplicates | Zero-area faces, repeated indices, duplicate faces | Drop degenerates (collapse edge if needed); keep one of same-orientation duplicates; opposite-orientation duplicates ⇒ flag "double-sided sheet" | — |
| 3 | **Non-manifold edges** (> 2 incident faces) | Edge fans | Sort incident faces by dihedral angle around the edge axis; pair angular neighbors with compatible orientation; give each pair its own edge copy (Guéziec et al. 2001) | New boundary edges become **mandatory seams**. Often *intentional*: an ear or fin sandwiched into a body seam → see §4 inserted pieces |
| 4 | **Non-manifold vertices** (bowtie) | Vertex whose faces form > 1 fan | Split into one vertex per fan | Usually separates components (e.g., two cones touching at a tip); reported, no seam |
| 5 | Orientation | Inconsistent winding | BFS per component from the largest face, flipping to agree; component orientation chosen so signed volume > 0 when closed | Non-orientable (Möbius-like) conflict edge ⇒ forced seam |
| 6 | Components | Multiple connected components | Each processed independently. Components with area < 1% of total (or user choice) ⇒ candidate **detail parts** (eyes, noses, buttons) | Detail parts produce placement marks, not pieces (FR-005, FR-042) |
| 7 | Holes | Boundary loops | Classify: user marks as *opening* (keep; hem allowance) or *accidental* (fill: Liepa 2003 minimal-area triangulation + fairing) | Openings are unsewn edges, not seams |
| 8 | Self-intersections | BVH triangle-triangle tests | **Warn only.** Disk charts flatten regardless; intersecting limbs in 3D usually need separate pieces anyway | Suggest seams along intersection curves |
| — | Last resort | Hopeless input | Opt-in voxel remesh via `baby_shark` (pending S1). Destroys UVs and sharp features → explicit warning | All seams/decals must be redrawn |

## 4. Seams, Charts, and Sewing Correspondence

### SeamSet (non-destructive)
```rust
pub enum SeamKind { Cut, Dart, Opening, FabricBoundary, Mandatory /* from repair */ }
pub struct SeamSet { kind: BTreeMap<EdgeId, SeamKind> }
```
The mesh never changes when seams change. Undo/redo is a `SeamSet` diff.

### Seam graph nodes
Seam edges form a graph on mesh vertices. Node classes:
- **Junction**: seam-valence ≥ 3 (three or more seam edges meet).
- **Endpoint on boundary**: seam meets a hole/opening.
- **Free endpoint** (seam-valence 1, interior): **dart tip**.
- **Chain vertex**: seam-valence 2.

A **seam chain** is a maximal path between non-chain nodes. Chains are the unit of notching, labeling ("A3"), and length parity.

### ChartMesh (virtual cut)
Charts = flood fill of faces across non-seam edges. For each chart, build a local half-edge mesh by **duplicating vertices along seam edges inside the chart** — except dart tips, which stay single (a slit, not a separation). Provenance maps:
```rust
pub struct ChartMesh {
    mesh: LocalHalfEdgeMesh,
    vtx_to_global: Vec<VertexId>,
    he_to_global: Vec<HalfEdgeId>,
}
```

### SewingMap — generalized to N sides
Each global seam edge has `k ≥ 2` sides. Standard seams: `k = 2`. **Inserted pieces** (ear sandwiched into a head seam, from step 3 non-manifold edges or user construction): `k = 4` (head-left, ear-front, ear-back, head-right).
```rust
pub struct SeamBundle {
    chain: SeamChainId,
    sides: SmallVec<[ChainSide; 2]>,   // (chart, ordered boundary half-edges, direction)
}
```
MVP UI exposes `k = 2`; the model supports `k > 2` from day one so FR-016 isn't a migration later.

### Chart topology check (pre-flatten)
χ = V − E + F; for a connected surface with genus g and b boundary loops, χ = 2 − 2g − b. Flattenable ⇔ **g = 0, b = 1** (disk, χ = 1). Darts don't change this — a slit extends the single boundary loop.

| Failure | Cause | Auto-proposal (requires user approval — Constitution VII) |
|---|---|---|
| b = 0, g = 0 | Closed sphere-like region, no seam | One cut along the max-distortion geodesic, or a dart pair at highest angle defect |
| b > 1 | Annulus (e.g., a sleeve with both ends cut) | Shortest path connecting boundary loops (Dijkstra; edge cost = length × visibility × concavity weight) |
| g > 0 | Handle (e.g., a mug handle, arm touching body) | Tree-cotree generators shortened greedily (Eppstein 2003; Erickson & Whittlesey 2005) |

## 5. Seam Optimization

Two different problems — don't conflate them:
1. **Improve given seams** (MVP, FR-022): heatmap + suggestions.
2. **Generate seams from scratch** (FR-015, MVP status per Q3).

### Objective (shared)
`E = w_d·D + w_l·L + w_v·V + w_c·C + w_s·S + w_n·N`

| Term | Meaning | Note |
|---|---|---|
| D | Distortion vs. fabric tolerance (Σ area-weighted excess σ) | Fabric-relative, not absolute |
| L | Total seam length | Sewing effort ∝ seam length (Pillow, 2008) |
| V | Visibility | Prefer underside, behind ears, inside limbs (Seamster) |
| C | **Unsupported concavity** | Stuffing pushes fabric outward; a concave crease (neck, between legs) holds only if a seam runs along it. Seams *reward* concave creases |
| S | Symmetry mismatch | Mirror seams across the detected/declared symmetry plane |
| N | Piece count | Fewer pieces = less assembly |

**Hard constraints**: every chart a disk; min piece size (provisional: 20 mm in each principal direction at target scale); min seam curvature radius ≥ seam allowance (else machine sewing fails); dart length ≤ fraction of chart diameter.

### MVP: dart suggestion
1. Find faces whose σ exceeds fabric tolerance; cluster.
2. Within each cluster, find the vertex of max angle defect (Gaussian curvature) — a cone point.
3. Propose a dart from the nearest chart boundary along the shortest path toward that cone point.
4. Re-flatten only that chart; accept the suggestion for display if D drops by ≥ threshold.

### v0.2: generation
- Seed with **D-Charts** (quasi-developable segmentation) — plush panels are near-developable patches, so this matches the domain.
- Refine with **OptCuts**-style alternation: flatten ↔ topology moves (extend/retract cuts, merge), accept moves that lower E.
- Symmetrize: detect bilateral plane (PCA → reflection-ICP refinement) or take user's; mirror seams; dedupe mirrored pieces (FR-034).

## 6. Grainline and Nap Mapping

Fabric is a rigid plane: within one piece, grain and nap are **constant in 2D**. The job is to choose each piece's rotation so that the constant fabric direction best matches a direction field on the 3D surface.

### Direction field on the surface
- **Nap (pile with head/tail)**: a **vector field** (1-RoSy). 180° matters.
- **Grain without nap**: a **line field** (2-RoSy). 180° equivalent.
- Input: user strokes ("fur flows this way") as sparse face constraints.
- Solve: smoothest field with soft alignment to constraints — Knöppel et al. 2013; a sparse linear system in complex form, same solver path as flattening.
- Default with no strokes: project a global "down/back" vector onto each tangent plane. Singular where normal ∥ vector (top of head) — interpolate across.

### Per-piece grain angle
For each face f in the chart, express the 3D direction in the face's tangent basis, push forward through the face Jacobian: `d₂(f) = normalize(J_f · d₃(f))`, angle θ_f.

- **Vector (nap)**: θ* = atan2(Σ a_f sin θ_f, Σ a_f cos θ_f).
- **Line (grain)**: average doubled angles 2θ_f, then halve.
- **Coherence** R = |Σ a_f e^{iθ_f}| / Σ a_f ∈ [0, 1] (use 2θ for line fields). Low R ⇒ nap direction varies visibly across the piece ⇒ warn and suggest a split along the field's high-gradient curve (FR-052). Provisional thresholds: warn R < 0.85, suggest split R < 0.7.
- Rotate the piece by −θ* so the fabric's warp/nap axis maps to pattern +Y (printed arrows).

### Cross-seam continuity
At every seam bundle, compare the mapped nap directions on each side at matched arc-length samples. A large jump means fur visibly changes direction across the seam → report.

### Constraints passed to layout
| Fabric | Allowed rotations | Mirrored copies |
|---|---|---|
| Napped (faux fur, velvet, minky with pile) | {0°} (± small tolerance, provisional 3°) | Reflect across the axis **parallel** to the nap — preserves nap direction |
| Non-napped, no directional print | {0°, 180°} | Either axis |
| Bias-cut woven | {45°, 225°} | Parallel to grain |

`jagua-rs` supports reflection about specified local axes, which maps directly to mirrored pieces (S4 confirms discrete rotation sets).

### Anisotropic stretch (v0.2)
Once θ* is fixed, the fabric's stretch tensor `M` is known in the piece frame → anisotropic re-flatten (research R3). Example: knit-backed faux fur stretches mostly crosswise, so the solver may place compression along that axis but not along the nap.

## 7. Seam Allowance

- **Per-edge allowance** (seam default, per-seam override, opening allowance, hem for unsewn edges) breaks single-distance polygon offset. Construct instead: union of the sewing polygon with one strip per boundary segment (segment ⊕ rectangle of that segment's allowance, plus round/miter caps) via `i_overlay`. Result is the cut line.
- **Corners**: MVP uses miter joins with a miter limit (prevents spikes at acute corners). v0.2: drafting-standard **reflected corners** — allowance ends shaped as the mirror of the adjacent seam's allowance so edges align when pressed [likely].
- **Tight curves**: where sewing-line curvature radius < allowance, the offset self-intersects; the union resolves it, and the piece gets **clip marks** (concave) or **notch-out marks** (convex) at a spacing proportional to curvature (FR-035).
- **Robustness**: simplify input polygons first; catch `i_overlay` panics (known issue #74) at the stage boundary and convert to a typed error with the offending piece ID; fuzz in CI (S3).

## 8. Notches and Labels

- Parametrize each chain side by arc length. Notches at both chain ends (junctions) plus every `Δ` mm (provisional 50 mm) along the chain.
- Place side B's notches at the **same fractional arc length** as side A. If lengths differ within tolerance, label the chain "ease" and distribute proportionally.
- **Pairing codes** must be unambiguous: chain label (A3) printed at the midpoint on both sides; notch multiplicity (single/double/triple) differs between adjacent chains on the same piece, so a sewist can't rotate a piece and match the wrong edge.
- Piece label block: ID, name, cut quantity, mirror flag, fabric, grain arrow, nap arrow, scale reference.

## 9. Markings, Detail Parts, Texture

- **Surface-attached storage**: markings are stored as barycentric coordinates on faces of the `ManifoldMesh` — not in pattern space. They survive re-seaming and re-flattening, and map through each chart's provenance.
- **Crossing seams**: a marking is clipped per chart; matching alignment ticks are added on both sides of the seam.
- **Detail parts**: component centroid and contact footprint are closest-point projected onto the main surface → placement mark (center cross + outline) on the piece beneath. Safety-eye holes get a distinct mark.
- **Texture transfer (v0.2)**: per piece, rasterize triangles in pattern space at print DPI, sampling the source texture with the preserved corner UVs. Bleed into the allowance by dilation so print-on-fabric has no white edge. Embedded in SVG as an `<image>` under a `<clipPath>` of the cut line.

## 10. Units and Scale
- `ImportOptions.units` declared or inferred (heuristic on bbox size) and confirmed by the user.
- `Scale` is a single uniform factor applied at the pattern stage. Flattening output is in model units × scale; physical constraints (allowance, min piece size, min curvature radius) are evaluated in mm after scaling.

## 11. Determinism and Tolerances
- Arenas iterate in index order; maps are `BTreeMap`; sorts are stable with total-order tie-breaks.
- All epsilons are **relative to bbox diagonal** unless physical: `ε_weld = 1e-6`, `ε_area = 1e-12` (relative², area), flip threshold det J > 1e-10 (relative). Physical tolerances (seam parity, allowance) are absolute mm.
- Solver: fixed ordering for the symbolic factorization; fixed iteration caps and convergence tests.

## 12. Persistence (project file)
- JSON document + embedded binary mesh (glb-style chunk). `schemaVersion` with forward migrations in `openplush-project`.
- **Stable references**: seams and markings reference edges/faces by keys derived from **source provenance** (sorted pair of canonical source-vertex IDs; face = sorted source-corner triple) — not by `EdgeId`, which changes if repair settings change.
- Stored: source mesh, `RepairPlan` (re-applied deterministically), `SeamSet`, fabrics and region assignments, direction strokes, markings, scale, options.
- Not stored: anything recomputable (charts, parameterizations, pieces). Optional cache block keyed by content hash for fast reopen.

## 13. Contracts to Write Next (`contracts/`)
1. `engine-api.md` — `openplush-pipeline` functions, inputs/outputs, error enum.
2. `worker-protocol.md` — message types, handle lifetime, transferables, cancellation.
3. `project.schema.json` — JSON Schema for the project document, v1.
