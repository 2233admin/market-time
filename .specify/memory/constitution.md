<!--
SYNC IMPACT REPORT
Version change: 1.1.0 → 1.2.0
Bump rationale: MINOR. Principle III is narrowed and retitled. Its hygiene clause ("never
  literals embedded in source code") is dropped as ordinary engineering practice that does
  not need constitutional standing; its reproducibility guarantee is retained and
  strengthened by making revision immutability explicit.

  Bump-type reasoning, recorded because this case is arguable: a MAJOR bump is reserved for
  backward-incompatible removals or redefinitions. Relaxing a constraint cannot make
  already-conforming work non-conforming, the principle itself is not removed, and the
  load-bearing guarantee is stronger than before — so MINOR. The one genuinely
  backward-incompatible effect is cosmetic: anything citing Principle III by its old title
  now cites a title that no longer exists. No such citation exists outside this repository.

Modified principles:
  III. "Versioned Rule Data, Never Hardcoded" → "Reproducible Rule Data"
      - Dropped: the "never literals embedded in source code" requirement.
      + Added: dataset revisions are IMMUTABLE — a correction produces a new revision,
        never an edit in place. This was implied before and is now stated.
      + Added: every answer MUST be attributable to the revisions that produced it.
      = Retained: builds report the revisions they run against; same revision set plus same
        query returns the same answer.
      Rationale rewritten to state the division of labour against Principle I explicitly:
      I says where a rule came from, III says which rule set was in force.

Unmodified principles: I, II, IV, V

Sections modified: none in this revision.

Added sections: none
Removed sections: none

Prior revisions:
  1.1.0 (2026-07-29) — Principle II materially expanded: time scale MUST be declared on
    absolute instants (UTC published, TAI/GNSS/monotonic input tagged and converted
    leap-second-aware, never by a hardcoded constant); leap-second table governed as rule
    data and leap smearing recorded against the source; precision is not accuracy, so
    published and observed boundaries are distinct claims. Domain and Data Constraints
    gained "Clock discipline is out of scope; its quality is not".
  1.0.0 (2026-07-29) — initial ratification of Principles I–V, Domain and Data Constraints,
    Development Workflow and Quality Gates, and Governance.

Template consistency:
  ✅ .specify/templates/plan-template.md — "Constitution Check" is a derive-at-plan-time
     placeholder with no hardcoded principle names; no edit required.
  ✅ .specify/templates/spec-template.md — stock scope/requirements structure, compatible
     with Principles I–V; no edit required.
  ✅ README.md — Principle III summary retitled and rewritten in the same change, so the
     public summary does not cite a principle title that no longer exists. (Principle II's
     summary was brought in line at 1.1.0.)

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

**Frame.** Instants are nanosecond-precision and unambiguous about their frame: an instant
is either an absolute instant on a declared time scale, or a civil (wall-clock) time bound
to a named zone, never an implicit mixture.

**Time scale.** An absolute instant MUST declare its time scale. UTC is the default and the
only scale in which answers are published. An input on any other scale — TAI, GNSS system
time, or a host monotonic scale — MUST be tagged as such and converted through a
leap-second-aware path. Converting by adding a hardcoded constant is forbidden, because
that constant is itself a dated fact that changes. GNSS system time currently leads UTC by
a whole number of seconds; in a system that claims nanoseconds, treating the two as
interchangeable is an error of roughly ten orders of magnitude.

**Leap seconds and smearing.** UTC is not a uniform scale. The leap-second table is
versioned rule data governed by Principle III, and historical leap seconds remain facts
regardless of future policy changes. A time source that smears a leap second is not serving
UTC across the smear window; where a host or an upstream is known to smear, that MUST be
recorded with the source rather than silently accepted as UTC.

**Precision is not accuracy.** Nanosecond representation is a statement about arithmetic
and lossless round-tripping. It is never a claim of nanosecond accuracy. An answer MUST NOT
be presented as more precise than its evidence supports. Where a venue publishes a boundary
to the second, states it in local wall time, or deliberately randomises the exact instant,
the answer's uncertainty MUST reflect that. The published boundary and the observed
boundary are two different claims and MUST NOT be conflated.

**Uncertainty.** Every answer MUST be able to carry uncertainty. Where a rule version, an
upstream publication lag, or a pending-but-unconfirmed change makes the answer less than
exact, the answer MUST say so rather than emitting a bare point estimate.

**Unknown.** Outside known coverage the system MUST return an explicit unknown.
Extrapolating a calendar past its verified range, or silently falling back to "probably the
usual schedule", is forbidden.

**Rationale**: Downstream systems make money decisions on "is it open". A confident wrong
answer is more expensive than an honest unknown. The scale and smearing rules exist because
at nanosecond resolution a mislabelled time scale is not a rounding error — it is a
whole-second-class defect wearing a precise-looking number, and it will not announce
itself.

### III. Reproducible Rule Data

Rule data — time-zone data, exchange calendars, session and phase definitions, crypto venue
schedules — ships as versioned dataset revisions with stable identifiers. **A revision is
immutable**: correcting a rule produces a new revision, never an edit in place.

Every build MUST report the revisions it is running against, at minimum the IANA tzdata
release and each calendar dataset revision. Every answer MUST be attributable to the
revisions that produced it. Given the same revision set and the same query, the system MUST
return the same answer it returned before.

**Rationale**: Principle I says where a rule came from. This principle says which rule set
was in force. An audit needs both — reconstructing why an answer was given last quarter is
impossible if the dataset that produced it was overwritten since. Zone rules and exchange
calendars change on political and administrative timelines, so overwrite-in-place is the
default failure mode here, not an edge case.

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
- **Clock discipline is out of scope; its quality is not**: Mark Time does not synchronise
  clocks, and per Principle IV the core reads no clock at all. Where a shell surface reports
  "now", it MUST source the host's own discipline bounds — for example the offset and
  dispersion an NTP or PTP daemon already reports — and expose them as uncertainty rather
  than presenting the host clock as exact. Wide-area internet time synchronisation does not
  reach nanoseconds; no surface may imply that it does.

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

**Version**: 1.2.0 | **Ratified**: 2026-07-29 | **Last Amended**: 2026-07-29
