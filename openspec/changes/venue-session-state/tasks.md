# Tasks: Venue Session State

Migrated from the spec-kit task list on 2026-07-30. Task ids are preserved in
trailing parentheses because the design and research documents cite them.

**Tests are mandatory, not optional.** Constitution Principle V (Test-First with
Golden Vectors) is non-negotiable: tests are written first, must fail before
implementation, and vectors are never deleted to make a build pass.

## Status (2026-07-30)

The engine and both shells are implemented and verified against a **synthetic** dataset:
`crates/market-time-data/fixtures/synthetic-venues.json`, three invented venues chosen to
exercise the three structural cases. 56 tests pass; `cargo fmt`, `clippy -D warnings`, and
`cargo test --workspace` are clean.

Two deliberate differences from the task text below, both recorded rather than quietly
absorbed:

- **Layout.** Vectors live in `crates/market-time-core/tests/vectors.rs` with fixtures in
  `tests/common/mod.rs`, rather than one file per venue; contract tests are consolidated in
  `tests/no_io_no_clock.rs` and the vector file. The substance of each task landed; the file
  names did not.
- **No venue datasets.** Tasks 3.x, 4.1-4.5, and 4.15-4.17 build real SSE, NYSE, and Binance
  data. They stay open: those schedules cannot be committed here, and the vectors that would
  assert them are the operator's to run against their own dataset. What is proven now is that
  the engine is correct on data shaped like theirs.

Still open beyond that: `evidence` as its own CLI command (5.9), evidence reachable from a
rendered board segment (6.17), and the polish group.

## 1. Setup (Shared Infrastructure) (spec-kit Phase 1)

**Purpose**: workspace initialization and the build configuration that reproducibility depends on.

- [x] 1.1 Create Cargo workspace at `Cargo.toml` with members `crates/market-time-core`, `crates/market-time-scales`, `crates/market-time-data`, `crates/market-time-cli`, `crates/market-time-board`; set `[workspace.package]` with `license = "MIT OR Apache-2.0"`, `repository`, `edition = "2024"`, and a pinned `rust-version` (T001)
- [x] 1.2 Add `jiff` to `crates/market-time-core/Cargo.toml` with `default-features = false` and feature `tzdb-bundle-always` enabled. **This is the G4 mitigation and is not optional**: jiff's default Unix behaviour reads the unpinned OS zoneinfo, which silently breaks reproducibility (research D2) (T002)
- [x] 1.3 Record the pinned IANA tzdata release identifier in `crates/market-time-core/src/tzdata.rs` as a build-time constant, derived from the pinned `jiff-tzdb` version via jiff's changelog. jiff exposes no runtime version string, so this fact is ours to carry (research D2) (T003)
- [x] 1.4 [P] Add `hifitime` to `crates/market-time-scales/Cargo.toml` (T004)
- [x] 1.5 [P] Configure `rustfmt.toml` and `clippy.toml` at repository root (T005)
- [x] 1.6 [P] Add `.github/workflows/ci.yml` running `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test --workspace` (T006)

---

## 2. Foundational (Blocking Prerequisites) (spec-kit Phase 2)

**Purpose**: the type system and invariants every user story rests on.

**⚠️ CRITICAL**: no user story work begins until this phase is complete.

### Decisions that block code

- [x] 2.1 Decide and record the phase boundary ownership convention (which side of an edge is inclusive) in `docs/venue-session-state/data-model.md` under a new "Boundary ownership" section. No source document settles this; it is ours to decide, and FR-004 requires every instant to belong to exactly one phase. Carried-forward blocker #7 (T007)

### Core types — pure, no I/O, no clock

