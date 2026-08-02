# Directions

**Design read:** a high-trust operational reader for people checking global market time under
pressure, with an instrument-like language, high information density, and static motion.

## A — Terminal clockboard

- **Thesis:** retain the current dark terminal grid, large local numerals, and equal-height market
  cards; improve labels and evidence hierarchy.
- **Fit:** low implementation cost and familiar clockboard vocabulary.
- **Risk:** equal cards make the page a collection of regional clocks. UTC remains a header fact,
  not the organising idea; evidence and unknown status remain visually secondary.

## B — Dayline instrument

- **Thesis:** centre the page on one horizontal UTC ruler and current-instant cursor, paired with
  a low-detail world map that anchors supported venue cities. Venue local clocks appear as reading
  stations aligned to that reference; server verdicts are nearby labels, not colour encoded marks
  on a fake session timeline.
- **Fit:** makes the product's one-instant/many-local-times model visible before a reader opens a
  card. The static map gives geographic intuition without becoming a state heatmap; a D3 SVG
  layer supplies both visuals without turning the shell into a client application framework.
- **Risk:** a time ruler can be mistaken for a trading schedule. The component must carry the
  permanent caption “当地民用时间参考，不推导交易时段” and never draw phase durations on it.

## C — City ledger

- **Thesis:** a dense, tabular city ledger with timezone columns, phase text, next boundary, and
  evidence indicators; the inspection control behaves like a query console.
- **Fit:** strongest comparison scan for a large future venue list and excellent for keyboard use.
- **Risk:** reads as an internal database and loses the spatial/time intuition that makes the
  human board worth having alongside the API.

## D — Market horizon (superseded)

- **Thesis:** use one current UTC cursor across a dense, full-width row ledger. A faint world map
  is an underlay, not a sibling panel; each row plots only the exact current phase interval returned
  by the server. The clock is compact so the horizon owns the first view.
- **Fit:** exposes the product's true comparison task—one instant, many venue answers—at a glance.
  It preserves the evidence and unknown boundaries while giving the page the direct, data-rich
  reading rhythm the user asked for.
- **Risk:** a wide bar could look like an inferred daily session. Permanent copy calls it a
  “current server window,” and implementation may use only returned `boundary_start` and
  `boundary_end`, clipped to the displayed UTC day.

## E — Global timetable (selected)

- **Thesis:** put the complete 24-hour core timeline on the homepage. One shared ruler, family
  groups, one dense row per venue, explicit local trading-window text, and a compact right-hand
  status column make the product read as a timetable rather than an audit console.
- **Fit:** directly answers the user's primary question: when does each exchange trade today?
  The former market-horizon audit remains intact at `/audit` for evidence and diagnosis.
- **Risk:** dense segments can lose meaning on small screens. Textual local windows and current
  phase remain visible after axis labels compress; unknown uses both hatch and text.

## Selection and source posture

Select E. It adaptively maps the user-supplied Jin10 reference's master clock, shared ruler,
grouped rows, phase bars, current cursor, and status column onto Mark Time's API. It rejects the
reference brand, text, source data, schedules, and non-auditable certainty. D remains useful as the
separate `/audit` page, not the product homepage.
