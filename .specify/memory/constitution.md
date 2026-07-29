<!--
SYNC IMPACT REPORT
Version change: (uninstantiated template) → 1.0.0
Bump rationale: Initial ratification. Every placeholder token replaced with concrete
  content; no prior ratified version existed to amend, so this is a MAJOR baseline.

Principles defined (all new):
  I.   Evidence-Backed Rules (NON-NEGOTIABLE)
  II.  Explicit Instants, Explicit Uncertainty
  III. Versioned Rule Data, Never Hardcoded
  IV.  Library-First Core, Thin Shells
  V.   Test-First with Golden Vectors (NON-NEGOTIABLE)

Sections added:
  - Domain and Data Constraints (replaces [SECTION_2_NAME])
  - Development Workflow and Quality Gates (replaces [SECTION_3_NAME])
  - Governance (instantiated)

Removed sections: none

Template consistency:
  ✅ .specify/templates/plan-template.md — "Constitution Check" is a derive-at-plan-time
     placeholder with no hardcoded principle names; no edit required.
  ✅ .specify/templates/spec-template.md — stock scope/requirements structure, compatible
     with Principles I–V; no edit required.
  ✅ README.md — created alongside this ratification; states Principles I–III, because
     they constrain how downstream consumers may trust Mark Time output.

Deferred TODOs: none
-->

# Mark Time Constitution

Mark Time answers one question with auditable precision: **what time is it, and what is
open, where.** It serves global city clocks, exchange trading sessions and phases, and
crypto funding and maintenance windows. Every answer it gives is a claim about the real
world, and every claim must be traceable to evidence.

## Core Principles

### I. Evidence-Backed Rules (NON-NEGOTIABLE)

Every rule that shapes an answer — a holiday, a session boundary, a half-day, a DST
transition, a funding interval, a maintenance window — MUST be stored with the evidence
that justifies it: `source_url`, `fetched_at`, `effective_from`, and where the upstream
publishes one, `source_updated_at`.

Derived or inferred fields MUST NOT be presented as observed. A rule whose
`effective_from` was reasoned about rather than published MUST be flagged as derived and
carry the reasoning. Fabricating an availability timestamp from an event timestamp is a
critical defect, not a shortcut.

**Rationale**: A time-and-session service that cannot show its work is a rumour. Users
will reconcile Mark Time against an exchange notice and must be able to see exactly which
document Mark Time read and when.

### II. Explicit Instants, Explicit Uncertainty

Instants are nanosecond-precision and unambiguous about their frame: an instant is either
an absolute epoch instant or a civil (wall-clock) time bound to a named zone, never an
implicit mixture.

Every answer MUST be able to carry uncertainty. Where a rule version, an upstream
publication lag, or a pending-but-unconfirmed change makes the answer less than exact, the
answer MUST say so rather than emitting a bare point estimate.

Outside known coverage the system MUST return an explicit unknown. Extrapolating a
calendar past its verified range, or silently falling back to "probably the usual
schedule", is forbidden.

**Rationale**: Downstream systems make money decisions on "is it open". A confident wrong
answer is more expensive than an honest unknown.

### III. Versioned Rule Data, Never Hardcoded

Time-zone data, exchange calendars, session and phase definitions, and crypto venue
schedules are versioned data artifacts with pinned identifiers — never literals embedded
in source code.

Every build MUST pin and report the versions it was compiled or loaded against (at
minimum the IANA tzdata release and each calendar dataset revision). Rule updates ship as
data revisions with their own evidence per Principle I, and MUST be reproducible: given a
dataset revision, the same query returns the same answer.

**Rationale**: Zone rules and exchange calendars change on political and administrative
timelines, not release timelines. Data that can be re-pinned and replayed is auditable;
recompiled constants are not.

### IV. Library-First Core, Thin Shells

The core is a self-contained, independently testable Rust library with no I/O, no network,
and no clock reads in its decision path — instants and rule data are passed in.