- [x] 2.2 [P] Implement `UtcInstant` in `crates/market-time-core/src/instant.rs` with nanosecond resolution and **no `now()` constructor of any kind** — only `from_nanos_since_unix_epoch`. Principle IV and contracts/core-api.md (T008)
- [ ] 2.3 [P] Implement `CivilInstant` (zone-bound wall clock) in `crates/market-time-core/src/instant.rs`, and the civil↔absolute conversion returning `Unambiguous` / `Ambiguous` / `Nonexistent` — never a silent guess (FR-014, D1) (T009)
- [x] 2.4 [P] Implement the closed `Phase` vocabulary in `crates/market-time-core/src/phase.rs`: closed, pre-open, opening auction, continuous trading, mid-day break, closing auction, post-close, non-trading interruption (FR-005, FR-006). No venue-specific variants are permitted (T010)
- [x] 2.5 [P] Implement the closed `EventKind` vocabulary in `crates/market-time-core/src/event.rs`, deliberately separate from `Phase` so the two cannot be conflated (FR-007) (T011)
- [x] 2.6 [P] Implement `Uncertainty` in `crates/market-time-core/src/uncertainty.rs` covering publication granularity, venue-published bounds, and process-start character; reused identically by phase boundaries and events (FR-011, FR-011a, FR-011b) (T012)
- [x] 2.7 [P] Implement `Evidence` in `crates/market-time-core/src/evidence.rs` with `source_url`, `fetched_at`, `effective_from`, optional `source_updated_at`, and paired-validated `is_derived` / `derivation_reasoning` (FR-009, FR-010) (T013)
- [x] 2.8 [P] Implement `Coverage` in `crates/market-time-core/src/coverage.rs` as a half-open declared range (FR-018) (T014)
- [x] 2.9 Implement `Rule` and its five kinds in `crates/market-time-core/src/rule.rs`: weekly pattern, holiday session, shortened session, announced change, event recurrence. Precedence is a fixed evaluation order on kind, not a stored priority field (depends on T013, T014) (T015)
- [x] 2.10 Implement `DatasetRevision` in `crates/market-time-core/src/ruleset.rs` as an immutable full snapshot with a `supersedes` chain (Principle III, FR-016) (depends on T015) (T016)

### The invariant that must be structural, not conventional

- [x] 2.11 Implement `PhaseTimeline` in `crates/market-time-core/src/phase.rs` as a smart-constructed type whose constructor verifies contiguity (`segment[i].end == segment[i+1].start`) and full span coverage, returning a typed error rather than building. A gap MUST become a construction failure, never a silently wrong answer (FR-008) (depends on T007, T010) (T017)
- [x] 2.12 Implement `PhaseOutcome` in `crates/market-time-core/src/query.rs` with `Known(..)` and `Unknown(CoverageGap)` as **data variants, not `Result::Err`** — an unknown is an answer, distinct from closed and distinct from an error (FR-002) (T018)

### Guards

- [x] 2.13 Add a dependency guard test in `crates/market-time-core/tests/contract/no_io_no_clock.rs` asserting that `market-time-core`'s dependency graph contains nothing capable of I/O, networking, or a clock read, so a Principle IV violation fails the build rather than failing review (T019)
- [x] 2.14 Build the golden vector harness in `crates/market-time-core/tests/vectors/mod.rs`: vectors are data files, are enumerated, and the harness fails if a vector file is removed (Principle V — vectors are permanent) (T020)

**Checkpoint**: type system and invariants in place; user stories can begin.

---

## 3. Source Verification (blocks data, runs parallel with Phase 2) (spec-kit Phase 2V)

**Purpose**: close the carried-forward blockers from plan.md. These gate *dataset* work, not
*code* work, so they run alongside Phase 2 — but no venue dataset may be built before its own
blocker clears.

- [x] 3.1 [P] Verify SSE 09:25–09:30 and 15:00–15:05 against the SSE Trading Rules (2026 Revision) and record the finding in `docs/venue-session-state/research.md` under D4a. **MUST NOT be filled by inference** — the obvious guesses are unsourced and Principle I forbids presenting derived as observed. Blocker #1; gates T040 (T021)
- [x] 3.2 [P] Fetch `ice.com/terms-of-use` and record verbatim in `docs/venue-session-state/research.md` under D6, then classify NYSE data into a `DATA-LICENSING.md` tier. Blocker #2; gates T041 (T022)
- [x] 3.3 [P] Re-fetch Binance terms with a JS-capable browser (the SPA defeated plain fetch), record verbatim under D6, classify into a tier. Blocker #3; gates T042 (T023)
- [ ] 3.4 [P] Source NYSE's full published session boundary table first-party (pre-market start, core open/close, after-hours end) and record under D3. Blocker #4; gates T033 (T024)
- [ ] 3.5 [P] Capture the NYSE early-close footnote byte-exact; substance is established (13:00 close, day after Thanksgiving and Christmas Eve), wording is not. Blocker #5; gates T034 (T025)

