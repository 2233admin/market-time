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
  - Regional session bands: `BandDefinition`, `SessionBand`, `BandOverlap`, `derive_band`,
    and `derive_overlap`. A band is never a published fact — it carries a `DerivationNote`
    naming why its members were grouped, never evidence of its own — and an unknown member
    is never dropped from the vote: the whole band reads unknown for that stretch rather
    than narrowing to what the remaining members say. Uncertainty only ever widens across a
    band's or an overlap's contributors, and an unknown band can never prove an overlap's
    absence.
- **`market-time-scales`** — the one leap-second seam. TAI and GPS to UTC through hifitime's
  IERS table, in integer nanoseconds.
- **`market-time-data`** — the only crate that opens a file. JSON dataset format, with the
  phase vocabulary enforced at load. Session bands are loadable too: a `bands` array on the
  dataset file, each entry checked against that same file's venues (a band may not name a
  venue the dataset cannot answer for) and against duplicate ids, with `derived_reasoning`
  required rather than optional — a band with no stated reasoning cannot be expressed.
- **`market-time-cli`** — `phase`, `evidence`, `timeline`, `board`, `bands`, and `venues`.
  Reads the clock so the core never does, and reports the host clock's discipline as
  unmeasured rather than inventing a bound. `--format json` for machine consumers: an
  unknown is `"phase": null` with a stated reason, never `"closed"`. `bands` derives the
  dataset's session bands over the same `--at`/`--hours` window `timeline` uses, and every
  unordered pair of the selected bands as a computed overlap — both carry their derivation
  note into the output, text or JSON, so neither can be mistaken for a venue-published
  window. `board` now derives and draws those same bands and overlaps by default, beneath
  the venue rows, in both `--format text` and `--format svg`; `--no-bands` suppresses the
  section, and a dataset with no bands declared renders with no section either way — never
  an empty one.
- **`market-time-board`** — the timeline board. One row per venue, phases across a shared
  axis, a marker on the instant being viewed, the viewer's zone as axis labelling only.
  Unknown renders distinctly from closed. `inspect` returns what a segment rests on, and the
  board prints a sources block beneath its rows. `render_svg` draws the same board as a
  self-contained SVG — the shape a global trading-hours board is recognised in — keeping
  every honesty rule the text renderer has: status in words, hatching for not-known, a soft
  edge on a process-start boundary, and the sources underneath.
  - `BoardView` gains a `bands: BandSection` field — the caller's already-derived
    `SessionBand`s and `BandOverlap`s, drawn as their own rows beneath the venue section in
    both renderers. `BandSection` defaults to empty, and empty renders nothing — no
    heading, no placeholder — so every caller and test that predates bands renders
    byte-identically to before. A band or an overlap is never a venue's published
    schedule, and neither renderer lets a reader forget it: every row is labelled
    "derived," and `band_glyph`/`overlap_glyph` (`+`/`~`/`x`) are their own vocabulary,
    never `glyph`'s phase characters. An `Unknown` band or overlap stretch uses the same
    `url(#not-known)` hatch the venue rows already use — the one visual promise this board
    makes about "not known," kept consistent rather than duplicated.
  - `render_html` — the board as one self-contained, interactive HTML page: the same
    `render_svg_with` output, embedded verbatim, with hover, a zone selector, and a live
    clock layered on top rather than a second drawing of the picture. Hovering or
    focusing any segment shows its evidence from an inline JSON payload this crate builds
    from `inspect` and the already-derived bands/overlaps at render time — the script
    never recomputes a phase, an uncertainty, or a zone conversion; the zone selector
    swaps between per-zone label sets this crate precomputes against the pinned IANA
    database, never `Intl.DateTimeFormat` or zone-aware `Date` arithmetic in the browser.
    The one deliberate exception is the live clock: when `now` came from a live
    host-clock read (never from a supplied instant), the script advances the already-drawn
    "now" line on the browser's own clock, permanently captioned "browser clock —
    discipline unmeasured," and never with more weight than an evidenced boundary. The
    page makes no network reference of any kind — no external asset URL survives outside
    an evidence source URL the dataset supplied — and reads with JavaScript disabled: the
    picture, legend, sources, and footer are static markup, and only hover, zone
    switching, and the clock are enhancements. The footer names the dataset revision(s)
    and pinned IANA tzdb version the page was rendered from. `market-time` CLI:
    `board --format html`.
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
  verified without data anyone is forbidden to redistribute. Its two session bands are
  synthetic groupings too — invented for this fixture so `bands` has something to derive
  and overlap, not any real desk's session definition.

### Governance

- Constitution amended to 1.2.1: the workflow section was retargeted from spec-kit to
  OpenSpec. No principle added, removed, or redefined.
- The workflow moved from spec-kit to OpenSpec 1.7.0; spec material lives in `openspec/`.

### Verification

163 tests, including 24 golden vectors and two mechanical Principle IV guards. `cargo fmt`,
`clippy -D warnings`, and `cargo test --workspace` all clean. Quickstart validation recorded
in `docs/venue-session-state/quickstart-results.md`.

### Not in this release

- Real SSE, NYSE, or Binance schedules, and therefore any claim of accuracy about a real
  venue. The engine is verified on data shaped like theirs.
- An NYSE adapter, and the first-party sourcing of its session table and early-close
  footnote. Same shape as the SSE adapter; it needs NYSE's terms accepted first.
- Prices, quotes, trades, or volumes. Mark Time answers schedule questions.
