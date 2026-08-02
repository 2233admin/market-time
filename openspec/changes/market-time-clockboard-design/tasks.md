# Tasks

## Design foundation

- [x] Record contract and constitution review in `decisions/grill.md`.
- [x] Assess the design scope: 22/24 session budget; no Wayfinder map required.
- [x] Produce three directions and select the dayline instrument.
- [x] Write the reusable `DESIGN.md` and `MOTION.md` foundations.
- [x] Map stable API fields to visual components and prohibited interpretations.

## Initial implementation (historical)

- [x] Restructure the static page around the reference instrument and dayline.
- [x] Add a failure-tolerant D3 world orientation map and 24-hour UTC chart without moving rule logic into the browser.
- [x] Render known readings and unknown data boundaries as separate semantic components.
- [x] Make inspection, loading, stale, and HTTP-failure states conform to the design contract.
- [x] Add targeted browser checks for known, holiday, unknown, inspection, and keyboard paths.
- [x] Run visual, responsive, accessibility, and implementation QA; update `qa.md`.

## Market-horizon revision

- [x] Replace oversized reference/card composition with a compact reference band and full-width
  shared UTC horizon.
- [x] Plot only each known venue's server-supplied current phase interval; preserve phase text,
  boundary uncertainty, evidence, and explicit unknown semantics.
- [x] Make the world map a non-state visual underlay and keep the ledger usable if it fails.
- [x] Recheck normal, holiday, unknown, stale, mobile, reduced-motion, focus, and resource-failure
  states in a real browser.

## Global-timetable correction

- [x] Add a tested `/v1/timeline` HTTP projection over the existing core timeline.
- [x] Move the current evidence-heavy screen to `/audit` without changing its decision logic.
- [x] Rebuild `/` as a grouped 00–24 UTC timetable with server segments and local window text.
- [x] Record the Jin10 structure as an adaptive reference mapping without copying its data or brand.
- [x] Run desktop/mobile, holiday, unknown, stale, navigation, accessibility, and resource-failure QA.

## Next.js and Appica revision

- [x] Replace the handwritten homepage runtime with a Next.js 16 App Router static export.
- [x] Use Appica UI for interactive controls and semantic state badges.
- [x] Keep the Rust service as the single origin and `/v1/timeline` as the only schedule authority.
- [x] Re-run desktop/mobile, inspection, unknown, stale, navigation, and accessibility browser QA.

## Pixel timetable revision

- [x] Replace the neutral palette with the square deep-indigo pixel-console system while retaining
  Appica controls and text labels for every state.
- [x] Return the next evidence-backed trading window from `/v1/timeline` so closed days can name
  the next local trading date and range without browser-side schedule inference.
- [x] Re-run targeted tests, production builds, HTTP smoke, and desktop/mobile browser QA.

## Market directory revision

- [x] Add the requested 44-market directory with grouped flags, identity, and location metadata.
- [x] Keep the three service-backed fixtures separate from pending catalog entries; never render a
  pending market as open or closed.
- [x] Verify pinned flag assets, desktop/mobile no-overflow layout, tests, lint, typecheck, and build.

## Unified timetable correction

- [x] Merge market identity, UTC track, and status into one aligned row system matching the latest
  supplied reference.
- [x] Remove synthetic fixture rows from the product surface while keeping them available through
  the separate audit/data-verification path.
- [x] Keep all 44 requested markets visible with empty `待接入` tracks until matching server venue
  IDs provide real segments.
- [x] Verify city switching, desktop alignment, mobile contained scrolling, console output, tests,
  lint, typecheck, and production build.

## Editorial market-atlas revision

- [x] Replace the reference-like dark table with a generated, original editorial-finance visual
  direction using an open clock/orbit/UTC-ruler composition.
- [x] Expand the directory to 52 entries across four families, including eight digital-asset
  venues, without adding venue schedule claims.
- [x] Show every catalog row's local civil clock from the same service-anchored instant while
  keeping trading sessions and states server-only.
- [x] Add Appica search, family filtering, explicit connected/unconfigured coverage, and a tested
  catalog filter.
- [x] Verify production build, TypeScript, Biome, Vitest, HTTP data loading, desktop layout,
  digital-asset filtering, search, and no-overflow behavior.

## Market-relay hierarchy revision

- [x] Replace the decorative world-time orbit with a service-ranked now/next market relay.
- [x] Promote trading, closing-soon, and opening-soon venues without browser-authored schedules.
- [x] Keep missing rule coverage visible and actionable instead of mixing it with market state.
- [x] Add a regression check for attention ordering and re-run responsive browser QA.

## Market-horizon map revision

- [x] Replace the waiting-state matrix as the dominant visual with an original world-market map.
- [x] Drive hub states only from connected service venues and keep pending, unknown, idle, and
      trading visually distinct.
- [x] Open the directory on equities and demote digital assets without deleting their coverage.
- [x] Surface the active ruleset and tzdb revision next to the coverage meter.

## Agent-first source intelligence revision

- [x] Validate `/v1/status` evidence before presentation and condense repeated source URLs with a
  regression test.
- [x] Add progressive-disclosure runtime-source and intake-protocol views using existing
  Appica/Base UI.
- [x] Keep automated discovery and extraction separate from terms, golden-vector, and human
  publication gates.
- [x] Re-run frontend checks, production build, HTTP smoke, and real-browser QA.

## Settings and theme correction

- [x] Remove source administration from the global-timetable scan path and serve it at `/settings`.
- [x] Make dark mode the default and add a persistent light-mode alternative using existing
  Appica controls.
- [x] Add an HTTP regression for the production Rust shell's `/settings` static route.
- [x] Verify both themes, settings tabs, route errors, build output, and browser console.

## Market relay proportion correction

- [x] Replace the sparse asymmetric hero with a full-width clock command strip and relay.
- [x] Give the map and market queue independent readable widths; remove the nested 319px queue.
- [x] Add service-derived trading, closed, unknown, and coverage summaries without new rule logic.
- [x] Verify geometry, text clipping, horizontal overflow, frontend checks, and production build.

## Settings consolidation

- [x] Consolidate time preferences, appearance, evidence, and revisions under `/settings`.
- [x] Remove the theme control and separate audit entry from the timetable header.
- [x] Redirect the legacy `/audit` route and remove the duplicate static audit frontend.
- [x] Re-run frontend, Rust, HTTP, and repository quality gates before publication.
