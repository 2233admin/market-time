# Feature Specification: Venue Session State

**Feature Branch**: `001-venue-session-state`

**Created**: 2026-07-29

**Status**: Draft

**Input**: First delivery slice for Mark Time — answer what trading phase a venue is in at a given instant, with source evidence and honest uncertainty, for SSE, NYSE, and Binance.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Phase at an instant (Priority: P1)

Someone needs to know what state a trading venue is in at a specific moment — right now, earlier today, or on a date months out. They name the venue and the instant; they get back the phase the venue is in, when that phase started, and when it ends.

Crucially, if the question falls outside what Mark Time actually knows, they are told so plainly rather than being handed a plausible-looking guess.

**Why this priority**: This is the product. Every other capability decorates this answer. A single venue answering correctly is already useful to someone who trades or researches that venue.

**Independent Test**: Pick one venue. Query a normal trading day, a holiday, a half-day, and a date beyond the loaded calendar. Correct phases for the first three and an explicit unknown for the fourth means the slice delivers value on its own.

**Acceptance Scenarios**:

1. **Given** SSE and an instant at 10:00 Shanghai time on a normal trading day, **When** the phase is requested, **Then** the answer is continuous trading, with the start and end of that phase.
2. **Given** SSE and an instant at 12:00 Shanghai time on a normal trading day, **When** the phase is requested, **Then** the answer is the mid-day break — not "closed" and not "open".
3. **Given** NYSE and an instant on a US market holiday, **When** the phase is requested, **Then** the answer is closed, and the holiday is identified.
4. **Given** NYSE and an instant during an early-close session, **When** the phase is requested, **Then** the shortened schedule is reflected, not the normal one.
5. **Given** any venue and an instant beyond the loaded calendar's declared coverage, **When** the phase is requested, **Then** the answer is an explicit unknown that names the coverage boundary — never an extrapolated schedule.
6. **Given** an instant that falls exactly on a phase boundary, **When** the phase is requested, **Then** the answer states unambiguously which phase owns that instant.

---

### User Story 2 - Evidence behind the answer (Priority: P2)

Someone doubts an answer, or has to justify a decision that depended on it. They ask where the answer came from and get the specific document the schedule was read from, when it was read, and the date from which it applies — enough to open the source and check it themselves.

**Why this priority**: This is what separates Mark Time from any calendar library. Without it the product is a rumour with good formatting. It is P2 only because a correct answer must exist before it can be justified.

**Independent Test**: Take any answer from User Story 1 and ask for its provenance. If every element of that answer traces to a named, dated, openable source, this story stands alone.

**Acceptance Scenarios**:

1. **Given** any phase answer, **When** its evidence is requested, **Then** the response names the source document, the time it was retrieved, and the date the rule takes effect.
2. **Given** a rule that was reasoned about rather than published verbatim, **When** its evidence is requested, **Then** it is marked as derived and the reasoning accompanies it.
3. **Given** a venue that publishes its schedule only to the second, **When** an answer's precision is inspected, **Then** the stated uncertainty reflects second-level publication, not nanosecond exactness.
4. **Given** the same query re-run later against the same dataset revisions, **When** the answers are compared, **Then** they are identical.

---

### User Story 3 - What is open right now, everywhere (Priority: P3)

Someone wants the global picture rather than one venue: at this instant, which of the tracked venues are open, which are in an auction, which are on a break, and which are closed — read at a glance, in one query.

**Why this priority**: This is the experience people actually come for, but it is composition on top of stories 1 and 2. It cannot be right before the single-venue answer is right.

**Independent Test**: Query the global view at an instant when the three launch venues are deliberately in different states. Seeing all three correct states side by side, each still carrying its evidence, proves this story independently.

**Acceptance Scenarios**:

1. **Given** an instant when Shanghai is on its mid-day break, New York is closed, and Binance is trading, **When** the global view is requested, **Then** all three states appear correctly in one response.
2. **Given** one venue in the set is outside its coverage at the queried instant, **When** the global view is requested, **Then** that venue reports unknown while the others still report their phases — one gap does not void the whole view.
3. **Given** the global view, **When** any single venue's entry is inspected, **Then** it carries the same evidence and uncertainty a single-venue query would return.

---

### Edge Cases

