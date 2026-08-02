# QA

## Design and source evidence

- Input mode: user-supplied interaction reference, adapted rather than cloned. The reference is
  [Jin10 global trading hours](https://www.jin10.com/activities/global_trading_hours/index.html):
  only its shared UTC-ruler, current-cursor, and dense-row information hierarchy was adopted.
  No brand, copy, palette, venue data, or market schedule was copied.
- `DESIGN.md` foundation: `ready`, SHA-256
  `817248eab21a081cf4fe95ba3da628777278d8c349bccbb7bd6e21e9912663c9`.
- `MOTION.md` foundation: `ready`, static posture, no selected primitive, SHA-256
  `6b4aaae958b46b31f56eb6608ca132a437f83ead2b5a4f2a698a39f1e7ac0c93`.
- Design-pipeline dependency self-check passed. The missing optional Matt Pocock surfaces have no
  effect on this native HTML/CSS/JS change; no new dependency was added.

## Market-horizon verification — 2026-08-01

- `node --check frontend/assets/app.js`, `git diff --check`, and `cargo fmt --check` passed.
- Real HTTP smoke used the running `market-time-server` with `synthetic-venues.json` on
  `127.0.0.1:18123`. The normal inspection snapshot at `2026-07-30T14:00:00Z` rendered three
  known rows, a shared cursor, one server-window bar per row, a static map underlay, and the
  `Continuous trading` answer for Synthetic Daylight Exchange.
- The exact row bar is built only when the response includes both `boundary_start` and
  `boundary_end`; it is clipped to the displayed UTC day for presentation. The visible copy names
  it a current server window and explicitly rejects browser-derived schedules.
- Gate review caught and corrected two phase-colour selectors so `mid_day_break`, `post_close`,
  `non_trading_interruption`, and `closing_auction` colour the supplied bar only—not an entire
  venue row. A real `Mid-day break` inspection confirmed the exception-coloured bar with the row
  surface unchanged.
- A synthetic non-zero nanosecond boundary was explicitly rejected for rail placement and local
  time formatting; the original server strings remain visible with a precision warning instead of
  a false millisecond-precise chart.
- Historical inspection at `2026-10-01T02:00:00Z` preserved the calendar exception as
  `节假日例外 · Synthetic National Day`; inspection at `2030-01-01T00:00:00Z` rendered zero known
  rows and three explicit `unknown` rows.
- Injected current HTTP 503 after a successful inspection retained three known rows, froze the
  old UTC readout, labelled the mode `STALE`, and showed the frozen-snapshot recovery text.
- A programmatic retry after the 503 restored keyboard focus to the same evidence source link.
- At 390px with `prefers-reduced-motion: reduce`, the page had no horizontal overflow, the compact
  UTC strip was `position: sticky`, rails remained 309px wide, and controls reported `0s`
  transition duration.
- A browser route that aborted World Atlas left all three service rows intact and rendered the
  map-specific textual fallback. The horizon does not depend on visual CDN resources.
- Visual inspection used real Chrome screenshots for desktop, a normal in-session state, and the
  390px mobile state. The compact UTC band and ledger now own the scan path; the map is visibly a
  quiet orientation layer rather than a competing card.
- `cargo test --workspace` passed (118 tests); `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo build --workspace` passed.

## Global timetable verification — 2026-08-01

- `/v1/timeline` was test-driven at the HTTP boundary. It returns one tiled UTC day, server-owned
  segment positions, trading windows, the next server-derived open/close transition, calendar
  evidence, and explicit unknown coverage; `/audit` is served separately. The server HTTP suite
  now has nine passing tests, including a regression that distinguishes the next open from the
  clipped end of the displayed UTC day.
- Real browser QA on `127.0.0.1:18123` rendered three grouped venue rows on one 00–24 UTC ruler,
  one server segment per weekend row, local clocks, local trading-window summaries, one D3 map
  geometry, and no console errors or horizontal overflow at 1280px.
- The `2026-10-01` inspection displayed `Synthetic National Day` as a calendar exception while
  preserving known closed/trading semantics. The `2030-01-01` inspection displayed three unknown
  rows, three hatched unknown segments, `服务端未覆盖本日交易时段`, and no fabricated next boundary.
- At 390×844 the axis header collapsed, each venue row became a single column, navigation and date
  controls remained usable, and document width stayed within the viewport.
- The live clock and cursor advance from the server snapshot instant plus monotonic elapsed time,
  not from the browser wall clock. Timeline tracks expose one coherent `role=img` label to
  assistive technology, including explicit full-day and unknown descriptions.
- After the preview server was stopped, a new query kept all three last-known rows, switched to
  `STALE`, froze the server-anchored clock, and exposed a retry action. Restarting the server and
  retrying restored a normal inspection response and moved keyboard focus to the date input.
- `/audit` rendered the previous evidence/revision-focused screen with its own active navigation;
  `/` remained the direct timetable product surface.
- Fresh checks passed: `cargo fmt --check`, `node --check` for both browser scripts,
  `git diff --check`, `cargo test --workspace` (121 tests),
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace`.
- The adaptive Jin10 mapping adopts only the shared-dayline information structure. Exact visual
  fidelity is intentionally not claimed. Reference runtime computed-style capture timed out, while
  manual DOM/raster evidence and the design foundation gates passed.
- Design-pipeline feedback `dpf-8152f89a3063ef9f` records that the website-clone initializer added
  unsupported `surfaces` and `decisions` fields to an existing v2 state. The generated state was
  not edited manually; this tooling defect does not affect the product build or runtime.

## Gate scorecard

| Gate | Score | Evidence |
| --- | --- | --- |
| Visual hierarchy | 5/5 | Shared UTC horizon replaces sparse card grid; map is subordinate. |
| UX clarity | 5/5 | Live/inspection, stale retention, query, and current-window scope are explicit. |
| Accessibility | 4/5 | Semantic rows, visible focus, retry-focus retention, mobile and reduced-motion checks passed. |
| Responsive behavior | 5/5 | 390px screenshot and no-overflow measurement passed. |
| Motion quality | 5/5 | Data-only cursor, no animated phase bar, reduced-motion fallback checked. |
| Engineering fit | 5/5 | Native DOM/CSS and existing D3 map; no new dependency or browser rule logic. |

## Pixel timetable revision evidence (2026-08-01)

- `/v1/timeline` now returns a nullable `next_trading_window` assembled from contiguous core
  trading phases. Weekend HTTP smoke returned `2026-08-03T01:25:00Z`–`03:30:00Z` for
  `SYNTH-AUCT`; unknown coverage returned `null`.
- The homepage displays the closed-day range in the venue home zone, for example
  `本日休市｜下一交易段 08/03周一 09:25—11:30`; JavaScript only validates and formats it.
- Desktop 1354×1288 and mobile 390×844 real-browser checks both had zero horizontal overflow,
  three venue rows, no console warnings/errors, and the expected closed-day trading-time text.
- Fresh checks passed: 4 frontend tests, Biome lint, TypeScript, Next production build, npm audit,
  121 Rust tests, rustfmt, clippy with warnings denied, HTTP static/API smoke, and Code Intel normal.

## Market directory revision evidence (2026-08-01)

- The homepage now renders 47 directory rows: three service-connected synthetic venues and the
  requested 44-market target catalog grouped as equities, international spot/FX, and
  commodities/futures. Pending rows say `待接入`; they do not claim open or closed.
- Pinned Twemoji 14.0.2 SVG flags replace unsupported Windows flag emoji while preserving the text
  fallback. A direct CDN check returned HTTP 200 and the browser rendered 40 flag surfaces.
- Desktop browser QA at 1354px rendered a 288px directory beside a 947px timetable with no
  horizontal overflow. Mobile QA at 390px stacked the panels, limited the directory to a 272px
  internal scroll area, retained all 47 rows after data load, and had no horizontal overflow.
- Fresh frontend checks passed: six Vitest assertions, Biome lint, TypeScript typecheck, Next.js
  production build, and `git diff --check`.

## Unified timetable correction evidence (2026-08-01)

- The supplied reference was converted into a fresh implementation reference before coding. The
  result uses one continuous instrument, a primary city clock, and aligned identity/UTC/status
  columns; the old independently scrolling directory was removed.
- Real-browser desktop QA at 1440px rendered 44 catalog rows in three groups. The first identity
  and timeline track differed vertically by less than one pixel, document width matched the
  viewport, all 38 country-flag surfaces loaded, and no synthetic fixture name appeared.
- The current fixture matches zero real catalog IDs, so all 44 rows correctly render empty tracks
  and `待接入`; the summary says `0 个市场正在交易` and the footer says `0 / 44 已接入`.
- Tokyo selection changed the clock control, city label, and native UTC offset to `UTC+9`. At
  390px each row stacks identity, full-width track, and status without page-level or internal
  horizontal overflow. Browser console warnings/errors were empty.
- Fresh checks passed: six Vitest assertions, Biome lint, TypeScript typecheck, and Next.js
  production build.

## Remaining verification limits

- The local fixture contains three deliberately synthetic venues, so the real-market rows cannot
  yet demonstrate service-backed phase bars or status changes.
- External fonts and Twemoji flags remain CDN resources. Text and emoji fallbacks preserve meaning,
  but production CSP/CDN availability remains an operator concern.

## Editorial market-atlas revision evidence (2026-08-01)

- A fresh implementation reference was generated before coding. The coded result keeps its warm
  paper palette, oversized tabular clock, world-time orbit, shared UTC ruler, category rail, and
  aligned market rows while correcting the reference image's unsupported crypto `24×7` claims.
- The directory now has 52 unique entries in four families: 21 equities, 6 spot/FX, 17 futures,
  and 8 digital-asset venues. Digital rows explicitly say that maintenance, funding, and product
  rules require server evidence.
- Every row shows a local civil clock from the service-anchored instant and its declared IANA zone.
  Empty hatched tracks remain `未配置`; no browser-authored session or open/closed state is shown.
- Real browser QA at 1265px showed document width `1265 = 1265` and market table width
  `977 = 977`, with the status column visible. Digital filtering rendered eight rows; searching
  `东京` rendered exactly two rows in the equities and futures groups.
- Real HTTP smoke returned page HTTP 200 plus three API fixture venues and TZDB `2026c`. Fresh
  frontend checks passed: seven Vitest assertions, Biome with no warnings, TypeScript, and Next.js
  production build.
- Independent real-Chrome gate testing at 390px found and then verified the responsive layout;
  the category rail is width-contained and rows stack without page-level horizontal overflow.

## Market-relay hierarchy revision evidence (2026-08-01)

- The decorative world-time orbit was replaced with a market relay that ranks only connected,
  known service venues. A controlled timeline produced Shanghai `临近收盘`, Nasdaq `交易中`, then
  Tokyo `即将开盘`; an unknown venue stayed out of the relay and remained `UNKNOWN` in the ledger.
- With the current synthetic-only runtime dataset, no IDs match the 52-market product catalog.
  The relay therefore shows an honest four-family coverage matrix (`0/21`, `0/6`, `0/17`, `0/8`)
  instead of guessed sessions.
- Real-Chrome mobile QA at 390px measured document/body `390px`, relay `373/372px`, and market
  table `374/374px`, with no page or table overflow. Relay and coverage text computed to 12px;
  the darker trading green reached 4.95:1 contrast on the rendered surface.
- Fresh checks passed: eight Vitest assertions, Biome lint, TypeScript typecheck, Next.js production
  build, `git diff --check`, and HTTP smoke (`200`, three fixture venues, TZDB `2026c`).

## Market-horizon map revision evidence (2026-08-01)

- The relay now keeps the world map visible in both covered and empty states. Its eight hub nodes
  report `trading`, `idle`, `unknown`, or `pending` from service timelines only; a ninth Vitest
  assertion locks those four outcomes.
- A Git-ignored local operator preview was validated by `market-time-cli` and served two
  source-reviewed, narrowly covered venues (`XSHG`, `XNYS`) through the real HTTP stack. The page
  rendered two connected rows and exposed revision `operator-preview-2026-08-01` plus tzdb `2026c`.
- In-app browser inspection at 1280px measured document width 1265px in a 1280px viewport,
  retained all 21 equity rows, and confirmed a map-node click switches the primary clock to New
  York.
- Fresh checks passed: nine Vitest assertions, Biome lint/format, TypeScript typecheck, Next.js
  production build, Rust dataset load, and page/timeline/health HTTP smoke.

## Agent-first source intelligence evidence (2026-08-02)

- The source workspace reads the real `/v1/status` evidence and rendered three unique publisher
  URLs for `XSHG` and `XNYS`, including publisher domain, venue coverage, fetched time, effective
  date, revision, and tzdb version. It does not claim that the browser imported or approved data.
- Repeated evidence URLs are condensed by a tested pure projection; strict runtime validation
  rejects malformed timestamps, unsupported status shapes, and non-HTTP(S) source URLs.
- Existing Appica/Base UI tabs, badges, and progress were reused. No package or second component
  primitive stack was added.
- Real-browser QA at 1280px measured document width 1265px, found no horizontal overflow, and
  exercised both the runtime-source and four-step intake-protocol tabs. The agent contract exposes
  the real `/v1/timeline` and `/v1/status` read paths and names `UNKNOWN` as distinct from closed.
- Fresh frontend checks passed: ten Vitest assertions, Biome format/lint, TypeScript typecheck, and
  Next.js production build.

## Settings and theme correction evidence (2026-08-02)

- The homepage no longer contains `#source-intelligence`; its primary scan now goes directly from
  the shared UTC ruler to the market directory. The header links to the separate `/settings` route.
- The Rust production shell serves `/settings` as HTML while retaining JSON 404 behavior for
  unknown routes. All nine server HTTP tests passed, including the new route regression.
- Real-browser QA loaded `/settings` through `127.0.0.1:18123`, rendered three live evidence
  sources, selected the four-step intake protocol, and reported no console messages or horizontal
  overflow at 1280px.
- Dark is the initial theme. Switching to light changed the computed page background to
  `rgb(238, 241, 244)` and survived a reload; the final browser state was returned to dark.

## Market relay proportion correction evidence (2026-08-02)

- Code Intelligence normal run `20260802-010113-844-core` completed with a green hospital report,
  `clean snapshot`, and `observe` disposition.
- Before the correction, the 1225px hero split into a 422px clock, 751px relay, 431px map, and a
  319px market queue. Two real market readings were visibly compressed inside the nested rail.
- After the correction, the clock and relay each span 1225px; the relay gives 703px to the map and
  520px to the queue. Each current market reading is 519px wide, with no clipped label or detail.
- The relay header now exposes only server-derived counts (`交易中`, `已知休市`, `Unknown`, and
  connected rules). Real-browser inspection found no page overflow at a 1280px viewport.

## Settings consolidation evidence (2026-08-02)

- The timetable header now has two product destinations: the timetable and `时间字典与设置`.
  Theme controls and the separate audit entry are absent from the scan path.
- `/settings` consolidates primary time-zone selection, boundary reminders, appearance, live
  evidence, revision, and controlled-intake information. The three Appica preference controls use
  a responsive card grid without horizontal overflow.
- The production HTTP shell returns `200` for `/`, `/settings`, `/health`, and `/v1/timeline`;
  legacy `/audit` returns `308` to `/settings#source-intelligence`. The duplicate static audit
  frontend and asset route were removed.
- Real-browser QA rendered 21 connected markets out of 52, switched from dark to light, verified
  that the choice survived reload, then restored dark. Both themes had no horizontal overflow.
- Fresh gates passed: 12 Vitest assertions, Biome format/lint, TypeScript typecheck, Next.js
  production build, 121 Rust tests, rustfmt, clippy with warnings denied, zero high-severity npm
  advisories, no gitleaks findings, and Code Intelligence normal run
  `20260802-113145-152-core` completed successfully.
