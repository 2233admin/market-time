# Changelog

All notable changes to this project are recorded here. Dataset revisions are versioned
separately from the code and are never distributed from this repository; see
`DATA-LICENSING.md`.

This project follows semantic versioning. The constitution
([`CONSTITUTION.md`](CONSTITUTION.md)) is versioned on its own track, currently 1.2.1.

## [0.1.0] — 2026-07-30

First release with working code. Nothing is published to crates.io yet, and no venue data
ships here, now or ever.

### Added

- **`market-time-core`** — the phase engine. Pure: no I/O, no network, and no clock read in
  its decision path, enforced by a dependency allow-list and a source scan rather than by
  review.
  - `UtcInstant` with no `now()` constructor, and half-open `[start, end)` intervals as the
    repository-wide boundary ownership convention.
  - A closed eight-kind `Phase` vocabulary, and a separate closed `EventKind` so a scheduled
    occurrence can never be substituted for a state.
  - `Uncertainty` covering publication granularity, a venue-published bound, a process start
    that carries no invented number, and the two daylight-saving cases. Combining never
    sharpens: an unbounded case is wider than any number.
  - `EvidenceRef` and `DerivationNote`, neither constructible empty.
  - Rules stored as civil time-of-day change points tied to a venue's zone, so the tiling
    invariant is structural — a day with a gap in it cannot be expressed.
  - `Ruleset::from_parts`, which validates once: revisions exist, rules sit inside coverage,
    and every date inside coverage has a rule.
  - `resolve_phase`, `resolve_phases`, and `resolve_timeline`.
- **`market-time-scales`** — the one leap-second seam. TAI and GPS to UTC through hifitime's
  IERS table, in integer nanoseconds.
- **`market-time-data`** — the only crate that opens a file. JSON dataset format, with the
  phase vocabulary enforced at load.
- **`market-time-cli`** — `phase`, `evidence`, `timeline`, `board`, and `venues`. Reads the
  clock so the core never does, and reports the host clock's discipline as unmeasured rather
  than inventing a bound. `--format json` for machine consumers: an unknown is
  `"phase": null` with a stated reason, never `"closed"`.
- **`market-time-board`** — the timeline board. One row per venue, phases across a shared
  axis, a marker on the instant being viewed, the viewer's zone as axis labelling only.
  Unknown renders distinctly from closed. `inspect` returns what a segment rests on, and the
  board prints a sources block beneath its rows. `render_svg` draws the same board as a
  self-contained SVG — the shape a global trading-hours board is recognised in — keeping
  every honesty rule the text renderer has: status in words, hatching for not-known, a soft
  edge on a process-start boundary, and the sources underneath.
- **The operator path** — `SourceRegistration` (which cannot be built without the terms the
  source may be used under), the `SourceFetcher` trait with a `FileFetcher` implementation,
  and `RevisionAssembly`, which transcribes evidence from the retrieval — URL, fetch time,
  terms, and a sha256 digest of the bytes — and validates through the loader before writing
  anything. No HTTP client is vendored: the transport is the operator's.
- **An SSE adapter** — `adapters::sse`, the first venue adapter: a parser for the session
  table as SSE publishes it, a mapping from the venue's own session names to the shared
  vocabulary, and a refusal to assign any interval the document leaves unlabelled without a
  ruling that carries its reasoning. It holds no session times.
- **A venue catalog** — `AssetFamily` (a closed set: equities, spot and FX, futures) and
  `VenueProfile` (display name, location, family). Both renderers group rows by family the
  way a conventional board does, print the city under the venue's name, and report how many
  venues are trading with not-known counted separately.
- A synthetic three-venue fixture at
  `crates/market-time-data/fixtures/synthetic-venues.json`, so the tool can be run and
  verified without data anyone is forbidden to redistribute.

### Governance

- Constitution amended to 1.2.1: the workflow section was retargeted from spec-kit to
  OpenSpec. No principle added, removed, or redefined.
- The workflow moved from spec-kit to OpenSpec 1.7.0; spec material lives in `openspec/`.

### Verification

76 tests, including 24 golden vectors and two mechanical Principle IV guards. `cargo fmt`,
`clippy -D warnings`, and `cargo test --workspace` all clean. Quickstart validation recorded
in `docs/venue-session-state/quickstart-results.md`.

### Not in this release

- Real SSE, NYSE, or Binance schedules, and therefore any claim of accuracy about a real
  venue. The engine is verified on data shaped like theirs.
- An NYSE adapter, and the first-party sourcing of its session table and early-close
  footnote. Same shape as the SSE adapter; it needs NYSE's terms accepted first.
- Prices, quotes, trades, or volumes. Mark Time answers schedule questions.
