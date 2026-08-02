# Handoff

<!-- DESIGN-PIPELINE:DESIGN-SYNTHESIS:START -->
## Market-horizon frontend design

- Historical synthesis manifest: `openspec/changes/market-time-clockboard-design/design-synthesis.json`
- Historical manifest stage: `design-validation` / `needs-review`; it records the initial
  requirements-only foundation before the user-approved visual-resource revision.
- Current pipeline stage: `verification` / `verifying`, recorded in `state.json` and `events.jsonl`.
- Product design output: `DESIGN.md`
- Project motion foundation: `MOTION.md`
- Change design: `openspec/changes/market-time-clockboard-design/design.md`
- Input mode: user-supplied interaction reference, with external typography and a static D3 vector
  map; no reference branding, copy, palette, market data, or schedule is adopted.
- Scope: 22/24 (fit); no Wayfinder map is required.
- Design foundations: `DESIGN.md` and `MOTION.md` both validate as `ready`.
- Design direction: global timetable — `/` groups every venue on one 00–24 UTC dayline using the
  server-supplied full-day timeline, local clocks, local trading windows, current phase, next
  server-derived open/close transition, and calendar exceptions. The previous evidence-heavy
  surface now lives at `/audit`.
- Verification: desktop/mobile, holiday, coverage-unknown, stale recovery, navigation, HTTP,
  workspace tests, clippy, formatting, syntax, and build checks pass. Exact reference cloning is
  not claimed; Jin10 contributed only the shared-dayline information structure.
- Tooling note: local design-pipeline feedback `dpf-8152f89a3063ef9f` records an initializer/schema
  mismatch. The product runtime is unaffected and generated pipeline state was not hand-edited.
- Next: retain the visible local preview and verification evidence. Do not commit, push, release,
  or deploy until separately authorized.

The v2 pipeline state and event chain are consistent. Product requirements and the recorded
contract review determine the design; no external template is authoritative. The historical
synthesis manifest is retained as evidence, not used as the current gate-status source.
<!-- DESIGN-PIPELINE:DESIGN-SYNTHESIS:END -->

<!-- design-pipeline:website-cloning -->

## Website Cloning

- Manifest: `openspec/changes/market-time-clockboard-design/website-cloning.json`
- Targets: `127-0-0-1` (primary), `www-jin10-com-activities-global-trading-hours-index-html` (reference)
- Next: verify authorization and Browser/Builder/Evidence port capabilities, then capture `127-0-0-1`.
- Added: 2026-08-01T10:59:15.378Z
