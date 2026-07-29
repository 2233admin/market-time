# Specification Quality Checklist: Venue Session State

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-07-29
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

## Constitution Alignment (Mark Time v1.2.0)

- [x] Principle I (Evidence-Backed Rules) — FR-009, FR-010; SC-002, SC-008
- [x] Principle II (Explicit Instants, Explicit Uncertainty) — FR-002, FR-011, FR-012, FR-013, FR-014; SC-003, SC-007
- [x] Principle III (Reproducible Rule Data) — FR-016, FR-017, FR-018; SC-005
- [x] Principle IV (Library-First Core, Thin Shells) — not constrained at spec level; deferred to `/speckit-plan`
- [x] Principle V (Test-First with Golden Vectors) — edge cases enumerate the vector set: DST both directions, holidays, shortened sessions, break, boundary instants, out-of-coverage
- [x] Shared phase vocabulary constraint (Domain and Data Constraints) — FR-005, FR-006; SC-006

## Validation Notes

**Iteration 1 (2026-07-29)**

Two failures found and fixed before this checklist was finalised:

1. *Success criteria contained an implementation-flavoured metric.* An earlier draft measured answer latency. Replaced with user-facing outcomes (SC-004, SC-008) per the technology-agnostic rule.
2. *Phase and event were conflated.* An earlier draft modelled crypto funding as a phase, which breaks the "phases tile all time without overlap" property and would have forced Binance to introduce venue-specific phase names — violating the constitution's shared-vocabulary constraint. Split into Phase and Event as distinct entities (FR-007, FR-008).

**Iteration 2 (2026-07-29)**

Both open clarifications resolved; recorded in the spec's Resolved Scope Decisions section
rather than deleted, so the reasoning survives into planning.

- Binance product scope → **perpetual futures (USD-M)**. Funding was in the founding scope and
  is the only launch-set case that exercises FR-007's phase-versus-event split for real.
- Visual board → **in this slice**. Deliberate order: match a conventional trading-hours board
  first, then carry evidence and uncertainty all the way to the surface.

Two constraints fell out of the board decision and are now recorded in the spec: the board
passes `now` into the core rather than the core reading a clock (Principle IV), and a board
displaying "now" must surface host clock discipline bounds as uncertainty (constitution,
Domain and Data Constraints). The spec also states the accepted hard part outright — if a
display cannot express an honest answer, the display changes, not the answer.

Two assumptions were added in the same pass: no single downstream consumer shapes the schema
or venue list, and aggregators are design references rather than data sources. The second is
grounded — a well-known financial aggregator's published terms were checked on 2026-07-29 and
explicitly forbid reuse of its data for AI training or commercial purposes.

**Status: all checklist items pass. Ready for `/speckit-plan`.**