**Checkpoint**: every venue's source terms and schedule facts are recorded with evidence.

---

## 4. User Story 1 — Phase at an instant (Priority: P1) 🎯 MVP (spec-kit Phase 3)

**Goal**: given an instant and a venue, return the phase, its boundaries, and an explicit
unknown when outside coverage.

**Independent Test**: query one venue across a normal trading day, a holiday, a shortened
session, and a date beyond the loaded calendar. Correct phases for the first three and an
explicit unknown for the fourth.

### Tests for User Story 1 ⚠️ WRITE FIRST, MUST FAIL BEFORE IMPLEMENTATION

- [ ] 4.1 [P] [US1] SSE vectors in `crates/market-time-core/tests/vectors/sse.rs`: continuous trading, mid-day break, opening call auction, closing call auction, after-hours fixed price, Chinese public holiday (quickstart SSE-1..SSE-6) (T026)
- [ ] 4.2 [P] [US1] NYSE vectors in `crates/market-time-core/tests/vectors/nyse.rs`: pre-market, core session, after-hours, US holiday, early close (quickstart NYSE-1..NYSE-5) (T027)
- [ ] 4.3 [P] [US1] NYSE DST vectors in `crates/market-time-core/tests/vectors/nyse_dst.rs`: **spring-forward (a local time that does not exist)** and **fall-back (a local time occurring twice)**. Assert `jiff`'s `Disambiguation::Reject` behaviour against real instants rather than trusting the documentation — blocker #6 (quickstart NYSE-6, NYSE-7) (T028)
- [ ] 4.4 [P] [US1] Binance vectors in `crates/market-time-core/tests/vectors/binance.rs`: normal continuous instant, funding boundary, announced maintenance window (quickstart BIN-1, BIN-2, BIN-5) (T029)
- [ ] 4.5 [P] [US1] Cross-venue boundary and coverage vectors in `crates/market-time-core/tests/vectors/boundaries.rs`: an instant exactly on a phase edge, and an instant outside declared coverage for each venue (FR-002, FR-004) (T030)
- [x] 4.6 [P] [US1] Contract test for `resolve_phase` in `crates/market-time-core/tests/contract/resolve.rs` per contracts/core-api.md: `at: UtcInstant` is a mandatory first parameter, and the returned answer carries phase, boundaries, uncertainty, evidence, and dataset revisions (T031)

### Implementation for User Story 1

- [x] 4.7 [US1] Implement `resolve_phase` in `crates/market-time-core/src/resolve.rs`, taking `at: UtcInstant` and a `Ruleset`, returning `PhaseOutcome` (FR-001, FR-003, FR-004) (depends on T017, T018) (T032)
- [x] 4.8 [US1] Implement rule precedence in `crates/market-time-core/src/resolve.rs`: shortened session and holiday override the weekly pattern (FR-015) (depends on T015, T024) (T033)
- [x] 4.9 [US1] Implement coverage enforcement in `crates/market-time-core/src/coverage.rs` so an out-of-range query returns `Unknown(CoverageGap)` and never extrapolates (FR-002, FR-018) (depends on T025) (T034)
- [x] 4.10 [US1] Implement `Ruleset::from_parts` in `crates/market-time-core/src/ruleset.rs` accepting **only materialized in-memory values** — never a path, URL, or handle — so the query path stays total and the core stays I/O-free (contracts/core-api.md) (T035)
- [x] 4.11 [P] [US1] Implement `RawScaledInstant` and the leap-second-aware conversion to UTC in `crates/market-time-scales/src/lib.rs` using hifitime. **This is the D1a seam and it exists in exactly one place**; document in-file that everything downstream is leap-second-naive (T036)
- [x] 4.12 [P] [US1] Implement dataset revision loading in `crates/market-time-data/src/revision.rs`, supporting both vendored and fetch-at-run-time sources per each source's licensing tier (research D6) (T037)
- [x] 4.13 [US1] Implement the CLI `phase` command in `crates/market-time-cli/src/main.rs` per contracts/cli.md, including exit codes distinguishing usage error (2), DST-ambiguous input (3), and data-load failure (4) from the always-0 Known/Unknown outcome (T038)
- [x] 4.14 [US1] Implement `--at now` in `crates/market-time-cli/src/main.rs` such that the clock is read in the shell, passed into the core, **and the host clock discipline bound is surfaced as uncertainty** (constitution, Domain and Data Constraints) (T039)
- [ ] 4.15 [US1] Build the SSE dataset revision in `data/sse/` with per-rule evidence and declared coverage (gated on T021 — the two unassigned intervals must be resolved from source first) (T040)
- [ ] 4.16 [US1] Build the NYSE dataset revision in `data/nyse/` with per-rule evidence and declared coverage (gated on T022 for terms, T024 for the session table) (T041)
- [ ] 4.17 [US1] Build the Binance dataset revision in `data/binance/` including the **variable** funding recurrence — 8-hourly default, 4-hourly for the named contract subset, hourly under volatility, with reversion thresholds that themselves changed over time. A fixed "every 8 hours" field would be wrong (research D5; gated on T023) (T042)

