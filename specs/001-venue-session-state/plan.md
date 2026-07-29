# Implementation Plan: Venue Session State

**Branch**: `001-venue-session-state` | **Date**: 2026-07-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-venue-session-state/spec.md`

## Summary

Answer "what phase is venue X in at instant T" for SSE, NYSE, and Binance USD-M perpetual
futures, with source evidence and honest uncertainty, and surface those answers on a global
trading-hours board.

Technical approach: a pure Rust core library holding the phase model and the resolution logic,
with no I/O and no clock reads. `jiff` provides the civil/DST layer; `hifitime` provides
leap-second-aware conversion for non-UTC input at the ingest boundary. Rule data lives outside
the code as immutable, versioned dataset revisions carrying per-record evidence. The board and
the CLI are thin shells that pass `now` in and render what the core returns — including
unknowns.

## Technical Context

**Language/Version**: Rust (2024 edition; MSRV pinned at first commit)

**Primary Dependencies**: `jiff` (civil time, IANA zones, DST ambiguity — built with
`tzdb-bundle-always`), `hifitime` (time scales, IERS leap-second table). Serialization and
board rendering deliberately not pinned at this stage.

**Storage**: versioned dataset revisions as files, vendored or fetched at run time depending on
each source's redistribution terms (research D6). No database in this slice — rule data is
thousands of records.

**Testing**: `cargo test`, with a golden-vector corpus as the primary correctness instrument
(Principle V).

**Target Platform**: cross-platform library; board renders in a browser.

**Project Type**: library-first, with thin CLI and board shells.

**Performance Goals**: not a differentiator for this slice. A phase query is a lookup against a
small in-memory ruleset. No throughput target is set, and none should be invented.

**Constraints**: core does no I/O, opens no sockets, reads no clock. Every answer carries
evidence, uncertainty, and the dataset revisions that produced it. Coverage is declared;
queries outside it return unknown.

**Scale/Scope**: 3 venues, ~8 phase kinds, thousands of rule records, one board.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Gates derived from constitution v1.2.0.

| Gate | Requirement | Phase 0 status |
|---|---|---|
| **G1 — Evidence (I)** | Every rule carries `source_url`, `fetched_at`, `effective_from`; derived is marked, never presented as observed | PASS — FR-009/FR-010; research D6 identified every first-party source and its terms |
| **G2 — Instants (II)** | ns representation, declared time scale, leap-second-aware conversion, no hardcoded offsets | PASS — D1 selects `hifitime` for the scale layer precisely because it is the only surveyed crate with a real IERS table |
| **G3 — Uncertainty (II)** | Answers carry uncertainty; precision never presented as accuracy; outside coverage returns unknown | PASS — FR-011/011a/011b; D5a supplies a venue-published bound (±15 s), D3 a process-not-instant boundary |
| **G4 — Reproducibility (III)** | Immutable dataset revisions; builds report revisions; same revisions + query = same answer | PASS **with required mitigation** — see G4 note |
| **G5 — Library-first (IV)** | Core has no I/O, no network, no clock reads; shells hold no domain logic | PASS — enforced structurally, see Structure Decision |
| **G6 — Golden vectors (V)** | DST both directions, holidays, shortened sessions, boundaries, out-of-coverage; every defect becomes a permanent vector | PASS — vector set enumerated in spec Edge Cases; D3/D4 corrected two of them before any code exists |
| **G7 — Shared vocabulary** | One phase vocabulary across all venues; no venue-specific phase names | PASS — FR-005/006; SSE after-hours fixed price maps to `post_close`, Binance funding is an event not a phase |

**G4 note — required mitigation, not a silent pass.** `jiff` exposes no runtime tzdata version
string (research D2), and its default behaviour on Unix reads the unpinned OS zoneinfo, which
would break reproducibility silently. The build MUST enable `tzdb-bundle-always` and record the
IANA release identifier in dataset-revision metadata as our own recorded fact. Without that
mitigation G4 fails; with it, G4 passes. This is a build-configuration requirement, not a
preference, and belongs in the first task.

### Post-design re-check (after Phase 1)

Re-evaluated 2026-07-29 against `data-model.md`, `contracts/`, and `quickstart.md`.

| Gate | Post-design status |
|---|---|
| G1 — Evidence | **PASS** — `Evidence` is 1:1 on `Rule` and mandatory; `is_derived` / `derivation_reasoning` are paired-validated so a derived rule cannot ship unexplained |
| G2 — Instants | **PASS** — `RawScaledInstant` (scale-tagged, ingest only) is a distinct type from the core's UTC-only `Instant`; civil↔absolute conversion is one explicit operation returning Unambiguous / Ambiguous / Nonexistent. `UtcInstant` has no `now()` constructor at all |
| G3 — Uncertainty | **PASS** — one composable uncertainty structure covers publication granularity, venue-published bounds (Binance ±15 s), and process-start character (NYSE), reused identically by phase boundaries and events. `PhaseOutcome::Unknown(CoverageGap)` is a data variant, not an error |
| G4 — Reproducibility | **PASS with the stated mitigation** — dataset revisions are immutable full snapshots with a `supersedes` chain; answers carry the revisions that produced them. The `tzdb-bundle-always` build requirement stands and is task #1 |
| G5 — Library-first | **PASS, structurally** — `resolve_phase` takes `at: UtcInstant` as a mandatory first parameter, and `Ruleset::from_parts` accepts only materialized in-memory values, never a path, URL, or handle. The core's dependency graph contains nothing capable of a clock read, so a violation fails to compile |
| G6 — Golden vectors | **PASS** — 24 vectors enumerated (SSE 8, NYSE 9, Binance 7) covering both DST directions, holidays, early close, break, auctions, funding boundaries, phase boundaries, and out-of-coverage. All eight Success Criteria map to a concrete runnable check |
| G7 — Shared vocabulary | **PASS** — 8-kind closed vocabulary; `EventKind` is a separate closed enum so events and phases cannot be conflated. Zero venue-specific phase names. **Caveat: see D4a** |

**G7 caveat — the invariant did its job.** Designing against FR-008 exposed that the captured
SSE schedule does not tile all time: 09:25–09:30 and 15:00–15:05 have no assigned phase
(research D4a). Two agents found this independently. The gate still passes because the
vocabulary is sound and the tiling invariant is structurally enforced — it is the *data* that
is incomplete, and the invariant is what surfaced it. Both intervals are blocking verification
items against the SSE Trading Rules before any SSE dataset revision is built, and MUST NOT be
filled by inference.

**No unjustified violations.** Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-venue-session-state/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output — D1 through D7
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/
├── market-time-core/         # Pure. No I/O, no network, no clock.
│   ├── src/
│   │   ├── phase.rs          # Shared phase vocabulary (FR-005, FR-006)
│   │   ├── event.rs          # Events overlaid on phases (FR-007, FR-008)
│   │   ├── instant.rs        # ns instants + declared time scale (FR-013)
│   │   ├── uncertainty.rs    # Uncertainty and explicit unknown (FR-002, FR-011)
│   │   ├── evidence.rs       # Per-rule provenance (FR-009, FR-010)
│   │   ├── coverage.rs       # Declared coverage; enforces FR-002 / FR-018
│   │   ├── ruleset.rs        # Dataset revisions; resolution (FR-016, FR-017)
│   │   └── resolve.rs        # instant + venue -> phase (FR-001, FR-003, FR-004)
│   └── tests/
│       ├── vectors/          # Golden vectors — permanent, never deleted
│       └── contract/
├── market-time-scales/       # Ingest boundary: hifitime bridge (the D1a seam)
├── market-time-data/         # Dataset revisions + loaders. I/O lives here, not in core.
├── market-time-cli/          # Thin shell
└── market-time-board/        # Thin shell — the global board
```

