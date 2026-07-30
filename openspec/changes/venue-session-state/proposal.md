## Why

Mark Time answers one question: what trading phase is a venue in at a given instant, with the
evidence behind the answer and honest uncertainty around it. Nothing in the repository answers
that question yet — the workspace, the pinned tzdata, and the CI gates exist, and the domain
types do not.

The consumers, in priority order, are autonomous agents that trade, then other systems querying
programmatically, then people. That ordering is a design constraint rather than positioning: a
person can see that a rendered figure looks approximate, an agent cannot. Unknown and closed are
therefore different values, not one value displayed two ways.

This change carries the first delivery slice — three venues chosen because they exercise three
structurally different market models: Shanghai (auction plus a mid-day break), New York
(daylight-saving-sensitive, with early closes and an open that is a process rather than an
instant), and Binance (always on, with scheduled funding settlements that are events rather than
phases).

## What Changes

- A shared phase vocabulary every venue is described in, with no venue-specific phase names, and
  a separate event concept for scheduled occurrences that do not replace the phase they sit in.
- A phase query that returns the phase, its start, and its end for any instant inside a venue's
  declared coverage — and an explicit unknown, naming the coverage boundary, for any instant
  outside it. No extrapolation past a verified range.
- Evidence and uncertainty carried in every answer as structural values: the source document,
  when it was retrieved, the date it takes effect, whether a rule was published or derived, and
  an uncertainty statement no narrower than the source's own precision.
- Nanosecond instants on an explicitly declared time scale, correct daylight-saving handling
  including local times that do not exist and local times that occur twice, and holidays and
  shortened sessions taking precedence over the normal weekly schedule.
- Identified, immutable dataset revisions, so the same query against the same revisions returns
  the same answer, and every answer names the revisions that produced it.
- A multi-venue view that answers all tracked venues at one instant, where one venue being
  outside coverage does not suppress the others.

The repository ships no venue data. All three launch venues forbid commercial redistribution of
their published schedules, so schedules are fetched at run time by the operator under the
operator's own relationship with each venue (see `DATA-LICENSING.md`). CI enforces this.

## Capabilities

### New Capabilities

- `session-query`: answering what phase a venue is in at an instant, including boundary
  ownership, explicit unknowns outside coverage, and the multi-venue view.
- `phase-vocabulary`: the shared, closed vocabulary of phases, and the phase-versus-event
  distinction that keeps funding settlements from being modelled as states.
- `answer-evidence`: provenance, published-versus-derived marking, and uncertainty carried on
  every answer.
- `venue-time-handling`: instant resolution, time scales, daylight saving, and the precedence of
  holidays and shortened sessions over the weekly schedule.
- `dataset-reproducibility`: identified dataset revisions, declared coverage, and deterministic
  answers.

### Modified Capabilities

None. This is the first change; `openspec/specs/` is empty.

## Impact

- `crates/market-time-core` — phase, event, evidence, uncertainty, coverage, rule, and timeline
  types. Pure: no I/O, no sockets, no clock.
- `crates/market-time-scales` — the hifitime bridge, where time-scale conversion lives.
- `crates/market-time-data` — dataset revisions and loaders. All I/O lives here.
- `crates/market-time-cli`, `crates/market-time-board` — thin shells over the core, holding no
  domain logic. The board reads `now` and passes it in, and surfaces the host clock's discipline
  bounds as uncertainty rather than presenting the host clock as exact.
- Governance: `CONSTITUTION.md` (ratified v1.2.0) binds this work and wins over habit.
- Reference material migrated from the spec-kit slice lives in `docs/venue-session-state/`.
