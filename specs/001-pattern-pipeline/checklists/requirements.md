# Specification Quality Checklist: 3D Model → Plush Sewing Pattern

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-13 (retroactive — 001 was specified before checklists were in use)
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [ ] No [NEEDS CLARIFICATION] markers remain
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

**Status: 15/16 passing.**

### The one failing item

Five `[NEEDS CLARIFICATION]` markers remain, all deliberate, all with a stated default that applies if nobody decides, and none blocking implementation:

| Marker | Default | Where it bites |
|---|---|---|
| FR-001 STL/PLY import | yes | T052 — a parser either exists or does not |
| FR-061 PDF in v0.1 | yes | T091, already assumed by D-008 |
| FR-030 default seam allowance | 6 mm metric / quarter-inch US | `patternOptions.defaultAllowanceMm` |
| FR-055 pile-length multiple | 3x | `patternOptions.pileAllowanceMultiple` |
| FR-058 stretch-difference threshold | 20 percentage points | `patternOptions.stretchMismatchThresholdPct` |

All five are **settings with defaults already wired into the project schema**, not architecture. Leaving them open costs nothing; the schema carries each default and each is a one-line change. They are listed in the spec's "Open Clarifications" section rather than hidden.

Marking this item as failing is the honest reading: markers do remain. Closing it would require either deciding five things nobody needs to decide yet, or pretending the markers are gone.

### Blocking clarifications — closed

Q1, Q2, and Q3 closed 2026-10-13 and are recorded in the spec's Clarifications section with their consequences traced into the plan's Decision Ledger (D-012 through D-019). Two of them changed the design rather than merely confirming it:

- **Q2 (exact seam parity)** selected a different flattening approach than research R3 had been leaning toward — boundary-pinned rather than free-boundary with a heavy weight. That is D-013, it supersedes D-005, and it introduces an injectivity risk now tracked as spike S7 (T007).
- **Q1 (fabric shell)** removed inflation compensation from the roadmap entirely rather than deferring it, which is why it moved from "v0.3" to the spec's out-of-scope list.

The two follow-up decisions on warning semantics (FR-064, FR-065) came out of noticing that the spec had five "warn when..." requirements and never said what a warning *does*.

### Constitution coverage

| Principle | Where the spec carries it |
|---|---|
| I Engine/shell separation | FR-082 (headless parity) |
| II CLI parity | FR-082 |
| III Invariants as contracts | FR-020, FR-023, and the error/warning split in FR-064 |
| IV Test-first oracles | SC-001, SC-002, SC-007, SC-008 are all written as measurable oracles |
| V Determinism | SC-008 |
| VI Units | FR-006, FR-060 (millimetres, true scale) |
| VII Explicit repair | FR-004, FR-013, FR-022, FR-065 — every proposal needs acceptance |
| VIII Local-first | FR-081, SC-006 |
| IX Free and open | Out of spec scope; lives in the constitution and CONTRIBUTING |
| X Baseline first | Spec requires no learned model anywhere |

### Not measurable yet

- **SC-002** (95% of surface area within ±5% stretch) needs a "reference seams" set per fixture. `fixtures/seams/` is T037; until it exists this criterion cannot be evaluated.
- **SC-004** (usability, n ≥ 5) needs recruiting and a written protocol — T171. It gates the release claim, not the build.
- **SC-003**'s printed tolerance requires a physical printer and a ruler. T170 makes that an explicit step rather than an assumption, because a pattern printing at 97% looks perfect on screen.
