# Change Design

Selected direction: **Global timetable**. Reusable product rules live in
[`DESIGN.md`](../../../DESIGN.md); the contract review is
[`decisions/grill.md`](decisions/grill.md).

## Requirements Map

| Server contract | Frontend responsibility | Forbidden interpretation |
| --- | --- | --- |
| `at` / `clock` | Name the UTC reference and clock discipline for this snapshot. | Treat the host clock as exact or mix snapshots. |
| `home_zone`, `location` | Format the same `at` as a local clock, label the city/zone, and place supported city labels on a static map. | Use local time to infer a phase or invent geography for an unplaced venue. |
| `status: known`, `phase` | Render the supplied phase as the service verdict. | Translate `unknown` to `closed` or infer “open”. |
| `boundary_start`, `boundary_end`, and `uncertainty` | Plot the exact current server window on the displayed UTC day and name both supplied qualifications. | Generate a recurring schedule, infer a session, or promise a next open/close beyond the supplied boundary. |
| `calendar` / `events` | Show holiday, shortened session, or announced change in the scan path. | Reduce an exception to a generic warning colour. |
| `evidence`, `derived_reasoning` | Provide an accessible disclosure titled “为什么这样判断”. | Present derived reasoning as a first-party observation. |
| `dataset_revisions`, `tzdb_version` | Show the snapshot revision strip and per-venue revisions. | Hide the revision that produced a claim. |
| `status: unknown`, `reason`, `coverage` | Put the venue in a data-boundary lane with its stated reason. | Render as an error, empty card, or known market state. |
| `/v1/timeline` interval, positions, segments | Draw the full UTC day exactly as supplied and list server-aggregated local trading windows. | Infer recurring sessions or classify phase names in JavaScript. |

## Spatial Contract

```text
desktop
┌ master clock / UTC date / current counts / quiet world silhouette ┐
├ 00 ── 03 ── 06 ── 09 ── 12 ── 15 ── 18 ── 21 ── 24 UTC ──────────┤
│ family heading                                                     │
│ venue / local clock │ complete core timeline │ current / boundary │
│                     │ local trading windows   │ calendar exception │
└────────────────────────────────────────────────────────────────────┘
nav: 全球交易时间表 / 时间字典与设置
```

At 640px, each venue stacks identity, full-width timeline, then status. Axis labels hide rather
than forcing page-level horizontal scroll; the local trading-window text remains. Historical
instant inspection stays on the timetable rulers; preferences, appearance, evidence, and revisions
are consolidated at `/settings`. `/audit` permanently redirects to that workspace.

## Component States

- **Live:** UTC and local clocks tick; a new timeline snapshot is requested at the earliest
  supplied segment end or after one minute. A live badge names the mode in text.
- **Inspection:** the selected UTC date is queried at 12:00Z; local displays are fixed; “回到今天”
  is present and deterministic.
- **Loading:** keep the prior snapshot visible where available and announce “正在读取此 UTC 时刻的
  服务端规则”; first load uses reading-shaped skeleton rows.
- **HTTP failure:** preserve any last successful snapshot, mark it stale, and give a retry action.
- **Unknown:** use the dedicated lane, supplied reason, coverage, and revision—not the loading or
  failure state.

## Accessibility and Asset Strategy

Use semantic `header`, `nav`, `section`, `article`, `time`, `form`, and `button`. Keyboard focus
follows navigation → date controls → grouped venue rows. Preserve visible focus, 4.5:1 text
contrast, concise snapshot announcements, and status text independent of colour. Every segment
has a native text label; unknown is both hatched and named. The D3 world layer is supplementary,
and font/map failures leave the complete semantic timetable intact.

## Evidence Adoption Matrix

| Source | Decision | Target |
| --- | --- | --- |
| Mark Time Constitution 1.2.1 | Adopt explicit evidence, uncertainty, revisions, and unknown as first-class UI concepts. | reference strip, known reading, unknown lane, disclosure |
| Existing server API | Adopt its snapshot as the sole state authority. | `app.js` rendering boundaries |
| User direction | Adopt “express time” as the page's central job; exclude execution and factor workflows. | market-horizon hierarchy |
| Existing frontend MVP | Retain native browser capability and the local-clock card affordance. | HTML/CSS/JS implementation |
| User reference revision | Adopt a dense shared UTC ruler and current cursor from the supplied Jin10 page; keep a static city map and external typography as orientation aids only. | server-window horizon with textual fallback |
| Generic dashboards, state heatmaps, terminal themes | Reject. They would make counts, regions, or execution feel like the product. | deliberately absent |
| Appica UI + Next.js 16 | Adopt Appica controls inside a statically exported App Router shell; Rust remains the single HTTP/API process. | homepage controls and status badges |

## Global dayline mapping

Mapping id `global-dayline` is **supporting**, from reference target
`www-jin10-com-activities-global-trading-hours-index-html` region “master clock and grouped
24-hour schedule rows” to primary target `127-0-0-1` region “global timetable”. Adopted:
large reference clock, shared 00–24 ruler, category headings, compact venue rows, phase bars,
current cursor, and right-side status. Rejected: Jin10 brand, copy, theme-switch UI, exact palette
values as brand identity, venue list, trading hours, market data, and any aggregator-derived fact.
Required states: desktop timetable, 390px timetable, holiday exception, explicit unknown, and
reference-resource failure.

## Frontend runtime decision

The corrected timetable is implemented as a Next.js 16 static export in `web/`. Appica UI supplies
the reusable controls and status badges; the project-specific 24-hour horizon remains a semantic
React component because no generic table component represents server-supplied interval geometry.
The Rust shell serves `web/out/index.html` and `web/out/_next`, preserving one origin and the stable
API. There is no Next.js server, proxy, route handler, or duplicated session calculation.

## Agent-first source intelligence revision

- `/v1/status` is the only runtime source-evidence input; `/v1/timeline` remains the schedule
  authority.
- `/settings` owns data-source administration so the homepage remains a pure market-time scan.
  The UI may deduplicate exact source URLs for scanning, but labels the result as a summary and
  retains the original source link, affected venues, fetch time, and effective date.
- Agent discovery and extraction produce candidate rules only. Terms registration, golden-vector
  verification, and human approval gate publication of an immutable revision.
- Retain Appica/Base UI. Do not add a duplicate shadcn primitive stack or chat-oriented AI Elements
  and assistant-ui surface until a genuine conversational operator workflow exists.
- Default to the dark market-horizon theme, offer an explicit light alternative only in settings,
  and restore the saved choice before React hydration.
- Constitution check: passes evidence, uncertainty, revision, licensing, and separation-of-concerns
  requirements; no data is vendored, no browser rule logic is added, and no mutation is implied.