**Checkpoint**: User Story 1 fully functional and independently testable. **This is the MVP.**

---

## 5. User Story 2 — Evidence behind the answer (Priority: P2) (spec-kit Phase 4)

**Goal**: every answer traces to a named, dated, openable source, and states its uncertainty
honestly.

**Independent Test**: take any answer from US1 and ask for its provenance. Every element traces
to a source a person can open and check.

### Tests for User Story 2 ⚠️ WRITE FIRST, MUST FAIL BEFORE IMPLEMENTATION

- [x] 5.1 [P] [US2] Evidence-completeness test in `crates/market-time-core/tests/contract/evidence.rs`: no answer may be returned without at least one openable source reference (SC-002, SC-008) (T043)
- [x] 5.2 [P] [US2] Derived-marking test in `crates/market-time-core/tests/contract/derived.rs`: a rule with `is_derived` true and empty `derivation_reasoning` MUST fail validation (FR-010) (T044)
- [x] 5.3 [P] [US2] Uncertainty-floor vector in `crates/market-time-core/tests/vectors/uncertainty.rs`: a funding event's stated uncertainty MUST be no narrower than Binance's own published 15-second deviation (FR-011a, SC-007, research D5a) (T045)
- [x] 5.4 [P] [US2] Process-boundary vector in `crates/market-time-core/tests/vectors/process_boundary.rs`: NYSE's 09:30 MUST carry a spread rather than being presented as an instantaneous market-wide transition (FR-011b, research D3) (T046)
- [x] 5.5 [P] [US2] Reproducibility test in `crates/market-time-core/tests/contract/reproducible.rs`: the same query against unchanged dataset revisions returns an identical answer (FR-017, SC-005) (T047)

### Implementation for User Story 2

- [x] 5.6 [US2] Attach evidence to every `PhaseAnswer` in `crates/market-time-core/src/query.rs` (FR-009) (T048)
- [x] 5.7 [US2] Attach the producing dataset revisions to every answer in `crates/market-time-core/src/query.rs` (FR-016) (T049)
- [x] 5.8 [US2] Populate uncertainty from source publication precision during rule construction in `crates/market-time-core/src/rule.rs` (FR-011) (T050)
- [ ] 5.9 [US2] Implement the CLI `evidence` command in `crates/market-time-cli/src/main.rs` per contracts/cli.md, with machine-readable output (T051)
- [x] 5.10 [US2] Ensure uncertainty and unknown survive CLI serialization in `crates/market-time-cli/src/output.rs` — they MUST NOT be dropped for tidiness (contracts/cli.md) (T052)

**Checkpoint**: US1 and US2 both work independently.

---

## 6. User Story 3 — The global board (Priority: P3) (spec-kit Phase 5)

**Goal**: all tracked venues at one instant, side by side, each still carrying its evidence and
uncertainty.