**Structure Decision**: a Cargo workspace with a pure `market-time-core` and everything that
touches the outside world in sibling crates. This makes Principle IV structural rather than
aspirational — `market-time-core` declares no dependency capable of I/O or clock reads, so a
violation fails to compile instead of failing review.

`market-time-scales` is a separate crate specifically to make the D1a seam visible. Conversion
from a tagged non-UTC scale into UTC happens there and nowhere else, so the leap-second-naive
boundary has exactly one location in the tree.

The board is its own crate rather than a page bolted onto the CLI, because Principle IV
requires it to hold no domain logic — including the logic for deciding what an unknown means.
*What* it renders comes from the core; *how* it renders is its own business.

## Complexity Tracking

No Constitution Check violations. Table intentionally empty.

## Phase 0 Outcome

See [research.md](./research.md). Seven decisions recorded; two changed the spec before any
code existed:

- **D3** — NYSE does not randomise its open; it has no single open instant, because opening is
  a per-security market-maker process. The spec's edge case was rewritten and FR-011b added.
- **D5a** — Binance publishes a 15-second deviation on funding settlement times. FR-011a now
  requires our stated uncertainty to be no narrower than a venue's own published bound.

## Carried forward — blocking, not optional

Nothing below may be quietly dropped. Each blocks a specific downstream step.

| # | Item | Blocks |
|---|---|---|
| 1 | **SSE 09:25–09:30 and 15:00–15:05 have no assigned phase** (D4a). Verify against SSE Trading Rules. MUST NOT be inferred | any SSE dataset revision |
| 2 | NYSE governing terms — fetch and record `ice.com/terms-of-use` verbatim | any NYSE ingestion |
| 3 | Binance terms — re-fetch with a JS-capable browser, record verbatim (SPA defeated plain fetch) | any Binance ingestion |
| 4 | NYSE full session boundary table — pre-market start, core close, after-hours end are not yet sourced first-party | NYSE vectors NYSE-1/3/8 |
| 5 | NYSE early-close footnote — substance established (13:00, day after Thanksgiving + Christmas Eve), exact wording needs one byte-exact pass | NYSE early-close vector |
| 6 | `jiff` `Disambiguation::Reject` — assert against a real spring-forward and fall-back instant by golden vector, do not trust the docs | DST vectors |
| 7 | Phase boundary ownership convention — which side of an edge is inclusive. No source document settles this; it is ours to decide and record | `resolve` implementation |

**Blocking on data, not on design**: SSE Trading Rules Art. 5.1.3 reserves all use and
publication of its trading information to the Exchange. SSE data is therefore
non-redistributable under `DATA-LICENSING.md` and MUST NOT be vendored into this repository.
The design accommodates this — `market-time-data` supports fetch-at-run-time sources — and that
constraint is a large part of why that crate exists as a boundary at all.
