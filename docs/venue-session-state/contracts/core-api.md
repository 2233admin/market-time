# Contract: `market-time-core` public API

Binding principle (Constitution IV): no I/O, no network, no clock read, anywhere in the decision
path. Instants and rule data are always arguments, never sourced by the core itself. See
`README.md` for the rules shared across all three contracts.

Signatures are Rust-flavored pseudocode to fix shape and invariants. They are not an
implementation.

## 1. `UtcInstant` — the only instant type the core accepts

```rust
pub struct UtcInstant { nanos_since_unix_epoch: i128 }

impl UtcInstant {
    pub fn from_nanos_since_unix_epoch(nanos: i128) -> Self;
    // No `now()`. No constructor reads a clock, a scale table, or a file.
}
```

**How "core never reads a clock" is visible, not just asserted:**

- `UtcInstant` has exactly one family of constructors, and none of them is `now()` or anything
  that resolves to one. The type cannot be produced without a caller supplying the nanosecond
  value.
- Every core query function below takes `at: UtcInstant` as its first, non-defaulted parameter.
  There is no code path through core that produces a phase answer without one being handed in.
- `market-time-core`'s dependency graph contains nothing capable of a clock read (no
  `std::time::SystemTime`, no `jiff::Zoned::now`, no `hifitime::Epoch::now`). This is a workspace
  fact per `design.md`'s Structure Decision, and it is what makes a violation of this contract fail
  to compile rather than fail review.

**Judgment call**: the core only ever sees `UtcInstant`, never a scale-tagged instant. FR-013's
"explicitly declared time scale" is satisfied by the type itself — `UtcInstant` cannot be
anything but UTC — rather than by a runtime tag core would have to check. Non-UTC input (TAI,
GNSS, monotonic) is tagged and converted through `hifitime`'s leap-second-aware path in
`market-time-scales` (research D1a) *before* it reaches core; core has no scale-conversion logic
at all, so it cannot get a scale conversion wrong. This is a stronger reading of Principle II's
"never by a hardcoded constant" than a runtime check would give: the conversion code has exactly
one location in the tree, and it isn't here.

## 2. Primary query — phase at an instant

```rust
pub fn resolve_phase(at: UtcInstant, venue: VenueId, ruleset: &Ruleset) -> PhaseOutcome;

pub enum PhaseOutcome {
    Known(PhaseAnswer),
    Unknown(CoverageGap),
}

pub struct PhaseAnswer {
    pub venue: VenueId,
    pub phase: Phase,                        // FR-005/006, closed shared vocabulary
    pub boundary_start: PhaseBoundary,        // FR-003
    pub boundary_end: PhaseBoundary,          // FR-003
    pub evidence: Vec<EvidenceRef>,           // FR-009
    pub uncertainty: Uncertainty,             // FR-011, answer-level
    pub dataset_revisions: Vec<DatasetRevisionId>, // FR-016
}

pub struct PhaseBoundary {
    pub instant: UtcInstant,
    pub uncertainty: Uncertainty,             // per-boundary; carries FR-011b's process-spread case
}

#[non_exhaustive]
pub enum Phase {
    Closed, PreOpen, OpeningAuction, ContinuousTrading,
    MidDayBreak, ClosingAuction, PostClose, NonTradingInterruption,
}
```