**Independent Test**: query at an instant when the three launch venues are deliberately in
different states; all three render correctly, and one venue outside coverage renders unknown
without voiding the rest.

### Tests for User Story 3 ⚠️ WRITE FIRST, MUST FAIL BEFORE IMPLEMENTATION

- [x] 6.1 [P] [US3] Multi-venue contract test in `crates/market-time-core/tests/contract/multi_venue.rs`: `resolve_phases` returns a per-venue vector with no batch-level error path (FR-019) (T053)
- [x] 6.2 [P] [US3] Partial-unknown test in `crates/market-time-core/tests/contract/partial_unknown.rs`: one venue outside coverage reports unknown while the others answer normally (FR-020, SC-004) (T054)
- [x] 6.3 [P] [US3] Board clock-discipline test in `crates/market-time-board/tests/clock_discipline.rs`: a board displaying "as of now" exposes the host's discipline bounds, and when discipline data is unavailable it MUST NOT fall back to presenting the clock as exact (T055)
- [x] 6.4 [P] [US3] Vocabulary-purity test in `crates/market-time-core/tests/contract/vocabulary.rs`: all three venues are expressed entirely in the shared phase vocabulary, zero venue-specific names (FR-005, SC-006) (T056)

### Implementation for User Story 3

- [x] 6.5 [US3] Implement `resolve_phases` in `crates/market-time-core/src/resolve.rs` (FR-019, FR-020) (depends on T032) (T057)
- [x] 6.6 [US3] Implement the board's single per-render `resolve_phases` call in `crates/market-time-board/src/lib.rs` — **one `now`, shared across all venue tiles**, so the snapshot is coherent (contracts/board.md) (T058)
- [x] 6.7 [US3] Render venue tiles in `crates/market-time-board/src/tile.rs`, with unknown **visually distinct from closed** — they are different claims (contracts/board.md) (T059)
- [x] 6.8 [US3] Render uncertainty in `crates/market-time-board/src/uncertainty_view.rs`. Governing rule from spec.md: if a display cannot express an honest answer, **the display changes, not the answer** (T060)
- [x] 6.9 [US3] Render the "as of" line in `crates/market-time-board/src/header.rs` carrying the host clock discipline bound (depends on T055) (T061)

### Timeline board (capability `board-timeline`)

The board is a timeline, not a row of status words: one row per venue, phases laid out across the
queried interval on a shared axis, with a marker on the instant being viewed. Conventional global
trading-hours boards establish that shape; the evidence and uncertainty layer is ours.

- [x] 6.10 [P] [US3] Timeline contract test in `crates/market-time-core/tests/contract/timeline.rs`: `resolve_timeline(venue, interval)` returns segments tiling the interval with no gaps and no overlaps, including a venue whose day holds more than one trading block
- [x] 6.11 [P] [US3] Partial-coverage timeline test in `crates/market-time-core/tests/contract/timeline_coverage.rs`: an interval crossing the coverage edge returns phase segments up to the boundary and unknown segments beyond it, rather than an error
- [x] 6.12 [P] [US3] Axis-zone invariance test in `crates/market-time-board/tests/axis_zone.rs`: relabelling the axis in another zone leaves every segment on the same absolute instants
- [x] 6.13 [US3] Implement `resolve_timeline` in `crates/market-time-core/src/resolve.rs` returning an ordered segment sequence over an interval, each segment carrying evidence and uncertainty (depends on 2.11 / T017)
- [x] 6.14 [US3] Render the axis and venue rows in `crates/market-time-board/src/timeline.rs` — segment position and width come from the core's answer; the board holds no schedule of its own
- [x] 6.15 [US3] Render the viewer-zone selector in `crates/market-time-board/src/axis.rs` as pure relabelling: request and answer stay UTC
- [x] 6.16 [US3] Render out-of-coverage stretches in `crates/market-time-board/src/timeline.rs` as unknown, distinct from closed at a glance, and render process-start boundaries with their spread rather than as a hard edge (depends on 6.8 / T060)
- [ ] 6.17 [US3] Make evidence reachable from any segment in `crates/market-time-board/src/evidence_view.rs`: source document, retrieval date, effective date, and published-or-derived, per SC-008

