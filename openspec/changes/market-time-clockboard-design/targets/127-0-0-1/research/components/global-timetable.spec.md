# GlobalTimetable Contract

## Ownership

- Primary target: `127-0-0-1`
- Reference mapping: `global-dayline`
- Target files: `frontend/index.html`, `frontend/assets/app.css`, `frontend/assets/app.js`
- Evidence root: `targets/127-0-0-1/research`
- Fidelity mode: adaptive

## Environment

- Primary URL: `http://127.0.0.1:18123/`
- Viewports: 1440×900, 768×1024, 390×844; device scale 1
- Locale / scheme: zh-CN / dark
- Page ready: `/v1/timeline` succeeds and `#schedule-groups[aria-busy=false]`
- Dynamic region: clocks and current cursor only

## Interaction Model

- Driver: time + native date input
- Keyboard: navigation and every control use normal DOM order with visible focus
- States: first-load, live, inspection, stale, error, holiday, unknown, resource failure
- Repeated input: last request wins through a request sequence

## Structure

- Landmarks: main, header, nav, master section, timetable section, footer
- Layout: one shared ruler; grouped semantic articles; no canvas-owned content
- World SVG is an aria-hidden background below semantic content

## Palette Foundation

- Palette evidence: `../palette-evidence.json`
- Roles: ground, card, primary text, time needle
- Target tokens: `--mt-bg`, `--mt-card`, `--mt-text`, `--mt-needle`
- Intentional adaptation: Mark Time adds semantic phase green/amber/blue/orange and hatched unknown

## Content And Assets

- Text is authored for Mark Time; venue labels and schedules come only from the local API.
- External assets: Google fonts; D3, TopoJSON, World Atlas. No reference asset is copied.

## Responsive Contract

- 1440: three-column row with 184px label and 132px status.
- 768: narrower fixed columns; same shared axis.
- 390: venue row stacks; full timeline remains visible; dense axis header hides.

## Target-Project Mapping

- Reuse: existing native HTML/CSS/JS, fonts, D3 world layer, semantic tokens.
- New API: `/v1/timeline`; no frontend schedule logic or new package.
- Reject: reference brand, copy, data, theme toggle, lower market-data sections.

## Builder Contract

- Allowed files: server route/test plus listed frontend and design artifacts.
- Required checks: Rust server tests, workspace test/clippy/build/fmt, JS syntax, HTTP and browser QA.
- `SPEC_INCOMPLETE`: server does not supply full tiling segments, positions, unknown, or local windows.
- Completion: homepage directly lists all loaded venues across one UTC day; `/audit` retains details.

## Evidence Gate

- Palette foundation: required before browser QA.
- Text/interaction coverage: all authored product states, not reference copy.
- Pixel/layout thresholds: not configured in adaptive mode.
- Accessibility: semantic rows, text state, visible focus, responsive no-overflow.
- Verdict: pending implementation QA.
