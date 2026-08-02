# Product and contract review

## Evidence consulted

- `AGENTS.md`: the board is a thin shell; an unknown is a value, not a visual variant of
  `closed`; humans are a secondary but valid board audience.
- `CONSTITUTION.md` 1.2.1: evidence, uncertainty, immutable revisions, and no shell-domain
  logic are binding constraints.
- Existing `frontend/index.html`, `frontend/assets/app.js`, and
  `crates/market-time-server/src/lib.rs`: the browser already consumes one `/v1/status?at=`
  snapshot and only formats local civil time with `Intl`.
- User direction, 2026-08-01: this surface's purpose is to express time; factor mining and
  controlled live trading are not its primary workflow.

## Resolved design decisions

1. **Job to be done.** A person looking at the board must be able to answer: “At this one
   UTC instant, what does each venue's published rule say, until when, and why should I trust
   it?” The API remains the programmatic surface for autonomous agents.
2. **Product boundary.** This is a global time-and-rule reader, not an execution terminal,
   strategy workspace, price chart, or venue-calendar editor.
3. **Truth boundary.** The browser may format `at` in `home_zone` and tick a local display. It
   must not infer, interpolate, sort by, or label a session phase. `status`, `phase`,
   `boundary_end`, `calendar`, `events`, `uncertainty`, `evidence`, and `dataset_revisions`
   retain their service meaning verbatim.
4. **Unknown boundary.** `unknown` is a first-class response lane with its supplied reason,
   coverage, and revisions. It is never put in a “closed” bucket or shown as an error state.
5. **Visual thesis.** Use a single market horizon: a UTC ruler with a current-instant cursor that
   gives every venue row a common reference. A row bar may only plot the service-provided current
   `boundary_start` / `boundary_end`, and its caption states that it is not a browser-derived
   schedule. Market states remain service-sourced text.
6. **Evidence posture.** Phase, end boundary, calendar exception, and clock discipline are
   visible in the scan path. Sources, derived reasoning, uncertainty detail, and exact
   revisions are available through an accessible disclosure without being hidden behind a
   generic settings screen.
7. **Motion posture.** The board is information-dense and repeated-use. Its only movement is
   the already-necessary second tick and short state replacement feedback; no ambient or
   decorative animation is needed.

## Deferred decisions

- Venue expansion, real operator-owned datasets, authentication, execution, strategy and
  backtest workflows remain outside this design change.
- No external reference site, visual template, or third-party brand is adopted. The direction
  is requirements-only and authored for Mark Time.

## User-approved visual revision

The user requested a map, chart, framework, and external typography. The implementation therefore
uses D3 only for a static SVG city-orientation underlay, while native CSS renders the service
window horizon and the native page owns semantic data rendering and failure handling. The map may
show supported city anchors but is not a state map; no visual position or color calculates,
predicts, or replaces a server verdict. Dynamic imports and textual fallbacks keep the board
readable when a visual resource is unavailable.

## User reference revision

The user supplied Jin10's global trading-hours page as a direction correction: the previous page
was semantically sound but visually too much like a sparse internal tool. Adopt its interaction
lesson—one UTC ruler, one current cursor, dense market rows—as a hierarchy decision only. The
board will plot a row bar only when the API supplies both `boundary_start` and `boundary_end` for
the current phase, and labels it as a current server window. It will not copy the site's brand,
copy, colours, venue list, or schedules, and it will not calculate a recurring market calendar.

## Constitution check — before design

| Principle | Design consequence |
| --- | --- |
| I — Evidence | Source fields and derived status have an explicit place in the UI. |
| II — Instants / uncertainty | UTC is named; clock discipline and unknown are never softened. |
| III — Revisions | Snapshot and per-venue revisions remain inspectable. |
| IV — Thin shell | No trading-time calculation moves to JavaScript. |
| V — Golden vectors | This design does not change rule behaviour; implementation must cover visual rendering of known, holiday, and unknown responses. |
