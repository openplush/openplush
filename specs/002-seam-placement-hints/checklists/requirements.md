# Specification Quality Checklist: Seam-Placement Hints

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-13
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

**Status: 16/16 passing.** Planning complete: `plan.md`, `research.md`, `data-model.md`, `quickstart.md`, and `tasks.md` all exist, and the 82 tasks in `tasks.md` trace back to every FR and SC.

### Validation run 3 — 2026-10-13, after planning

No spec changes were needed during planning, which is the outcome you want: the design fit the spec rather than the spec bending to the design. Three things the design *clarified* without changing the spec text, recorded here so a reader of the spec alone is not misled:

- **FR-030 for dart candidates** means the cluster region, not a specific dart polygon. The detector locates where a dart is needed; direction and size come from 001's dart machinery on acceptance (research R1, data-model section 6). Promising a specific dart would mean duplicating seam optimisation inside a hints feature.
- **FR-006's monotonicity** turned out to be a genuine architectural constraint rather than a UI nicety. It forbids percentile and adaptive thresholds entirely (plan H-003), because those are not nested and can remove the hint the maker was examining as sensitivity rises.
- **FR-005's "report none"** splits into two distinct cases the spec treats as one: no candidate scored well, and two candidates scored too closely to choose between. The UI says which (H056), because a maker can act on the difference.

### Validation run 2 — 2026-10-13, after clarification integration

Q1 through Q3 resolved; see the spec's Clarifications section. SC-010 was also fixed in that run: it had said "without scrolling past a stated limit" and never stated the limit, so it failed "success criteria are measurable". Now a hard cap of 50 hints per category, with FR-025 updated to the same number so requirement and criterion cannot drift.

### Validation run 1 — 2026-10-13, at creation

15/16. SC-002 and SC-003 were unquantified ("faster", "quickly") and were given numbers: 30% median improvement, 5 s initial and 1 s re-filter.

### Constitution coverage

| Principle | Covered by |
|---|---|
| I Engine/shell separation | New leaf crate `openplush-hints`; overlay and panel in `apps/web` only |
| II CLI parity | FR-050, task H035 |
| III Invariants as contracts | Detection cannot violate an invariant; acceptance goes through 001's existing validation (H049) |
| IV Test-first oracles | Sphere-yields-zero, cube-yields-12, symmetry-refusal — all independent of the implementation |
| V Determinism | FR-009, SC-008, data-model section 9 |
| VI Units | FR-007, physical thresholds in mm and degrees |
| VII Never silent mutation | FR-010 (borrow-checker enforced), FR-032 |
| VIII Local-first | FR-051, SC-009 |
| IX License | No new dependencies, so no new license surface and no ADR needed |
| X Baseline before cleverness | This feature *is* the deterministic baseline; learned models are explicitly out of scope |

### The one real hole

**SC-001 cannot be measured yet.** The feature's headline claim — that 70% of hand-placed seam length falls within 5 mm of a hint — requires fixtures that ship with known-good hand-made patterns. None exist. That is task H004, it is fixture-lane work rather than code, and it is called out in the plan's trade-offs, the quickstart's "what done means", and H082.

The consequence is worth stating plainly: this feature can pass every test, be fully deterministic, and ship on time while its central premise remains unverified. H082 says to disclose that at release rather than quietly dropping the criterion.

### Not measurable yet, lower stakes

- **SC-002** (30% faster first seam, n ≥ 5) needs recruiting and a protocol — H081. Gates the release claim, not the build.
- **H005's threshold tuning** depends on H001's quadruped fixture. Until it lands, every default threshold in the design is an educated guess, same as 001's fabric preset values.