CLI, service, and UI surfaces are thin shells over that library and MUST NOT hold domain
logic. Ingestion of upstream sources lives behind an explicit boundary and MUST NOT be
importable from the core decision path.

**Rationale**: A pure core is what makes Principle V's golden vectors possible: any
instant, any zone, any dataset revision can be replayed deterministically. It also keeps
the presentation layer — including the dense global-session board the product is modelled
on — free to change without touching correctness.

### V. Test-First with Golden Vectors (NON-NEGOTIABLE)

Tests are written before implementation, and the boundary cases carry named golden
vectors covering at minimum: DST spring-forward and fall-back transitions (including the
nonexistent and ambiguous local hours), exchange holidays and half-days, session and phase
edges including pre-open auctions and mid-day breaks, crypto funding boundaries, and
queries outside coverage.

Every defect found in a time or session answer MUST become a golden vector before it is
fixed. Vectors are permanent and never deleted to make a build pass.

**Rationale**: Time bugs are rare, seasonal, and expensive — a fall-back bug surfaces once
a year in one zone. Only an accumulating vector set catches them before users do.

## Domain and Data Constraints

- **Stack**: Rust. The core is a library crate; shells are separate crates in one
  workspace.
- **Upstream classes**: structured feeds, published calendars, and unstructured venue
  announcements (crypto maintenance is routinely a blog post or a social post). All three
  MUST satisfy Principle I. An unstructured source is admissible only through a recorded
  ingestion step that captures the document and its retrieval time; a human-entered rule is
  acceptable evidence when it records who entered it, from which document, and when.
- **Licensing**: before any upstream is ingested programmatically, its terms MUST be
  checked and the finding recorded with the source registration. Redistribution rights are
  not assumed.
- **Coverage is declared**: every dataset states the range it is valid for. Principle II's
  explicit-unknown rule is enforced against that declaration, not against convenience.
- **Phase vocabulary is shared**: venues with different market structures (auction-plus-break
  cash equities, DST-sensitive continuous markets, 24/7 crypto with funding and maintenance
  windows) MUST map onto one documented phase model. Adding a venue that does not fit is a
  signal to amend the model deliberately, not to special-case it in a shell.

## Development Workflow and Quality Gates

- Work flows through spec-kit: `/speckit-specify` → `/speckit-plan` → `/speckit-tasks` →
  `/speckit-implement`. Constitution Check in the plan template gates on the principles
  above and MUST pass before Phase 0 research and again after Phase 1 design.
- Any Constitution Check violation carried forward MUST be recorded in the plan's
  Complexity Tracking table with the simpler alternative that was rejected and why.
- `cargo test` covers the change, and golden vectors are added or extended per Principle V.
- Rule-data changes are reviewed as data: dataset revision, evidence fields, and coverage
  declaration are all inspected, and a change without evidence is rejected regardless of
  how obviously correct it looks.
- Public API and rule-data schema changes are versioned per the Governance section and
  noted in the changelog.

## Governance

This constitution supersedes other practices in this repository. Where a runbook, template,
or habit conflicts with it, this document wins and the other artifact is corrected.

**Amendment procedure**: amendments are proposed as a change to this file, state the
principle affected and the rationale, and are ratified by the repository owner. An
amendment that invalidates existing behaviour MUST ship with a migration note describing
what changes for consumers.

**Versioning policy**: this constitution follows semantic versioning. MAJOR for removing or
redefining a principle in a backward-incompatible way, MINOR for adding a principle or
materially expanding guidance, PATCH for clarifications and wording.

**Compliance review**: every plan runs Constitution Check at both gates. Reviews verify
that new rules carry evidence (I), that new answers can express uncertainty and unknown
(II), that no rule was hardcoded (III), that domain logic did not leak into a shell (IV),
and that boundary behaviour gained a golden vector (V).

**Version**: 1.0.0 | **Ratified**: 2026-07-29 | **Last Amended**: 2026-07-29
