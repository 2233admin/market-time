## Context

The repository has a Cargo workspace, a pinned tzdata bundle, and CI gates. It has no domain
types yet. This change designs the first delivery slice against the ratified constitution
(`CONSTITUTION.md`; gates below were checked against v1.2.0, and the 1.2.1 amendment changed
no principle) for three venues: Shanghai Stock Exchange, New York Stock Exchange, and Binance
USD-M perpetual futures.

The deep artifacts behind this design are preserved verbatim and are the authority on detail:

- `docs/venue-session-state/research.md` — Phase 0, decisions D1 through D7
- `docs/venue-session-state/data-model.md` — type-level design
- `docs/venue-session-state/contracts/` — core API, CLI, and board contracts
- `docs/venue-session-state/quickstart.md` — runnable validation scenarios
- `docs/venue-session-state/checklists/requirements.md` — spec quality checklist

## Goals / Non-Goals

**Goals:**

- Correct phase answers for three structurally different market models, verified against each
  venue's own published schedule.
- Evidence and uncertainty carried as values an agent cannot skip, not as rendering.
- Reproducibility that holds across machines and time: same dataset revisions plus same query
  yields the same answer.
- Principle IV made structural — a core that *cannot* read a clock or open a socket, because it
  declares no dependency capable of it.

**Non-Goals:**

- Observed venue behaviour (what the matching engine actually did). Published schedules only,
  with the two kept distinguishable so observed data can be added later.
- Prices, quotes, trades, or any live market data feed.
- Predicting unscheduled halts. A halt is recorded once published, never anticipated.
- Vendoring venue datasets. Every launch venue forbids commercial redistribution, so
  fetch-at-run-time is the only compliant ingestion shape — not one option among several.

## Decisions

**Language and dependencies.** Rust 2024, MSRV pinned. `jiff` for civil time, IANA zones, and
DST ambiguity; `hifitime` for the time-scale layer, chosen because it was the only surveyed
crate carrying a real IERS table (research D1).

**Workspace shape.** `market-time-core` is pure. `market-time-scales` isolates the hifitime
bridge so the leap-second-naive seam (D1a) has exactly one location in the tree.
`market-time-data` owns all I/O. `market-time-cli` and `market-time-board` are shells holding no
domain logic. The board is its own crate rather than a page on the CLI, because deciding what an
unknown *means* is core work; only how it renders belongs to the board.

**Clock discipline.** `resolve_phase` takes `at: UtcInstant` as a mandatory first parameter and
`UtcInstant` has no `now()` constructor. `Ruleset::from_parts` accepts materialized in-memory
values only — never a path, URL, or handle. A Principle IV violation fails to compile rather
than failing review.

**Time-scale seam.** `RawScaledInstant` (scale-tagged, ingest only) is a distinct type from the
core's UTC-only instant. Civil-to-absolute conversion is one explicit operation returning
Unambiguous, Ambiguous, or Nonexistent — daylight-saving edges are values, not panics.

**Unknown is data, not an error.** `PhaseOutcome::Unknown(CoverageGap)` is a variant of the
answer. Outside declared coverage the system names the boundary; it never extrapolates.

**Uncertainty is one composable structure**, reused identically by phase boundaries and events,
covering publication granularity, venue-published bounds (Binance publishes ±15 s on funding
settlement — a bound handed to us, not estimated), and process-start character (NYSE opens
security by security; 09:30 starts that process rather than marking a market-wide transition).

**The board is a timeline, not a status list.** One row per venue, that venue's phases laid out
across the queried interval on a shared axis, a marker on the instant being viewed, and the
viewer's zone applied as axis labelling only. That shape is the convention global trading-hours
boards already established, and matching it is deliberate: be legible as one of those boards
first, then carry the evidence and uncertainty layer through to the surface, which is the part
they do not do. It costs one core capability those boards do not need — `resolve_timeline` over
an interval rather than a point — which the phase timeline type already supports. Conventional
boards are a reference for what to display and never a source for what is true.

**Reproducibility mitigation (constitution gate G4), required not preferred.** `jiff` exposes no
runtime tzdata version string and its default Unix behaviour reads the unpinned OS zoneinfo,
which would break reproducibility silently. The build enables `tzdb-bundle-always` and records
the IANA release identifier as a build-time constant carried in dataset-revision metadata.
Without this, G4 fails.

**Dataset revisions** are immutable full snapshots with a `supersedes` chain; answers carry the
revisions that produced them.