- **Spring-forward**: a local wall-clock time in New York that does not exist on the day the clocks move forward. The answer must not silently invent it.
- **Fall-back**: a local wall-clock time in New York that occurs twice. The answer must resolve which occurrence is meant, or say it cannot.
- **Early close**: a venue trading a shortened session (for example the sessions around certain US holidays) where the normal schedule would give the wrong answer.
- **Mid-day break**: a venue that is neither open nor closed for a stretch of the trading day, which a two-state open/closed model cannot express.
- **Unscheduled halt or maintenance**: a venue stops trading inside a window where the published schedule says it should be trading.
- **Future-dated schedule change**: a venue announces a new schedule effective next quarter. Queries before the effective date must use the old schedule and queries after it the new one, from the same dataset.
- **Open is a process, not an instant**: NYSE opens security by security through a market-maker-driven process, so the moment any given security becomes tradable is neither simultaneous across the market nor published in advance. The venue-level "09:30" is the scheduled start of that process, not a claim that anything traded at that instant. (Corrected during Phase 0 research — an earlier draft assumed randomisation. Venues that genuinely randomise their open exist but none is in the launch set.)
- **Boundary instant**: the exact nanosecond a phase ends and the next begins.
- **Coverage edge**: the first and last instants inside declared coverage, and the instants immediately outside them.
- **Always-on venue**: a venue that never closes, so "closed" never occurs but scheduled events still do.

## Requirements *(mandatory)*

### Functional Requirements

**Answering**

- **FR-001**: System MUST return the phase a named venue is in for any queried instant that falls within that venue's declared coverage.
- **FR-002**: System MUST return an explicit unknown for any instant outside declared coverage, and MUST NOT extrapolate a schedule past its verified range.
- **FR-003**: System MUST report the start and end of the returned phase, subject to the same evidence and uncertainty rules as the phase itself.
- **FR-004**: System MUST resolve boundary instants deterministically, so that every instant within coverage belongs to exactly one phase.

**Shared vocabulary**

- **FR-005**: System MUST express every venue's state using one shared phase vocabulary. No venue may introduce a phase name of its own.
- **FR-006**: The phase vocabulary MUST be able to express, at minimum: closed, pre-open, opening auction, continuous trading, mid-day break, closing auction, post-close, and non-trading interruption.
- **FR-007**: System MUST represent scheduled recurring occurrences that are not states — such as crypto funding settlements — as events overlaid on a phase, not as phases themselves.
- **FR-008**: Phases MUST cover all time within coverage without gaps or overlaps; events MUST NOT be required to do so.

**Evidence and honesty**

- **FR-009**: Every answer MUST carry the evidence for the rules that produced it: the source document, when it was retrieved, and the date from which it takes effect.
- **FR-010**: System MUST mark any rule that was derived or inferred rather than published, and MUST carry the reasoning with it. Derived MUST NOT be presented as observed.
- **FR-011**: Every answer MUST carry an uncertainty statement that reflects the precision of its underlying source, and MUST NOT present an answer as more precise than its evidence supports.
- **FR-011a**: Where a venue publishes its own imprecision, the stated uncertainty MUST be no narrower than what the venue published. (Binance publishes a 15-second deviation on funding settlement times — a bound handed to us, not one to estimate.)
- **FR-011b**: Where a venue's published boundary is the scheduled start of a process rather than an instant at which the market changes state, the answer MUST reflect that the boundary has a spread. A process start MUST NOT be presented as an instantaneous transition. (NYSE opens security by security via a market-maker-driven process; 09:30 starts that process rather than marking a market-wide state change.)
- **FR-012**: System MUST keep a venue's published schedule and its observed behaviour as distinct claims, and MUST NOT present one as the other.

**Time handling**

- **FR-013**: System MUST accept and return instants at nanosecond resolution with an explicitly declared time scale.
- **FR-014**: System MUST correctly resolve venues in zones that observe daylight saving, including local times that do not exist and local times that occur twice.
- **FR-015**: System MUST apply venue holidays and shortened sessions in preference to the normal weekly schedule.

**Reproducibility**

- **FR-016**: System MUST report which dataset revisions produced an answer.
- **FR-017**: Given the same dataset revisions and the same query, System MUST return the same answer.
- **FR-018**: Each dataset MUST declare the range of time it is valid for, and FR-002 MUST be enforced against that declaration.

**Composition**

- **FR-019**: Users MUST be able to obtain the phases of all tracked venues at a single instant in one request.
- **FR-020**: In a multi-venue request, a venue outside its coverage MUST report unknown without suppressing the other venues' answers.

### Key Entities

- **Venue**: a market whose trading schedule is tracked. Has a home time zone, a declared coverage range, and a schedule. The launch set is Shanghai Stock Exchange, New York Stock Exchange, and Binance.
- **Phase**: a state a venue is in for a contiguous stretch of time, drawn from the shared vocabulary. Phases tile all covered time with no gaps and no overlaps.
- **Event**: something scheduled that occupies a point or short window and does not replace the phase it sits inside — for example a crypto funding settlement. Events may be absent, sparse, or overlapping in a way phases may not.
- **Rule**: a statement about a venue's schedule (a weekly pattern, a holiday, a shortened session, an announced change) together with the range of dates it applies to.
- **Evidence**: the provenance attached to a rule — the source document, when it was retrieved, the date it takes effect, and, where the publisher provides one, when the publisher last changed it.
- **Coverage**: the explicit declaration of the time range over which a venue's data is valid. Queries outside it return unknown.
- **Uncertainty**: the statement of how precisely an answer is known, derived from the precision and character of its sources.
- **Dataset revision**: an immutable, identified version of a body of rule data. Answers are attributable to the revisions that produced them.