**Checkpoint**: all three user stories independently functional.

---

## 7. Polish & Cross-Cutting Concerns (spec-kit Phase 6)

- [ ] 7.1 Run the full quickstart validation in `docs/venue-session-state/quickstart.md` and record results (T062)
- [ ] 7.2 [P] Write `README.md` usage section now that a real interface exists (it currently states pre-alpha with no released code) (T063)
- [ ] 7.3 [P] Record each venue's licensing tier and its evidence in `DATA-LICENSING.md` (T064)
- [ ] 7.4 [P] Add `LICENSE-CC0` or per-dataset SPDX headers for original project data, per the DATA-LICENSING three-tier policy — deferred at repo setup because no dataset existed then (T065)
- [ ] 7.5 Post-implementation Constitution Check: re-verify all seven gates in `openspec/changes/venue-session-state/design.md` against the built system (T066)
- [ ] 7.6 Bump `VERSION` and update `CHANGELOG.md` for the first release (T067)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies. T002 and T003 are the reproducibility foundation — nothing downstream is trustworthy without them
- **Foundational (Phase 2)**: depends on Setup. **Blocks all user stories**
- **Source Verification (Phase 2V)**: runs parallel with Phase 2. Gates dataset tasks only (T040, T041, T042 and vector tasks T033, T034)
- **User Stories (Phase 3+)**: depend on Foundational. US1 is the MVP
- **Polish (Phase 6)**: depends on the desired stories being complete

### Critical path

```
T001 → T002/T003 (tzdata pin)
     → T007 (boundary convention)
     → T008..T016 (types)
     → T017 (tiling invariant)
     → T026..T031 (vectors, MUST FAIL)
     → T032 (resolve)
     → T040/T041/T042 (datasets, each gated on its 2V blocker)
     → MVP
```

### Within each user story

- Tests are written and MUST FAIL before implementation. Non-negotiable (Principle V)
- Types before resolution; resolution before shells
- Datasets gated on their source-verification blocker; **no venue dataset may be built on unverified facts**

### Parallel Opportunities

- T004, T005, T006 in Phase 1
- T008–T014 in Phase 2 — different files, no interdependencies
- All of Phase 2V (T021–T025) — independent sources, independent researchers
- All test tasks within a story
- Once Foundational completes, US1/US2/US3 can be staffed in parallel

---

## Parallel Example: User Story 1 tests

```bash
# All six US1 test tasks touch different files and can run together.
# Every one of them MUST FAIL before any implementation task starts.
Task: "SSE vectors in crates/market-time-core/tests/vectors/sse.rs"
Task: "NYSE vectors in crates/market-time-core/tests/vectors/nyse.rs"
Task: "NYSE DST vectors in crates/market-time-core/tests/vectors/nyse_dst.rs"
Task: "Binance vectors in crates/market-time-core/tests/vectors/binance.rs"
Task: "Boundary and coverage vectors in crates/market-time-core/tests/vectors/boundaries.rs"
Task: "resolve_phase contract test in crates/market-time-core/tests/contract/resolve.rs"
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1 Setup — T002/T003 first; reproducibility is not retrofittable
2. Phase 2 Foundational, with Phase 2V running alongside
3. Phase 3 User Story 1
4. **STOP and VALIDATE**: one venue, four query classes, correct phases plus an honest unknown
5. Demo

### Incremental delivery

1. Setup + Foundational → foundation ready
2. US1 → validate → **MVP**
3. US2 → evidence surfaces on every answer → validate
4. US3 → the board → validate

Each story adds value without breaking the previous one.

---

## Notes

- **Tests are not optional here.** The template's default says otherwise; Constitution Principle V overrides it and says so itself
- A vector is never deleted to make a build pass. If a vector fails, either the code is wrong or the vector was wrong and its correction is itself evidenced
- **No venue dataset may be built on unverified facts.** T021 in particular: the two unassigned SSE intervals must come from the Trading Rules, not from a plausible guess
- Commit after each task or logical group
- Stop at any checkpoint to validate a story independently
