## Why

The launch slice tracks three venues, chosen to exercise three structurally different market
models. A global trading-hours board that people actually reach for covers roughly forty markets
across three families — cash equities, spot and FX, and futures — plus the FX session bands and
their overlaps. Conventional boards already do that; what none of them do is carry evidence and
uncertainty through to the surface, which is the whole point of Mark Time.

This change captures what "global coverage" requires beyond the launch slice, so the shape is
decided before venue number four arrives. It is deliberately a separate change: the launch set
stays at three venues, and nothing here justifies widening that set.

Conventional boards are a reference for *what to display*, never a source for *what is true*.
Nothing in this change is sourced from one. Every added venue needs its own first-party
published schedule and its own terms check at registration, exactly as the launch three did.

## What Changes

- A venue catalog: venues carry the metadata a board needs (display name, home city, home zone,
  asset-class family) and are grouped for display, so adding a venue is a data change plus a
  source registration — never a change to the phase model.
- Session segment roles: venues whose trading day is split into a main block and a night block
  (Chinese commodity futures, several CME products) are expressible without inventing venue
  specific phase names. The role rides alongside the phase; it does not become one.
- Derived session bands: the FX day is conventionally shown as four regional bands with two
  overlap windows. These are derived from venue schedules rather than published as such, so they
  are marked derived and carry their reasoning, per Principle I.

## Capabilities

### New Capabilities

- `venue-catalog`: venue identity, display metadata, asset-class grouping, and the rule that
  adding a venue is data plus a source registration rather than code.
- `session-segments`: the main-versus-night block distinction, carried as a role on a segment
  rather than as a new phase name.
- `session-overlap`: derived regional session bands and their overlap windows, marked derived.

### Modified Capabilities

None yet. `openspec/specs/` is written when `venue-session-state` is archived; the deltas here
are additive and do not change any requirement that change introduces.

## Impact

- `market-time-core` — a segment role attribute, and the derivation that produces session bands.
  The phase vocabulary stays closed and unchanged.
- `market-time-data` — catalog records per venue, and a coverage declaration per venue rather
  than per launch set.
- `market-time-board` — grouped rows, and a bands view beneath the venue rows.
- Each new venue carries its own `DATA-LICENSING.md` terms check. No venue dataset is committed,
  now or ever.

## Explicitly not in scope

- **Turnover and volume panels.** Conventional boards often close with average daily turnover by
  asset class. That is market data, not schedule data, and it comes from aggregators whose terms
  forbid reuse. Mark Time does not carry prices, quotes, trades, or volumes.
- **Any figure taken from an aggregator board**, including venue lists treated as authoritative.
  A board may tell us a market is worth displaying; only that market's own publication tells us
  when it trades.