## Success Criteria *(mandatory)*

- **SC-001**: For each launch venue, phase answers match that venue's published schedule across a full year of sampled instants, including every holiday, shortened session, and daylight-saving transition in that year, with zero mismatches.
- **SC-002**: Every answer returned includes at least one source reference that a person can open and independently verify — no answer is returned without one.
- **SC-003**: Every query outside declared coverage returns an unknown; across a deliberate sweep of out-of-range instants, no guessed phase is ever returned.
- **SC-004**: A user can determine the state of all three launch venues at any single instant in one request.
- **SC-005**: Re-running a sample of queries against unchanged dataset revisions returns byte-identical answers.
- **SC-006**: All three launch venues are described entirely in the shared phase vocabulary, with zero venue-specific phase names introduced.
- **SC-007**: For every answer, the stated uncertainty is no narrower than the publication precision of the source it rests on — verified across a sample spanning all three venues.
- **SC-008**: A person unfamiliar with the system can, from a single answer, reach the underlying source document without assistance.

## Assumptions

- **Scale**: UTC is the scale in which answers are published. Inputs on other scales are accepted only when explicitly labelled.
- **Launch venues**: the slice covers exactly three venues — Shanghai Stock Exchange, New York Stock Exchange, and Binance. They were chosen because they exercise three structurally different market models: an auction-plus-break cash equity market, a daylight-saving-sensitive continuous market, and an always-on crypto venue with scheduled events.
- **Published, not observed**: this slice delivers published schedules only. Observed behaviour — what the matching engine actually did — is out of scope here, but the model keeps the two distinguishable so observed data can be added later without redefining anything. This follows from the constitution's requirement that the two never be conflated.
- **Coverage depth**: initial coverage is whatever the venues' own published calendars span. Coverage is declared per dataset rather than assumed, so a shallow initial range is acceptable and honest.
- **Not a market data feed**: Mark Time answers questions about schedules and rules. It does not carry prices, quotes, or trades, and it does not observe live venue activity.
- **Rule data is small**: the tracked data is thousands of records, not millions. Bulk-scale storage concerns do not shape this slice.
- **Consumers**: technical systems querying programmatically, and people checking a venue's state directly. Both need the same evidence.
- **Unscheduled interruptions are recorded, not predicted**: the system reports a halt or maintenance window once it is published. It does not attempt to anticipate one.
- **No single downstream shapes this**: venue coverage, schema, and field naming are chosen on this product's own merits. A requirement justified only as "another project needs it" is out of scope. Mark Time does not know its consumers exist.
- **Aggregators are design references, never sources**: conventional global trading-hours boards are useful for deciding *what to display*. They are not admissible as *where the data comes from* — a venue's own published schedule is evidence, an aggregator repeating it is a second-hand claim. Separately, at least one well-known financial aggregator's published terms explicitly forbid reuse of its data for AI training or commercial purposes (checked 2026-07-29), which makes the licensing position concrete rather than theoretical. Sources are first-party.

## Resolved Scope Decisions

Both open questions were resolved on 2026-07-29. Recorded here rather than deleted, so the
reasoning survives.

- **Binance product scope — resolved: perpetual futures (USD-M).** Funding settlements exist
  on perpetual futures and not on spot. Funding was named in the product's founding scope, and
  it is the only thing in the launch set that exercises the phase-versus-event distinction
  (FR-007) against a real case rather than a hypothetical one. Spot would leave that
  distinction untested until a later slice.
- **Visual board — resolved: included in this slice.** User Story 3 ships as a board, not only
  as a query. This is a deliberate order: match what a conventional global trading-hours board
  already does, then carry the evidence and uncertainty layer through to the surface. The board
  is a shell over the core and holds no domain logic (Principle IV). Two constraints follow and
  are non-optional: the board reads `now` and passes it in rather than the core reading a clock,
  and where the board displays "now" it MUST surface the host's clock discipline bounds as
  uncertainty rather than presenting the host clock as exact.

**Known hard part, accepted deliberately**: unknown and uncertainty are harder to render than
to compute. A board showing "Shanghai: mid-day break" is easy; a board showing "New York: open,
boundary known only to the second, opening instant randomised by the venue" is not. The board
MUST NOT resolve this by hiding uncertainty. If a display cannot express an honest answer, the
display changes — the answer does not.