**Golden vectors are the primary correctness instrument.** 24 vectors enumerated (SSE 8, NYSE 9,
Binance 7) covering both daylight-saving directions, holidays, early close, mid-day break,
auctions, funding boundaries, phase boundaries, and out-of-coverage. Vectors are permanent: a
failing vector means either the code is wrong or the vector was wrong and its correction is
itself evidenced.

## Risks / Trade-offs

**The SSE data does not tile all time.** Designing against the no-gaps invariant exposed that
09:25–09:30 and 15:00–15:05 have no assigned phase in the captured schedule (research D4a, found
independently by two agents). The vocabulary is sound and the invariant is what surfaced the
hole — but both intervals are blocking verification items against the SSE Trading Rules before
any SSE dataset revision is built, and MUST NOT be filled by inference.

**Boundary ownership is ours to decide.** No source document settles which side of a phase edge
is inclusive, and the requirement that every instant belongs to exactly one phase forces a
convention. It is recorded before code, not discovered during it.

**Rendering honesty is the known hard part.** "Shanghai: mid-day break" is easy to display; "New
York: open, boundary known only to the second, opening is a process" is not. The board must not
resolve this by hiding uncertainty. If a display cannot express an honest answer, the display
changes — the answer does not.

**Two spec corrections already came out of research**, before any code existed: NYSE does not
randomise its open (it has no single open instant), and Binance's funding deviation is a
published bound rather than something to estimate. Both are recorded in `research.md` rather
than quietly edited away, because the reasoning is worth more than the tidiness.

## Post-implementation Constitution Check (2026-07-30)

The gates were checked before research and again after design. This is the third check,
against code that exists rather than a plan that describes it.

| Gate | Verdict | What it rests on now |
|---|---|---|
| **G1 — Evidence (I)** | **PASS, structurally** | `EvidenceRef::new` rejects a blank source or effective date, and `DerivationNote::new` rejects blank reasoning, so "a rule without provenance" and "derived, unexplained" are states the types cannot hold. The loader turns either rejection into a load failure. |
| **G2 — Instants (II)** | **PASS** | `UtcInstant` is UTC by construction and has no `now()`. Non-UTC input converts in `market-time-scales` alone, through hifitime's IERS table, in integer nanoseconds — the earlier f64 path silently lost the last two digits at 1.7e18 ns and was replaced. |
| **G3 — Uncertainty (II)** | **PASS** | Every boundary and answer carries an `Uncertainty`. `widest` treats an unbounded case as wider than any number, so combining never sharpens. `PhaseOutcome::Unknown` is a variant, not an error, and reaches JSON as `"phase": null` with a reason. |
| **G4 — Reproducibility (III)** | **PASS with the stated mitigation** | `tzdb-bundle-always` is on and the release identifier is read from `jiff-tzdb::VERSION` rather than transcribed. Revisions are immutable with a `supersedes` chain; answers name the revisions that produced them; two consecutive runs are byte-identical (recorded in `docs/venue-session-state/quickstart-results.md`). |
| **G5 — Library-first (IV)** | **PASS, and now enforced mechanically** | Upgraded from the design-time verdict. `tests/no_io_no_clock.rs` fails the build if `market-time-core` gains a dependency outside its allow-list, or if any source file in it names `SystemTime`, a `now()` constructor, `std::fs`, `std::net`, or `std::process`. The promise is a test, not a habit. |
| **G6 — Golden vectors (V)** | **PASS** | 24 vectors covering both daylight-saving directions, holiday precedence, a shortened session, mid-day break, auctions, funding events, boundary ownership to the nanosecond, coverage edges, multi-venue partial unknowns, and determinism — plus a minute-by-minute tiling sweep across a day. |
| **G7 — Shared vocabulary** | **PASS** | The vocabulary is a closed `#[non_exhaustive]` enum owned by the core; shells cannot extend it and must handle the wildcard. A dataset naming its own phase fails to load. |

**One gate changed character rather than status.** G5 was a design-time "PASS, structurally"
resting on the dependency graph. It is now a build failure if violated, which is a different
kind of guarantee: the reviewer who would have had to notice is no longer load-bearing.

**What the check cannot cover.** G1 and G6 are satisfied against synthetic venues. A rule
carrying evidence is enforced; whether that evidence describes SSE correctly is a question
only real data can answer, and the tasks that would answer it are open by design.

**No unjustified violations. Complexity Tracking stays empty.**
