# Data Model: Venue Session State

**Feature**: `specs/001-venue-session-state` | **Date**: 2026-07-29
**Inputs**: [spec.md](./spec.md) (FR-001..FR-020, Key Entities), [research.md](./research.md)
(D1..D7), [plan.md](./plan.md) (crate layout), Constitution v1.2.0 (Principles I-V)

This is a design document. Types are described by field name, type intent, required/optional,
and the constraint that forces the field to exist — not as Rust source. Where the crate layout
matters it is noted (`plan.md`'s `crates/market-time-core/src/*.rs`), but the model itself is
language-neutral.

## Crate mapping (plan.md's `crates/market-time-core/src/*.rs`)

Section-to-file mapping, so a reader coming from plan.md's Project Structure can find where each
entity is meant to land. Not every entity has an unambiguous, individually-enumerated file in
plan.md; those are marked accordingly rather than guessed.

| Section below | Entity/type | plan.md file |
|---|---|---|
| §1 | `Instant`, `CivilInstant` (core); `RawScaledInstant` (ingest boundary) | `instant.rs` (core crate); `RawScaledInstant` and the hifitime conversion live in the sibling `market-time-scales` crate, per plan.md's Structure Decision — "Conversion from a tagged non-UTC scale into UTC happens there and nowhere else" |
| §2 | `Venue` | not individually enumerated in plan.md's file list; its rule set is what `ruleset.rs` ("Dataset revisions; resolution") operates over |
| §3 | `Rule` and its subtypes | not individually enumerated; most plausibly part of `ruleset.rs` alongside `DatasetRevision`, since a dataset revision's payload *is* a set of rules — plan.md does not say so explicitly, so this is a placement inference, not a cited fact |
| §4 | `Evidence` | `evidence.rs` |
| §5 | `Coverage` | `coverage.rs` |
| §6 | `DatasetRevision` | `ruleset.rs` |
| §7 | `Phase`, `PhaseSegment`, `PhaseTimeline` | `phase.rs`; the timeline-resolution operation itself is `resolve.rs` ("instant + venue -> phase") |
| §8 | `Uncertainty`, `Boundary` | `uncertainty.rs` |
| §9 | `Event` | `event.rs` |
| §10 | `Query`, `Answer` | `resolve.rs` |

## Conventions (stated once, relied on everywhere below)

- **Interval convention — half-open, uniform.** Every interval in this model (phase segments,
  coverage ranges, rule effective ranges) is **start-inclusive, end-exclusive**: `[start, end)`.
  This is a resolved interpretive choice, not a transcription of the spec. Coverage's Key
  Entity text and the "coverage edge" edge case do not themselves state open vs. closed; a
  closed-interval reading (`[start, end]`) is equally consistent with that prose. Half-open was
  chosen because (a) it is the same convention phase-segment tiling needs anyway to satisfy
  FR-008 without double-owning a boundary instant (see Phase, below), and (b) using one
  convention everywhere means a coverage range and a rule's effective range compose without a
  seam — a rule's `effective_until` can equal the next rule's `effective_from` exactly, and a
  dataset's `valid_until` can equal a query instant that is correctly outside coverage, with no
  off-by-one case to special-case in code. **Flagged as a design decision, not an obvious fact.**
- **Identifiers.** All entity ids are opaque stable strings scoped to their kind (`venue_id`,
  `rule_id`, `evidence_id`, `revision_id`, `event_id`). None are reused across kinds and none are
  derived from mutable content — a `revision_id` in particular MUST be stable for the life of the
  revision (Principle III).
- **Instant, throughout this document, means the `Instant` type defined below** (UTC nanoseconds,
  declared scale) unless explicitly written as `CivilInstant`.

---

## 1. Instant and Frame (FR-013, Principle II, D1, D1a)

Principle II requires that "an instant is either an absolute instant on a declared time scale, or
a civil (wall-clock) time bound to a named zone, never an implicit mixture." The model makes this
two *distinct, non-interchangeable* types rather than one type with a mode flag, so that mixing
them is a type error, not a runtime bug waiting to happen.

### 1.1 `RawScaledInstant` (ingest boundary only — lives in `market-time-scales`, D1a)

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `nanos` | 128-bit signed integer, ns since an epoch | required | precision floor set by FR-013 |
| `scale` | enum: `Utc \| Tai \| Gnss \| Monotonic(host_id)` | required | Principle II: "tagged as such" — every non-UTC input MUST declare its scale, never assumed |

This type exists so that a TAI-, GNSS-, or monotonic-tagged input has *nowhere to go* except
through the leap-second-aware conversion below. It is not visible to `market-time-core`.

**Conversion**: a single operation takes a `RawScaledInstant` and produces an `Instant`, implemented
via `hifitime` (D1: the only surveyed crate with a real IERS leap-second table). Converting by
adding a hardcoded offset is
constitutionally forbidden (Principle II) and structurally impossible here — there is no field
on `RawScaledInstant` a caller could add a constant to and get an `Instant`; the conversion is a
function call through the one crate that carries the leap-second table as versioned data (D2:
`tzdb-bundle-always`, IANA release id recorded in dataset-revision metadata, not read from the
library at runtime).

### 1.2 `Instant` (core type — lives in `market-time-core/src/instant.rs`)

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `nanos_utc` | 128-bit signed integer, ns since epoch | required | FR-013; "UTC is the default and the only scale in which answers are published" (Principle II) |

By construction every `Instant` the core holds is UTC. This is what makes "declared time scale"
concrete rather than a documentation promise: there is exactly one scale representable in the
core's decision path, so Principle IV's "no clock reads, no ambiguous arithmetic" extends
naturally to "no ambiguous scale" — a value that isn't UTC yet cannot be an `Instant` at all, it
is still a `RawScaledInstant` sitting in the ingest crate.

**D1a seam, stated explicitly, not papered over**: once a `RawScaledInstant` crosses into
`Instant`, it is leap-second-naive again (jiff's `Timestamp` layer, chosen for civil/DST
correctness, has no leap-second concept). An instant *during* a historical leap second cannot
round-trip through the civil layer. Accepted as a permanent, named golden vector per D1a — no
venue phase boundary in the launch set has ever fallen inside a leap second — rather than an
untested assumption.

### 1.3 `CivilInstant` (wall-clock claim, distinct type)

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `date` | calendar date | required | — |
| `time_of_day` | h/m/s/ns within the day | required | FR-013 ns resolution |
| `zone` | named IANA zone identifier | required | Principle II: civil time is "bound to a named zone" |

`CivilInstant` and `Instant` are never silently interchangeable. The only path between them is a
single resolution operation that takes a `CivilInstant` and a `DisambiguationPolicy` and produces
one of the following outcomes (a `CivilResolution`):

| Outcome | Carries | Meaning | Grounding |
|---|---|---|---|
| Unambiguous | one `Instant` | exactly one UTC instant corresponds | normal case |
| Ambiguous | a list of `Instant` values (more than one candidate) | the local time occurred more than once (fall-back) | FR-014, spec Edge Case "Fall-back"; "must resolve which occurrence is meant, or say it cannot" |
| Nonexistent | the `Instant` bounds of the gap the local time falls in | the local time never occurred (spring-forward) | FR-014, spec Edge Case "Spring-forward"; "must not silently invent it" |

The product's default `DisambiguationPolicy` is **Reject** (D1: jiff's `Disambiguation::Reject`
turns ambiguity/nonexistence into a typed result rather than a silent guess — "Principle II's
'never silently guessed' expressed directly in the type system"). A query that supplies a
`CivilInstant` landing in `Ambiguous` or `Nonexistent` without resolving it is a malformed query
(see §10, `QueryError`), not an `Unknown` and not a phase answer.

**Where each type is used**: venue schedules (Rule, §3) are authored in civil terms, because that
is how venues publish them (D4's SSE table is stated in Beijing local time). Phase boundaries
(§7) and event instants (§9) are stored and published as `Instant` (UTC), because that is the one
scale answers are published in (Principle II). The resolution algorithm is the one place civil
rule-time becomes an absolute boundary, using the policy above.

---

## 2. Venue

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `venue_id` | opaque id | required | stable, unique |
| `display_name` | string | required | — |
| `home_time_zone` | named IANA zone id | required | needed to interpret every Rule's civil times (§1.3); Binance is UTC-native (no DST) per its always-on model, SSE/NYSE are zone-bound |
| `coverage` | 1:N `Coverage` (§5) | required, non-empty | FR-018: "each dataset must declare the range it is valid for"; a venue may have more than one coverage-bearing dataset (e.g., a calendar dataset and, for Binance, a funding-rule dataset) with independently declared ranges |
| `rules` | 1:N `Rule` (§3), scoped to this venue | required | the venue's schedule *is* its rule set — there is no separate "schedule" field, to avoid two places that could disagree |

Launch set: SSE, NYSE, Binance USD-M perpetual futures (spec Assumptions; Resolved Scope
Decisions — spot excluded because funding, the phase/event stress case, does not exist there).

**Relationship note**: a `Venue` does not store phases directly. Phases (§7) are *derived* from
a venue's `Rule`s at query time (or cached, but always reconstructible from the same rules) —
this is what keeps the tiling invariant enforceable structurally rather than by convention
(see §7.3).

---

## 3. Rule

A `Rule` is a statement about a venue's schedule together with the range of dates it applies to
(spec Key Entities). Every `Rule` carries exactly one `Evidence` (§4) — Principle I, FR-009 — and
an explicit derived/published marker (FR-010).

### 3.1 Common fields

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `rule_id` | opaque id | required | stable |
| `venue_id` | FK → Venue | required | — |
| `kind` | enum, see 3.2 | required | discriminates the rule subtypes below |
| `effective_from` | date/instant | required | **equal to** `evidence.effective_from` (§4) — not duplicated. A rule's applicability start *is* the date its evidence says it takes effect; storing it twice invites the two to drift. |
| `effective_until` | date/instant, optional | optional (open-ended if absent) | closes when superseded by a later rule (whose `effective_from` then equals this `effective_until`, per the half-open convention) or when a source publishes an explicit end (e.g., a time-boxed reversion-threshold rule, D5) |
| `evidence_id` | FK → Evidence | required, exactly one | Principle I non-negotiable |
| `revision_id` | FK → DatasetRevision (§6) | required | which immutable revision this rule row belongs to (FR-016 attribution) |

**Precedence is not a stored field.** FR-015 requires holidays and shortened sessions to override
the normal weekly schedule. This is expressed as a fixed evaluation order over `kind`
(`HolidaySession` and `ShortenedSession` outrank `WeeklyPattern` for a given date) built into the
resolution algorithm (`resolve.rs`), not as a per-row priority number. A stored priority field
would be a second place the override order could be edited out of sync with FR-015; a fixed order
keyed on `kind` cannot drift independently.

### 3.2 `kind` subtypes

| Kind | Produces | Fields (beyond common) | Grounding |
|---|---|---|---|
| `WeeklyPattern` | Phase segments for a day-of-week (or day-of-week set) | `applicable_weekdays`; ordered list of `(phase_kind, civil_start_time, civil_end_time)` | D4's SSE table is exactly one `WeeklyPattern` row's payload |
| `HolidaySession` | Full-day (or partial) override for one specific date | `date`; either "closed all day" or an alternate segment list | Acceptance Scenario 3; FR-015 |
| `ShortenedSession` | Alternate segment list for one specific date | `date`; alternate `(phase_kind, civil_start_time, civil_end_time)` list | Acceptance Scenario 4; D6 (NYSE 13:00 close, day after Thanksgiving, Christmas Eve — exact footnote wording still needs a byte-exact pass per research.md "Carried into Phase 1" item 3) |
| `AnnouncedChange` | A new `WeeklyPattern`-shaped schedule effective at a future date | same shape as `WeeklyPattern`, plus its own `effective_from` in the future | spec Edge Case "Future-dated schedule change"; structurally this is just a `WeeklyPattern` row whose `effective_from` has not yet arrived — no separate resolution logic needed, only the ordinary effective-range lookup |
| `EventRecurrenceRule` | **Events**, not phases (§9) | see 3.3 | D5 — kept as its own kind because it generates `Event`s, never a `Phase` (FR-007) |

`HolidaySession` and `ShortenedSession` are date-scoped (`effective_from`/`effective_until` span
exactly that date), which is why they can outrank a `WeeklyPattern` row that would otherwise also
apply — for a given instant, at most one date-scoped override rule is in force, and it wins.

### 3.3 `EventRecurrenceRule` fields (D5 — variable, versioned recurrence)

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `interval_kind` | enum: `EightHourly \| FourHourly \| Hourly` | required | D5: default 8h; named subset on 4h since 2023-10-12; volatility-triggered 1h since 2025-05-02 |
| `scope` | see below | required | which contracts this row governs |
| `reversion_threshold` | a pair — a rate-magnitude cap/floor value, and a required count of consecutive funding cycles at or inside it — optional | required when `interval_kind` can revert (i.e., describes the condition for leaving `Hourly`) | D5: 2025-09-01 set ≤\|0.002%\| for 36 cycles; 2026-01-02 changed it to ≤\|0.025%\| for 16 cycles — **two different, non-overlapping-in-time `EventRecurrenceRule` rows**, not one row with a mutated threshold (Principle III: correction/change = new revision, not edit in place) |

`scope` is one of:

- `Global` — applies to every USD-M perpetual contract by default (the 8-hourly base case).
- `NamedSubset(list of contract symbols)` — the 4-hourly cohort named since 2023-10-12 (D5).
- `PublishedPerSymbolOverride` — see the flagged tradeoff immediately below.

**Flagged design tension, not resolved by invention**: the hourly shift is described by D5 as
happening "when its funding rate hits the cap or floor" — i.e., it is state-dependent on a
contract's *recent funding-rate history*, which is market data. The spec's own Assumptions
explicitly exclude Mark Time from being a market-data feed ("It does not carry prices, quotes, or
trades, and it does not observe live venue activity"). Computing "is contract X in hourly mode
right now" by re-deriving it from raw funding-rate history would violate that boundary.

The resolution adopted here: model the **current per-symbol interval as an ingested, evidenced
fact** (`PublishedPerSymbolOverride`) — i.e., Mark Time ingests Binance's own published per-symbol
funding-interval state (Binance's API/announcements expose which mode a symbol is currently in)
the same way it ingests any other structured-feed fact, with its own `Evidence` and its own short
validity window, rather than computing the trigger condition itself from price history. This
keeps "when a contract enters/leaves hourly mode" a *sourced* fact rather than a *derived*
calculation, consistent with Principle I, at the cost of very frequent, narrow-validity rule rows
for any symbol currently near its cap/floor.

**This is a tradeoff, explicitly not settled by research.md**: research.md documents the
*existence* and *thresholds* of the volatility trigger (D5) but does not document the exact
per-symbol feed, its refresh cadence, or its endpoint. **Needs verification before
implementation** — this field's ingestion source is a gap in Phase 0 research, named here rather
than papered over with an invented endpoint.

---

## 4. Evidence

Per Principle I and FR-009, every `Rule` carries evidence. The task brief lists the four fields
literally ("source_url, fetched_at, effective_from, and source_updated_at where the publisher
provides one"); the model below keeps that as a direct 1:1 composition on `Rule` rather than
introducing a many-Rules-to-one-Evidence indirection, so "does this rule carry evidence" is
answered by looking at one record, not by chasing a shared foreign key.

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `evidence_id` | opaque id | required | — |
| `source_url` | URI | required | FR-009 |
| `fetched_at` | `Instant` | required | FR-009 — when *we* retrieved the document |
| `effective_from` | date/instant | required | FR-009 — the date the source says the rule takes effect; this is the single value `Rule.effective_from` (§3.1) reads, not a duplicate of it |
| `source_updated_at` | `Instant`, optional | optional | FR-009 — "where the upstream publishes one" (Principle I); e.g. no value for SSE's Trading Rules PDF unless it carries its own revision date, present where a source exposes a last-modified/announcement date |
| `is_derived` | boolean | required | FR-010 |
| `derivation_reasoning` | text | **required iff `is_derived = true`**, absent otherwise | FR-010: "MUST carry the reasoning"; a validation rule pairs the two fields — derived-without-reasoning and reasoning-without-derived-flag are both invalid states |
| `entered_by` | string (person/agent identifier), optional | required for human-transcribed evidence (i.e., whenever the source class is unstructured — D7: SSE and NYSE are HTML/PDF only) | Domain and Data Constraints: "a human-entered rule is acceptable evidence when it records who entered it, from which document, and when" |

**Storage-layer note, not a relationship change**: several sibling `Rule` rows legitimately share
identical evidence values when they come from one document capture at one `fetched_at` — D4's six
SSE segment rows all cite the same Trading Rules PDF revision. The conceptual model keeps this as
six `Evidence` records with identical field values (simplicity, literal-per-rule reading); a
storage layer MAY physically deduplicate identical evidence payloads as an implementation detail
without changing this model.

**Licensing note (D6)**: for SSE, `source_url` and the evidence fields are recorded, but the
underlying document is **not vendored** into the repository (SSE Trading Rules Art. 5.1.3 —
non-redistributable). `market-time-data` (a separate crate, per plan.md) is the boundary that
holds this distinction; `Evidence` itself is unaffected — it always cites the source regardless of
whether the payload is stored, which is exactly what SC-002/SC-008 need (a person can still open
the cited source) without requiring redistribution.

---

## 5. Coverage

The explicit declaration of the time range a body of rule data is valid for (spec Key Entities);
FR-002's explicit-unknown rule is enforced against this declaration (FR-018).

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `coverage_id` | opaque id | required | — |
| `venue_id` | FK → Venue | required | — |
| `dataset_kind` | enum/string (e.g. `SseCalendar`, `NyseCalendar`, `BinanceFundingRules`, `IanaTzdata`) | required | a venue can have more than one coverage-bearing dataset with independently declared ranges (e.g. calendar vs. funding-rule coverage for Binance) |
| `valid_from` | `Instant` | required | — |
| `valid_until` | `Instant`, optional | optional (open-ended if absent, e.g. a calendar with no known forward limit yet) | FR-018 |
| `revision_id` | FK → DatasetRevision | required | coverage is declared **per dataset revision** — a correction can also correct the declared range |

**Enforcement of FR-002**: `[valid_from, valid_until)` uses the half-open convention (§ Conventions
above). A queried instant `t` is in coverage iff `valid_from <= t < valid_until` (or `t >=
valid_from` when `valid_until` is absent). Outside that range, the answer is `Unknown` (§10), and
`Unknown` carries which `Coverage` record and which boundary (`valid_from` or `valid_until`) was
the deciding one, so the answer "names the coverage boundary" (Acceptance Scenario 5).

---

## 6. Dataset revision

An immutable, identified version of a body of rule data (Principle III; spec Key Entities).

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `revision_id` | opaque, stable id | required | Principle III — "stable identifiers"; MUST NOT be reused or reassigned once published |
| `dataset_kind` | enum/string | required | same vocabulary as `Coverage.dataset_kind`, plus `IanaTzdata` for the tzdata release itself (D2) |
| `created_at` | `Instant` | required | when this revision was published into the system |
| `source_description` | text | required | e.g. "SSE Trading Rules, 2026 Revision, effective 2026-07-06" |
| `supersedes` | FK → DatasetRevision, optional | optional | present when this revision exists specifically to correct a prior one — Principle III: "correcting a rule produces a new revision, never an edit in place" |

**Resolved tradeoff — full snapshot per revision, not a delta chain.** Two designs were
considered: (a) each `DatasetRevision` is self-contained, holding a complete copy of every `Rule`
that applies under it, even unchanged ones; (b) each revision stores only the delta from
`supersedes`, and reconstructing "the rule set in force" requires walking the chain. **(a) is
adopted**, justified by the spec's own Assumption that "the tracked data is thousands of records,
not millions" (no storage pressure forces a delta scheme) and because it makes FR-017 ("given the
same revision set and the same query, return the same answer") trivial: resolving an answer only
ever needs one revision's worth of rules, never a walk. The cost — accepted deliberately — is that
an unrelated correction elsewhere in a large calendar means re-publishing a revision that repeats
many unchanged rows; acceptable at this data scale.

**Attribution (FR-016)**: any produced answer (Phase, Event, or Unknown, §10) carries the set of
`revision_id`s actually consulted to produce it — at minimum the `IanaTzdata` revision (needed
for any civil-zone conversion) plus the venue's own calendar/rule revision(s), and for Binance
events, the funding-rule revision. More than one calendar revision can appear in one answer's
attribution set only if the winning rule and a supporting rule (e.g. the base weekly pattern
under a holiday override) happen to come from different revisions — normally they will not, since
(a) above snapshots the whole rule set together.

---

## 7. Phase

### 7.1 Shared vocabulary (FR-005, FR-006)

Exactly these eight kinds. No venue introduces its own name (FR-005; SC-006).

| Phase kind | Meaning |
|---|---|
| `closed` | not trading and no venue-run process is active |
| `pre_open` | a published pre-market state distinct from closed, before any auction/order-matching activity begins |
| `opening_auction` | a scheduled call-auction / order-matching process that sets the open |
| `continuous_trading` | ordinary two-sided continuous matching |
| `mid_day_break` | a scheduled intraday halt that is neither open nor closed (spec: "a two-state open/closed model cannot express" this) |
| `closing_auction` | a scheduled call-auction / order-matching process that sets the close |
| `post_close` | a published post-market state distinct from closed (e.g. a fixed-price window) |
| `non_trading_interruption` | an unscheduled halt or an announced maintenance window inside what would otherwise be a trading state |

### 7.2 `PhaseSegment` (the resolved/derived instance)

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `venue_id` | FK → Venue | required | — |
| `phase_kind` | one of §7.1 | required | FR-005 |
| `start` | `Boundary` (§8) | required | FR-003, FR-004 |
| `end` | `Boundary` (§8) | required | FR-003, FR-004 — always present within coverage; every venue's schedule cycles, so no segment is unbounded while inside declared coverage |
| `evidence_refs` | 1:N FK → Evidence | required, non-empty | the rule(s) whose evidence produced this specific segment (typically the one winning `Rule` — a `HolidaySession`/`ShortenedSession` override, or the applicable `WeeklyPattern` row) |
| `revision_ids` | 1:N FK → DatasetRevision | required, non-empty | FR-016 |

A `PhaseSegment`'s `start` is inclusive and `end` is exclusive (§ Conventions), so the instant
exactly on a boundary belongs to the segment that **starts** there — this is the deterministic
rule Acceptance Scenario 6 and FR-004 require; it is a single stated convention rather than a
per-boundary decision that resolution code would otherwise have to make up case by case.

### 7.3 `PhaseTimeline` and the tiling invariant (FR-008) — enforced structurally

FR-008 requires phases to "cover all time within coverage without gaps or overlaps," and the
brief specifically asks for this to be structural, not conventional. The design:

- **`PhaseSegment`s are never stored as independently editable records.** There is no operation
  that inserts, deletes, or edits one segment in isolation. Segments only ever come out of a
  single construction path: a timeline-resolution operation that takes a venue id and a coverage
  range and produces a `PhaseTimeline`, which
  folds the venue's applicable `Rule`s (in the fixed precedence order from §3.1) into a sorted
  sequence of segments.
- **`PhaseTimeline` is a smart-constructed type**, not a bare, unordered list of `PhaseSegment`
  values. Its constructor
  is the *only* way to obtain one, and it checks — refusing construction (returning a typed
  building error, not a partially-built value) if violated:
  1. the first segment's `start` equals the coverage range's `valid_from`;
  2. the last segment's `end` equals the coverage range's `valid_until` (or is open, matching an
     open-ended coverage);
  3. for every adjacent pair, `segment[i].end == segment[i+1].start` exactly (no gap, and — because
     the half-open convention means a segment cannot overlap its own end — no overlap either).
- Because rule data (WeeklyPattern/HolidaySession/ShortenedSession/AnnouncedChange) is itself
  allowed to be incomplete or in tension for a given date (e.g., a `WeeklyPattern` row alone
  leaves the SSE 09:25–09:30 span unassigned, see §7.4 below), condition 3 is precisely the check
  that surfaces such a gap as a *build failure of the timeline*, at the moment the data is
  resolved — not as a silently-wrong answer served later. This turns FR-008 from a rule authors
  must remember into a check the resolution step cannot skip.
- **Events (§9) get no equivalent constructor.** They are stored as a plain, independently
  addressable collection with no contiguity check — deliberately, per FR-008's second clause and
  the spec's Key Entities ("Events may be absent, sparse, or overlapping in a way phases may
  not").

**Open question, left unresolved (a genuine style choice, not a correctness question)**: whether
adjacent segments of the *same* `phase_kind` (which can arise, e.g., from a rule boundary that
doesn't actually change the observable phase) should be coalesced into one segment before being
handed to a consumer. Coalescing changes nothing about which phase owns which instant, only the
segment count. Left as a presentation-layer/rendering decision rather than fixed here, since
nothing in the spec or research depends on it either way.

### 7.4 Flagged gap in source data — SSE's un-narrated intervals

D4's published SSE table (Art. 2.4.2) has two spans with no assigned row:

| Span (Beijing local) | Between | Status |
|---|---|---|
| 09:25–09:30 | end of opening call auction, start of continuous trading | **NEEDS VERIFICATION** — not named by the source in research.md |
| 15:00–15:05 | end of closing call auction, start of after-hours fixed price | **NEEDS VERIFICATION** — not named by the source in research.md |

FR-008 requires these spans to belong to *some* phase before an SSE `PhaseTimeline` can be built
at all (§7.3's constructor would otherwise fail). Rather than assign a plausible-sounding phase
(e.g., guessing `closed` or a bespoke gap state) from general knowledge of exchange mechanics, this
is left as an explicit ingestion prerequisite: whoever authors the SSE `WeeklyPattern` `Rule` rows
must source what SSE itself calls these spans (or establish, evidenced, that they are folded into
the adjacent `opening_auction`/`closing_auction`/`post_close` segment) before the rule can satisfy
the timeline constructor. **research.md does not supply this fact, so it is not invented here.**

### 7.5 Flagged gap in source data — NYSE's full phase-time table

Unlike SSE, research.md does not carry a complete published phase-time table for NYSE. What *is*
sourced: `continuous_trading` begins as a process at 09:30 local (D3), holidays map to `closed`
(Acceptance Scenario 3), and shortened sessions end `continuous_trading` at 13:00 local on
specific published dates (D6, footnote text still needing a byte-exact pass per research.md's
"Carried into Phase 1" item 3). Pre-market/`pre_open`, `opening_auction`, `closing_auction`, and
`post_close` timing for NYSE are commonly known facts from general market knowledge, but **are not
present in research.md** and are therefore marked **NEEDS VERIFICATION** here rather than filled
in — per the brief's explicit instruction not to invent venue facts. The §7.3 timeline constructor
will fail for NYSE until these are sourced and evidenced the same way D4 sourced SSE's.

---

## 8. Uncertainty and `Boundary`

### 8.1 `Boundary`

The value used for every phase-segment `start`/`end` (§7.2) and every event's scheduled instant
(§9) — one "fuzzy instant" concept, reused rather than duplicated.

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `instant` | `Instant` (UTC) | required | the published/nominal instant |
| `uncertainty` | `Uncertainty` (§8.2) | required | FR-011 — every answer-bearing instant carries uncertainty, never a bare point estimate |

### 8.2 `Uncertainty`

Must express three distinct things the brief calls out — publication precision, a venue's own
published bound, and "boundary is a process start" — as one composable structure rather than
three unrelated ad hoc fields bolted on over time.

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `granularity` | duration (e.g. "1 minute", "1 second"), optional | optional | FR-011 — the rounding quantum the *source itself* publishes to (SSE/NYSE tables are stated to the minute; Binance funding settlement times are effectively second-level) |
| `published_bound` | duration, optional (symmetric for now — Binance's is ±15 s; no launch venue publishes an asymmetric bound) | optional | FR-011a — "where a venue publishes its own imprecision, the stated uncertainty MUST be no narrower than what the venue published"; sourced verbatim from D5a for Binance funding |
| `boundary_character` | enum: `Instantaneous \| ProcessStart` | required | FR-011b — distinguishes an ordinary state-change instant from "the scheduled start of a process" |
| `process_spread` | duration, optional; **absent for NYSE at launch** | present only if the venue itself publishes how long the process typically spreads over | D3: NYSE describes *that* opening is a rolling, security-by-security process but does not publish a quantified spread. Recording a spread the source doesn't state would itself be a fabricated-precision violation of Principle II — so this field is deliberately left absent for NYSE rather than estimated, and the honest answer is `boundary_character = ProcessStart, process_spread = None`, i.e., "known to be a process, magnitude not sourced." |
| `note` | free text, optional | optional | human-readable gloss, e.g. "NYSE opening is a per-security DMM-driven process; 09:30 is the scheduled start of that process, not a market-wide state change" (D3) |

**Composition rule (validation, not just a field)**: the *reported* uncertainty for a boundary
MUST be no narrower than `max(derived-from-granularity bound, published_bound)` when both are
present — this is the concrete mechanism behind FR-011a, not a separate concept from
`granularity`. A rule that would report a tighter bound than the venue's own published one is
invalid.

**Distinctness reaffirmed (Principle II, FR-012)**: `Uncertainty` describes imprecision in the
*published* boundary. It never encodes anything about what the matching engine actually did —
that is out of scope for this slice (spec Assumptions: "Published, not observed") and keeping it
out of `Uncertainty`'s fields is what keeps published and observed from being conflated in the
type itself, not just in prose.

---

## 9. Event

Something scheduled that occupies a point or short window and does not replace the phase it sits
inside (spec Key Entities; FR-007). Events are not required to tile (FR-008) — see §7.3's last
bullet.

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `event_id` | opaque id | required | — |
| `venue_id` | FK → Venue | required | — |
| `event_kind` | enum, extensible (launch value: `FundingSettlement`) | required | FR-007; funding is the only launch case (Resolved Scope Decisions: perpetual futures chosen specifically because funding exercises this) |
| `scheduled_at` | `Boundary` (§8) | required | reuses the same fuzzy-instant type as phase boundaries — funding's `published_bound = 15s` (D5a) is expressed exactly the same way NYSE's `ProcessStart` character is, through `Uncertainty`, not a bespoke event-only field |
| `generating_rule_id` | FK → `EventRecurrenceRule` (§3.3) | required | which specific recurrence-rule row (and therefore which `interval_kind`/`scope`/threshold state) produced this occurrence — needed for FR-016 attribution given D5's rule set is itself versioned over time |
| `evidence_refs` | 1:N FK → Evidence | required, non-empty | Principle I applies to events the same as phases |
| `contract_symbol` | string, optional (present for `FundingSettlement`) | required when `event_kind = FundingSettlement` | funding is per-contract, not venue-wide (D5) |

**Overlap and sparsity are permitted by construction**: unlike `PhaseTimeline`, there is no
constructor invariant over the `Event` collection — it is an ordinary addressable set, queried by
venue and time range, with no contiguity or non-overlap check. This is the direct structural
counterpart to §7.3's tiling constructor: phases get an enforcing smart constructor, events
deliberately do not.

---

## 10. Query and Answer

Not one of the eight Key Entities named in the spec, but required to make FR-002's "explicit
unknown, distinct from closed and distinct from error" concretely representable rather than left
as a documentation aspiration. Modeled here as the shape that composes everything above.

### 10.1 `Query`

| Field | Type intent | Req/Opt | Constraint / grounding |
|---|---|---|---|
| `venue_id` | FK → Venue, or a set of venue_ids for the multi-venue case | required | FR-001; FR-019 for the multi-venue board query |
| `instant` | `Instant` (already UTC) **or** `CivilInstant` (§1.3) | required, exactly one form | FR-013 |

### 10.2 `Answer` — a three-way outcome, not two

| Outcome | Carries | Distinctness |
|---|---|---|
| Known phase | a `PhaseAnswer`: `phase_kind` (§7.1 — **`closed` is a normal value of this**, not the absence of an answer), the owning `PhaseSegment`'s `start`/`end` `Boundary`s, `evidence_refs`, `revision_ids` | a fully known, in-coverage answer (FR-001, FR-003) |
| Explicit unknown | an `UnknownReason`: which `Coverage` record was checked, which boundary (`valid_from`/`valid_until`) was violated, and by how much | FR-002: explicit, first-class, distinct from both the known-`closed` outcome and a query error — "outside coverage" is a fact about the *dataset*, not a claim about the venue and not a system failure |

A malformed query (e.g. an unresolved Ambiguous/Nonexistent `CivilInstant` per §1.3, or an unknown
`venue_id`) is deliberately **not** a third outcome of `Answer` at all — it is a rejection of the
query before an answer is attempted, one level up. This is what makes "distinct from error"
concrete: the explicit-unknown outcome is something the system *asserts* about a well-formed query
("this instant is outside what I have data for"); a query error is the system declining to attempt
an answer because the question itself wasn't well-formed. Conflating the two would let a bad
query silently read as "the venue's state is unknown," which is a different and weaker claim.

### 10.3 Reproducibility and attribution (FR-016, FR-017)

Every known-phase answer and every explicit-unknown answer (an unknown is still attributable — it
names the `Coverage`, which belongs to a `DatasetRevision`) carries its `revision_ids` set. FR-017 follows
directly from §6's "full snapshot per revision" resolution: the same `revision_ids` plus the same
`Query` re-runs the same fold over the same immutable rule rows, with nothing else in the system
(no clock read, per Principle IV) able to perturb the result.

### 10.4 Multi-venue composition (FR-019, FR-020)

A multi-venue request is a fan-out: one `Query` per tracked venue at the same `instant`, each
independently producing its own `Answer`. No new entity is needed — FR-020's "one gap does not
void the whole view" falls out of `Answer` already being computed per-venue rather than as one
joint structure that could fail atomically.

---

## 11. Venue → shared vocabulary mapping (FR-005, FR-006, SC-006)

Grounded only in what research.md states; venue-specific phase names are never introduced (zero
per SC-006). Cells marked **NEEDS VERIFICATION** are gaps in Phase 0 research, not settled facts.

| Phase kind | SSE (D4) | NYSE (D3, D6, Acceptance Scenarios) | Binance USD-M perp (D5, spec Edge Cases) |
|---|---|---|---|
| `closed` | all non-listed hours; full-day holidays | holidays (Acceptance Scenario 3) | N/A — always-on venue (spec Edge Case "Always-on venue": "closed never occurs") |
| `pre_open` | not used (no gap in D4 is identified as this — see §7.4) | **NEEDS VERIFICATION** | N/A |
| `opening_auction` | 09:15–09:25 Beijing (D4) | **NEEDS VERIFICATION** (D3 establishes the *process* nature of the open, not a published auction window) | N/A |
| `continuous_trading` | 09:30–11:30, 13:00–14:57 Beijing (D4) | begins at 09:30 local as a scheduled *process start*, per D3 — `Boundary.uncertainty.boundary_character = ProcessStart` (FR-011b); shortened on published early-close dates to end 13:00 local (D6) | the default state; effectively always, except during `non_trading_interruption` |
| `mid_day_break` | 11:30–13:00 Beijing (D4) | not applicable — no US mid-day break sourced | N/A |
| `closing_auction` | 14:57–15:00 Beijing (D4) | **NEEDS VERIFICATION** | N/A |
| `post_close` | after-hours fixed price, 15:05–15:30 Beijing (D4, Art. 3.7.2) | **NEEDS VERIFICATION** | N/A |
| `non_trading_interruption` | not sourced as a distinct SSE case (Art. 2.4.3 implies full open/closed only, per D4 — "Mainland Chinese exchanges are fully open or fully closed for the day") | not sourced in research.md for NYSE | announced maintenance windows scoped to a product line, evidenced per-incident ("Notice of…" / "…Complete" announcements, D5a) — only when the announcement confirms USDⓈ-M is affected |

Binance funding settlements are **never** a phase kind in this table — they are `Event`s (§9)
overlaid on whatever `continuous_trading`/`non_trading_interruption` segment they fall inside
(FR-007), which is exactly the case D5/D5a exist to validate.

---

## 12. Entity relationship summary

```
Venue 1──N Coverage ──N:1 DatasetRevision
Venue 1──N Rule ──1:1 Evidence
Rule    N:1 DatasetRevision
Rule (kind = EventRecurrenceRule) 1──N Event ──N:1 Evidence (evidence_refs)
Venue ──(timeline resolution, not stored)──> PhaseTimeline ──ordered──> PhaseSegment ──N:1 Evidence (evidence_refs)
PhaseSegment start and end are each a Boundary, which pairs an Instant with an Uncertainty
Event's scheduled_at is likewise a Boundary (Instant + Uncertainty)
Query ──resolve──> either a known-phase answer or an explicit-unknown answer
  (a query error is handled out-of-band, before an answer is attempted — not a third answer outcome)
Both answer outcomes carry revision_ids : N:1 DatasetRevision (attribution, FR-016)
```

---

## 13. Consolidated open questions and tradeoffs

Restated together for visibility (each is also flagged inline above at its point of relevance):

1. **Interval convention (§ Conventions)** — half-open `[start, end)` chosen uniformly for
   coverage, phase segments, and rule effective ranges. This is an interpretive resolution of
   ambiguity in the spec's prose, not a fact the spec states outright.
2. **SSE's two un-narrated table gaps (§7.4)** and **NYSE's largely absent phase-time table
   (§7.5)** — both marked NEEDS VERIFICATION rather than filled from general market knowledge,
   per the explicit instruction not to invent venue facts. Both block the §7.3 timeline
   constructor for their respective venues until sourced.
3. **Binance per-symbol funding-interval state (§3.3)** — modeled as an ingested, evidenced fact
   (`PublishedPerSymbolOverride`) rather than computed from raw funding-rate history, specifically
   to keep Mark Time out of market-data-feed territory (spec Assumptions). The feed itself is not
   named in research.md and needs verification before implementation.
4. **Evidence composition (§4)** — kept as a direct 1:1 field set on `Rule` per the brief's literal
   phrasing, with sibling-row deduplication left as a storage-layer optimization rather than a
   modeled many-to-one relationship, so "does this rule carry evidence" never requires chasing an
   indirection.
5. **Same-`phase_kind` segment coalescing (§7.3)** — left unresolved as a rendering choice; it does
   not affect which phase owns which instant.
6. **NYSE opening `process_spread` (§8.2)** — deliberately left `None` rather than estimated,
   because research.md documents that a process exists (D3) but not its quantified duration.