`Phase` is `#[non_exhaustive]` and lives only in this crate: FR-005 ("no venue may introduce a
phase name of its own") is enforced by shells being structurally unable to construct or extend
the enum, not by convention.

`Uncertainty` and `EvidenceRef` are compact by design — full shape belongs in `data-model.md`,
not repeated here — but the contract fixes that `Uncertainty` MUST be able to express, at
minimum: exact-to-representation, a published granularity (second-level publication), a
published bound handed to us by a venue (FR-011a, Binance's ±15s), a process-start spread with
no invented number (FR-011b, NYSE's opening process), and a derived/reasoned rule (FR-010). A
`PhaseAnswer` or `PhaseBoundary` whose `uncertainty` claims exactness where the evidence doesn't
support it is a contract violation, not a rounding choice.

`PhaseBoundary` carries `uncertainty` but no separate `evidence` list of its own: a phase's start
and end trace to the same `Rule` record as the phase determination itself for every venue in the
launch set, so `PhaseAnswer.evidence` already covers both boundaries — satisfying FR-003's "same
evidence... rules as the phase itself" without duplicating the field. A venue whose open and
close were ever announced from genuinely separate documents would need this revisited; none of
SSE, NYSE, or Binance is such a case.

## 3. Events overlaid on a phase

```rust
#[non_exhaustive]
pub enum EventKind { FundingSettlement }

pub struct EventOccurrence {
    pub kind: EventKind,
    pub instant: UtcInstant,
    pub uncertainty: Uncertainty,             // FR-011a: Binance's published ±15s bound lives here
    pub evidence: Vec<EvidenceRef>,
}
```

`PhaseAnswer` carries `pub events: Vec<EventOccurrence>` — the events scheduled within
`[boundary_start, boundary_end)` for that phase (research D5, D5a: Binance funding settlement is
the launch case). Events are not phases and do not tile: `events` may be empty, sparse, or
contain occurrences that overlap one another; an event never replaces, splits, or extends the
phase it sits inside (FR-007, FR-008). `EventKind` is its own closed, `#[non_exhaustive]`
vocabulary, owned by core exactly like `Phase` — the two enums are deliberately separate types so
an event kind is never mistaken for, or substituted as, a phase (README rule 5 applies to both).

## 4. Unknown — a data outcome, not "closed" and not an error

`PhaseOutcome::Unknown(CoverageGap)` is returned when `at` falls outside `venue`'s declared
coverage (FR-002). It is a variant of the ordinary return value, not a `Result::Err` and not
`Phase::Closed`.

```rust
pub struct CoverageGap {
    pub venue: VenueId,
    pub queried_at: UtcInstant,
    pub coverage: CoverageRange,              // the declared boundary that was violated (FR-018)
    pub dataset_revisions: Vec<DatasetRevisionId>,
}
```

**Judgment call — what IS an error, then**: `resolve_phase` returns `PhaseOutcome` unconditionally
for any `VenueId` known to `ruleset`. A venue name the ruleset has no entry for at all is treated
as zero coverage — it resolves to `Unknown`, not to an `Err` — so the query path never fails for
a data reason. Genuine errors (a malformed `Ruleset`, e.g. rules that don't tile without gaps
per FR-008) are caught once, at `Ruleset` construction (§6), not on every query. This keeps the
decision function itself total and keeps "unknown vs. error" a single, simple line: unknown is
about the query, error is about the ruleset being invalid before any query was made.

## 5. Multi-venue query

```rust
pub fn resolve_phases(at: UtcInstant, venues: &[VenueId], ruleset: &Ruleset) -> Vec<VenueOutcome>;

pub struct VenueOutcome { pub venue: VenueId, pub outcome: PhaseOutcome }
```

**Invariants**:

- The returned `Vec` has exactly one `VenueOutcome` per entry in `venues`, in order. There is no
  path by which one venue's outcome suppresses another's (FR-020) — the return type has no error
  variant for the batch as a whole, only per-venue `PhaseOutcome`, so "one bad venue kills the
  batch" cannot type-check.
- `at` is a single value applied to every venue in the call. `resolve_phases` does not read time
  once per venue — there is nowhere in its signature it could — so every venue in one response
  describes the same instant. This is what makes a board render (see `board.md`) a coherent
  snapshot rather than a composite of moments read microseconds apart.

## 6. Supplying a ruleset the core cannot load itself

Core has no I/O, so nothing that reads a file, a URL, or a socket exists in this crate. A
`Ruleset` is constructed from data a shell has *already fully materialized in memory* —
`Ruleset::from_parts` accepts owned Rust values, never a path or a handle:

```rust
pub struct Ruleset { /* opaque */ }

impl Ruleset {
    pub fn from_parts(
        revisions: Vec<DatasetRevision>,
        venues: Vec<VenueRuleset>,
    ) -> Result<Ruleset, RulesetError>;   // the one place core-side validation happens
}

pub struct VenueRuleset {
    pub venue: VenueId,
    pub home_zone: IanaZoneId,                 // e.g. "America/New_York"
    pub coverage: CoverageRange,                // FR-018
    pub rules: Vec<Rule>,
    pub evidence: Vec<EvidenceRef>,
}

pub struct Rule {
    pub kind: RuleKind,                        // WeeklyPattern | Holiday | ShortenedSession | AnnouncedChange
    pub civil_schedule: CivilPhaseSchedule,     // phase boundaries as civil time-of-day
    pub applies: DateRange,                     // dates this rule is in force, venue-local calendar
    pub evidence: EvidenceRef,
    pub derived: Option<DerivationNote>,        // FR-010
}
```

**Judgment call — rules are stored as civil time-of-day tied to `home_zone`, never as
pre-computed UTC.** A boundary baked to UTC once cannot re-derive the correct offset the next
time DST shifts — that would silently reintroduce the hardcoded-offset failure Principle II
names, just moved into data instead of code. `resolve_phase` performs the civil-to-UTC
conversion at query time, through `jiff`'s DST-ambiguity-safe path (`Disambiguation::Reject`,
research D1), which is also why FR-014's nonexistent/ambiguous local times are a resolvable
question at all: the rule doesn't know the answer in advance, the conversion at query time does.

`from_parts` validates structural invariants against the materialized data it's given — phases
tile without gaps or overlaps (FR-008), every `Rule.applies` range sits inside its venue's
`coverage`, every rule cites a revision present in `revisions` — and returns `RulesetError` on
violation. This is computation over data already in memory, not I/O, and it is the only place in
core that rejects a ruleset rather than a query.

`VenueId`, `DatasetRevisionId`, and `IanaZoneId` are opaque string-backed identifiers, not
hardcoded enums — adding a venue or a dataset revision is a data change, never a core code
change.

## Violations

A change to this crate is non-conforming if it: adds any constructor or code path that reads
wall-clock time, a file, or a network resource; adds a `Phase` variant usage that isn't
`#[non_exhaustive]`-safe for shells; returns `Unknown` for an in-coverage instant or `Known` for
an out-of-coverage one; lets `resolve_phases` fail as a batch because one venue lacks coverage;
or accepts a `Ruleset` constructor that takes a path, URL, or handle instead of materialized
values.
