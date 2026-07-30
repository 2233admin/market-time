# Phase 0 Research: Venue Session State

**Feature**: `venue-session-state` | **Date**: 2026-07-29
**Constitution**: v1.2.0

Two research tracks ran in parallel: the Rust time-library landscape, and the first-party
published schedules and terms for the three launch venues. Findings that changed the spec are
called out explicitly.

---

## D1 — Time library: no single crate covers the requirements

**Decision**: `jiff` for the civil / wall-clock / DST layer, `hifitime` for the physical
time-scale layer, bridged at the UTC boundary.

**Rationale**: Principle II demands two things that no surveyed crate provides together:
nanosecond civil-time correctness with honest DST ambiguity (requirement A), and
leap-second-aware conversion across UTC / TAI / GNSS time scales (requirement B).

| Crate | Civil / DST | Time scales | Leap seconds | ns |
|---|---|---|---|---|
| `jiff` 0.2.x | **Best in class** | UTC only | **Explicitly unsupported** | yes |
| `chrono` + `chrono-tz` | good | UTC only | self-described incomplete | yes |
| `time` 0.3.x | none in core | UTC / fixed offset only | absent | yes |
| `hifitime` 4.3.x | **none at all** | **13 scales** (TAI, TT, UTC, GPST, GST, BDT, …) | **IERS table, modelled** | yes |

`jiff` wins the civil layer on one specific API: `Disambiguation::Reject` turns an ambiguous
or nonexistent local time into a hard typed error rather than a silent guess. That is
Principle II's "never silently guessed" expressed directly in the type system, not a
convention we would have to enforce by review.

`hifitime` wins the scale layer outright — it is the only surveyed crate with a real
IERS-sourced leap-second table and a first-class `TimeScale` on every `Epoch`. It can also
represent a genuine `23:59:60` leap second, which `jiff` silently clamps to `:59`.

**Alternatives rejected**:
- `chrono` + `chrono-tz` — credible second choice; `MappedLocalTime::{Single, Ambiguous, None}`
  is arguably the cleanest value-type answer to DST ambiguity. Rejected because `chrono-tz`'s
  release cadence had gone over a year stale at the time of survey, which is a direct risk to
  Principle III (tzdata must be pinnable and current).
- `time` alone — no IANA database or DST engine in core at all; satisfying the DST requirement
  means bolting on the third-party `time-tz`, which is exactly the unvetted assembly this
  audit exists to avoid.
- `hifitime` alone — necessary but not sufficient. It literally cannot answer "what phase is
  Shanghai in", having no civil-timezone concept.

### D1a — The seam is real and must be documented, not papered over

Once an instant crosses from `hifitime`'s `Epoch` into `jiff`'s `Timestamp`, it is
leap-second-naive again. This is an **architectural seam**, not a bug, and the design must
name it:

- Ingest converts a tagged non-UTC instant to UTC through `hifitime`, leap-second-aware.
- Everything downstream of that conversion operates on UTC nanoseconds in `jiff`.
- An instant *during* a historical leap second cannot round-trip through the civil layer.
  For a trading-session product this is acceptable — no venue phase boundary has ever fallen
  inside a leap second — but it MUST be stated, and it MUST be a golden vector that asserts
  the documented behaviour rather than an untested assumption.

---

## D2 — tzdata version reporting has no clean answer in `jiff`

**Decision**: pin the bundled database (`tzdb-bundle-always`) and carry the IANA release
identifier as our own recorded fact, not as something read from the library.

**Rationale**: Principle III requires every build to *report* the tzdata release it ran
against. The survey found:

- `jiff` exposes `TimeZoneDatabase::available()` and `is_definitively_empty()` but **no version
  method**. `jiff-tzdb` has no public version constant either. The release is knowable only by
  mapping the pinned `jiff-tzdb` crate version through jiff's own changelog.
- `chrono-tz` *does* export `IANA_TZDB_VERSION` — a clean runtime constant, on the crate we
  rejected for staleness.
- The independent `tzdb` crate (Kijewski) exports both `VERSION` and `VERSION_HASH` — a release
  identifier *and* a content hash. This is the closest thing to what Principle III asks for.

**Consequence**: default `jiff` behaviour on Unix reads the OS `/usr/share/zoneinfo` at runtime,
which is unpinned and would silently break reproducibility. We must therefore enable
`tzdb-bundle-always` so the database is compiled in and deterministic, and record the release
identifier in our own dataset-revision metadata.

**Open option, not decided here**: adopting the `tzdb` crate purely as an independent audit
cross-check for `VERSION_HASH`. Deferred — it adds a dependency for an assurance we can get
more cheaply by pinning. Revisit if reproducibility ever needs proving to a third party.

### D2a — Correction: `jiff-tzdb` does export a public version constant (T003, 2026-07-30)

**D2 above is wrong on its central claim** and the correction is recorded rather than the
original quietly edited, because the wrong version shaped a task.

The Phase 0 survey concluded that `jiff-tzdb` "only exposes `available()`/`get()` functions — no
public VERSION constant", and T003 was written to hand-maintain the release identifier against
jiff's changelog. Reading the vendored source during implementation showed otherwise:

```rust
// jiff-tzdb-0.1.8/lib.rs
pub static VERSION: Option<&str> = tzname::VERSION;
// jiff-tzdb-0.1.8/tzname.rs
pub(super) static VERSION: Option<&str> = Some(r"2026c");
```

`tzname::VERSION` is `pub(super)`, which is presumably what the survey saw; `lib.rs` re-exports
it publicly.

**Consequence — a strictly better implementation.** `market-time-core` takes `jiff-tzdb` as a
direct dependency and reads the release at runtime instead of carrying a transcribed constant. A
hand-copied provenance claim that drifts out of step with its dependency is worse than no claim,
because it is wrong with confidence.

**Verified at runtime, not inferred**: the pin test prints `IANA tzdb release in this build:
2026c`, and asserts both that the bundled database is populated (catching the case where
`tzdb-bundle-always` is not actually active and the host's unpinned zoneinfo is answering) and
that the release identifier is reportable.

The general lesson, worth keeping: a documentation survey establishes what is *documented*.
Reading the source establishes what is *there*. For a load-bearing claim, the second is what
counts.

---

## D3 — NYSE does not randomise its open. It has no single open instant at all.

**This finding contradicts an assumption in the spec and the spec has been corrected.**

The spec's edge-case list carried "randomised open" as a launch-venue concern. First-party
research shows NYSE does something different and, for our model, harder:

- The opening is a **rolling, DMM-driven process, security by security**. Securities within
  10% of their reference price open algorithmically; others are opened manually by the
  Designated Market Maker.
- The instant a given security opens is therefore **not simultaneous across the market** and is
  **not published in advance**.
- NYSE publishes a *process*, not a fixed instant. It does not describe randomisation.

**Consequence for the model**: "NYSE opened at 09:30:00" is a statement about the *scheduled
start of the opening process*, not about any security being tradable at that instant. This is
exactly the published-versus-observed distinction Principle II already forbids conflating
(FR-012) — and it turns out the launch set contains a live example, not a hypothetical one.

Venue-level phase remains the right abstraction for this slice (the board answers "is NYSE
open"), but the uncertainty attached to the `continuous_trading` start boundary MUST reflect
that per-security opening is a process with a spread, not an instant. Randomised opens do
exist at other venues (LSE), but none are in the launch set, so that edge case moves out of
this slice's vector list and into a note for future venue onboarding.

---

## D4 — SSE has no shortened sessions, and that is structural

**Finding**: SSE Trading Rules (2026 Revision, effective 2026-07-06) contain no early-close
concept. Article 2.4.3: *"Where the market is closed for any reason during trading hours, the
trading hours shall not be extended."* Mainland Chinese exchanges are fully open or fully
closed for the day.

**Consequence**: the shortened-session code path is exercised by NYSE only in this slice. That
is fine, but it means SSE cannot serve as a second test case for it — the golden vector set
must not create a false sense of coverage. Recorded so a later reviewer does not read
"two equities venues" as "two half-day test cases".

**Published SSE schedule** (Beijing time, UTC+8, no DST), from Trading Rules Art. 2.4.2:

| Phase | Time |
|---|---|
| opening call auction | 09:15 – 09:25 |
| continuous trading | 09:30 – 11:30 |
| mid-day break | 11:30 – 13:00 |
| continuous trading | 13:00 – 14:57 |
| closing call auction | 14:57 – 15:00 |
| after-hours fixed price | 15:05 – 15:30 |

The after-hours fixed-price window (Art. 3.7.2) is a real published phase and maps to
`post_close` in the shared vocabulary.

### D4a — The captured SSE table does not tile all time. Two gaps, both real.

**Found during Phase 1 design, independently by two agents working from this table.** The
schedule as captured above leaves two intervals with no assigned phase:

- **09:25 – 09:30** — between the end of the opening call auction and the start of continuous
  trading.
- **15:00 – 15:05** — between the end of the closing call auction and the start of the
  after-hours fixed-price window.

FR-008 requires phases to tile all covered time with no gaps. These two intervals therefore
either belong to a phase the capture missed, or SSE publishes nothing about them and the
correct answer is a phase we must justify from the Rules rather than assume.

**They MUST NOT be filled by inference.** The obvious guesses (`pre_open` and `post_close`
respectively) are plausible and unsourced, and Principle I forbids presenting a derived value
as observed. Both intervals are **carried forward as blocking verification items** against the
SSE Trading Rules (2026 Revision) before any SSE dataset revision is built.

Worth recording plainly: the tiling invariant found a hole in our own captured evidence before
a line of code existed. That is the invariant working, not a setback.

### D4b — Both intervals resolved from source (T021, 2026-07-30)

Verified against the Trading Rules PDF, full 61-page read. **Neither interval was filled by
inference.**

**15:00 – 15:05 — addressed by the Rules.** Three articles construct it affirmatively:

> Art. 3.7.9: "Closing price orders submitted between 9:30 to 15:05 shall not be included into
> real time quotations; those submitted and executed during the after-hours fixed-price trading
> session, which is between 15:05 to 15:30, shall be included into the real time quotations."

> Art. 3.7.3: "The Exchange will accept the closing price orders from trading participants from
> 9:30 to 11:30 and from 13:00 to 15:30 on each trading day. … During the sessions for accepting
> members' order routing, any unexecuted orders may be canceled."

> Art. 3.7.2: "The after-hours fixed-price trading session is 15:05-15:30 on each trading day."

Read together: during 15:00–15:05 after-hours orders **are accepted and cancellable, are not
matched, and are not in real-time quotations**. Order acceptance without matching, ahead of a
session that starts at 15:05. That is the definition of `pre_open` in our vocabulary.

**Mapping: `pre_open`.** Sourced, not guessed.

**09:25 – 09:30 — defined by exclusion, not by assertion.** The Rules never say "no orders are
accepted between 9:25 and 9:30". What they do is draw the boundary at 9:25/9:30 in three
independent provisions, none of which includes the interval:

> Art. 2.4.2: opening call auction 9:15–9:25; continuous auction 9:30–11:30 and 13:00–14:57.

> Art. 3.3.1: "The Exchange accepts auction trading orders from trading participants during the
> following periods on each trading day: 9:15 to 9:25, 9:30 to 11:30, and 13:00 to 15:00."

> Art. 5.2.1: real-time quotations cover 9:15–9:25 and 14:57–15:00.

So: **no order acceptance, no matching, no quotations.** Three separate provisions agreeing is
stronger than silence, but it is still exclusion rather than assertion, and the distinction is
recorded rather than smoothed over.

Corroborating that the drafters name sub-states when they mean to — Art. 3.3.1 also carves out
a no-cancellation sub-window *inside* the opening auction ("will not accept any order
cancellation requests during the opening auction between 9:20-9:25"). They had the vocabulary
and did not use it for 9:25–9:30.

**Mapping: `closed`** — see D4c for the reasoning, which is a modelling decision rather than a
source fact and is marked as ours.

### D4c — Modelling decision: 09:25–09:30 maps to `closed`

Ours to decide, so recorded as ours. During the interval the venue accepts nothing, matches
nothing, and publishes nothing. Three candidate mappings:

| Option | Verdict |
|---|---|
| `closed` | **Chosen.** "Closed" means the venue is neither accepting nor matching. It need not mean "outside the trading day" |
| `non_trading_interruption` | Rejected — that kind is for halts and maintenance, i.e. exceptions. This is a scheduled, recurring, daily interval |
| a ninth phase kind | Rejected — it would exist to serve one venue's five minutes, which is the venue-specific-vocabulary failure FR-005 forbids |

Marked `is_derived` per FR-010: the interval's *content* is sourced by exclusion, its *phase
name* is our mapping.

### D4d — The captured SSE day was also too short. Block trading runs to 17:00.

**Found during T021, not previously in scope.** Art. 3.6.3:

> "The Exchange accepts block trade orders during the following periods on each trading day:
> (1) Intent orders: 9:30-11:30 and 13:00-15:30; (2) Execution orders: 9:30-11:30, 13:00-15:30,
> and 16:00-17:00; and (3) Fixed-price orders: 15:00-15:30."

Art. 3.6.4: "Trades confirmed during the period from 16:00 to 17:00 on each trading day shall be
cleared and settled on the next trading day."

So after the after-hours session closes at 15:30 there is a gap to 16:00, then a further hour
where block-trade **execution orders only** are accepted and confirmed, settling next day.

**This was a scope question, not a mapping question. Resolved 2026-07-30 — see D4e.**

### D4e — A venue entity is one matching mechanism, not one legal exchange

**Decision**: SSE block trading is a **separate venue entity** from the SSE auction market.
Generalised into a rule, because this class of question will recur on every venue onboarded:

> **One venue entity per matching mechanism, not per legal exchange.**
> If two order flows at the same exchange have different schedules, different order-acceptance
> windows, or different matching rules, they are two venue entities.

**Rationale**: the alternative — one entity carrying several concurrent mechanism timelines —
breaks the single-timeline assumption that makes `PhaseTimeline`'s tiling invariant enforceable.
Once a venue may hold N overlapping timelines, "phases tile all covered time with no gaps or
overlaps" (FR-008) stops being checkable by a constructor and degrades into a convention. The
invariant is load-bearing, so the model bends first.

**What it resolves beyond SSE:**

- Binance spot and USD-M perpetual futures are two venue entities, not one venue with a flag.
  The slice-1 decision to cover perpetuals only is therefore a *scope* choice, not a modelling
  compromise.
- NYSE and NYSE Arca are separate entities, which matters for the Arca overnight session
  (9:00 PM–4:00 AM, pending SEC approval, targeted 2026-12-06) that would otherwise appear to
  contradict NYSE's published hours.

**Cost, stated honestly**: venue identity gets more granular, so a consumer asking "is Shanghai
open" must name which mechanism. That is a genuine ergonomic cost. It is accepted because the
alternative hides a real distinction behind a friendly-looking answer, which is exactly what this
product exists not to do. A convenience grouping over related entities can be added later at the
shell layer, where it belongs — and where it cannot corrupt the invariant.

**Slice-1 scope impact: none.** Block trading is not in the launch set. The model must merely not
preclude it, and this decision ensures it does not. `SSE` in slice 1 means the auction market
only, and the coverage declaration says so.

---

## D5 — Binance's funding interval is itself versioned rule data

**Finding**: the funding schedule is not a constant. From Binance's own FAQ and announcements:

- Default: every 8 hours at 00:00, 08:00, 16:00 UTC.
- Since 2023-10-12: a **named subset** of contracts settles every 4 hours.
- Since 2025-05-02: a contract can shift to **hourly** settlement when its funding rate hits
  the cap or floor.
- Reversion rules have themselves changed: 2025-09-01 set ≤|0.002%| for 36 consecutive cycles;
  2026-01-02 changed it to ≤|0.025%| for 16 cycles.

**Consequence**: the funding schedule is per-contract, state-dependent, and amended by
announcement over time. It is exactly the kind of thing Principle III exists for — it must be
versioned rule data with `effective_from`, never a constant in code. A naive "funding is every
8 hours" would be wrong for a named contract subset and wrong for any contract in a volatility
regime.

This also validates the phase-versus-event split (FR-007) against a real case: funding is an
event with a variable recurrence rule, not a phase.

## D5a — Binance publishes its own imprecision

**Finding**, verbatim from Binance's FAQ: *"There is a 15-second deviation in the actual funding
fee transaction time."*

**Consequence**: this is a published uncertainty bound handed to us by the source. It feeds
FR-011 directly and is the cleanest possible demonstration that nanosecond representation is
not a nanosecond accuracy claim — the venue itself states ±15 s. A golden vector MUST assert
that a funding event's stated uncertainty is no narrower than this.

**Maintenance** is announced per-incident on Binance's announcement pages, with a "Notice of…"
pre-announcement and a "…Complete" follow-up, scoped to a product line (e.g. COIN-M only, with
USDⓈ-M explicitly unaffected). Announcements give minute-level start times in UTC. This is the
unstructured-source class the constitution's Domain and Data Constraints already anticipated.

---

## D6 — Licensing: one hard gate, two unresolved

This is the highest-risk finding in Phase 0 and it constrains what can ship.

**SSE — hard gate, resolved and blocking.** Trading Rules Art. 5.1.3, verbatim:

> "The Exchange owns the trading information generated from the markets of the Exchange.
> Without the permission of the Exchange, no entity or individual may use or publish such
> information."

The SSE legal statement adds that without written permission no institution or individual may
use website content for profit-making purposes, and prohibits scraping, storage, and
redistribution. Per `DATA-LICENSING.md`, SSE data therefore falls in the
**non-redistributable** tier: the source is registered with its evidence record and its
schedule is referenced, but **the dataset is not vendored into this repository**.

**NYSE — resolved (T022, 2026-07-30). Also non-redistributable.**

The governing document is ICE's Terms of Use (`ice.com/terms-of-use`), not the previously found
`TOP_Terms_of_Use.pdf`, which is scoped to NYSE's ops portals. Confidence is high on three
independent signals: the ICE document's own scope clause names `www.nyse.com` explicitly; the
footer of `nyse.com/trade/hours-calendars` links "Terms of Use" straight to ICE; and
`nyse.com/terms`, `/legal`, and `/privacy` all return 404 — NYSE carries no site-level terms of
its own.

> "You acknowledge and agree that, unless ICE … give you prior written permission, you will not
> … sell, license, rent, modify, print, collect, copy, reproduce, download, upload, transmit,
> disclose, distribute, disseminate, publicly display, publicly perform, publish, edit, adapt,
> electronically extract or scrub, compile or create derivative works from any content or
> materials (including, without limitation, through framing or **systematic retrieval to create
> collections, compilations, databases or directories**)"

> "…a limited license to access and use this Website and to download and print copies of any
> content … but only for your own **personal, non-commercial use** … The foregoing license does
> not include use of any **data mining, robots or similar data gathering or extraction
> methods**."

Those emphasised phrases describe building a calendar dataset almost exactly. **No carve-out for
factual or schedule data was found**, and the Proprietary Rights clause bundles "all compilations
of real time or other information" without distinguishing a holiday calendar from a price feed.
`Trading_Days.pdf` carries no document-level notice of its own, so it inherits the hosting page's
terms.

**Binance — resolved (T023, 2026-07-30). Also restricted, and the retrieval is worth recording.**

The earlier failure had the right conclusion and the wrong reason. `binance.com/en/terms` is not
a slow SPA — **the legal text is not in the DOM at all.** It is a 73-page PDF fetched by
background XHR into a pdf.js viewer after an AWS WAF challenge completes. No plain fetch can ever
see it. It was obtained by reading the browser's own network log to find the PDF URL, then
extracting text through pdf.js's `getTextContent()` API.

**A trap worth naming**: `curl` against that same PDF URL returns HTTP 200 and an **8-page**
document; the browser's pdf.js reports **73 pages** for the identical URL. The resource silently
serves truncated content without the WAF session token. The curl copy was discarded rather than
quoted.

The document is an ADGM financial-services account agreement, not a website terms-of-use. A
full-text search of the 208,000 extracted characters for the usual boilerplate — scrape, copy,
reproduce, redistribute, resell, aggregate, frame — returned **zero matches**. The only clause
governing reuse is the IP licence:

> Clause 27: "We grant to you a non-exclusive licence for the duration of the Agreement … to use
> the Binance IP, excluding the Trade Marks, solely as necessary to allow you to receive the
> Binance Services for **non-commercial personal or internal business use**, in accordance with
> the Agreement."

Also confirmed: `developers.binance.com/.../PROD-TERMS-OF-USE` is a one-paragraph stub pointing
back to the same document, and the USD-M futures docs carry no separate terms. All three entry
points funnel to one legal document.

**Two caveats recorded rather than smoothed**: the served variant is entity- and
geography-dependent (ADGM entities, selected by egress IP), so other jurisdictions may see
different clauses; and "Funding Fee" is defined by reference to separate Clearing Rules that were
not reachable from these URLs and remain unfetched.

### D6a — All three launch venues are non-redistributable. This is a product constraint.

| Venue | Governing text | Position |
|---|---|---|
| SSE | Trading Rules Art. 5.1.3 + legal statement | Use or publication requires Exchange permission |
| NYSE | ICE Terms of Use | Personal, non-commercial only; systematic retrieval into a database explicitly named |
| Binance | ADGM Terms cl. 27 | Non-commercial personal or internal business use only |

Not one of the three permits commercial redistribution of its published schedule, and not one
carves out factual or calendar data.

**Consequences, stated plainly:**

1. **No venue dataset may be vendored into this repository.** All three fall in the
   non-redistributable tier of `DATA-LICENSING.md`. Fetch-at-run-time is the only compliant
   shape, which makes `market-time-data` the entire ingestion architecture rather than a
   convenience.
2. **What Mark Time can distribute is its own work**: the phase model, the venue-to-vocabulary
   mappings, the schema, the evidence structure, the code. Rule data is referenced and fetched,
   never shipped.
3. **There is a product question above the engineering one.** "Internal business use" covers a
   team running this for itself. Distributing Mark Time as a product that fetches these venues'
   data on a user's behalf is a materially different question. It belongs to the project owner,
   not to a spec. **Escalated, not assumed.**

**Separately confirmed**: a well-known Chinese financial aggregator's published terms forbid
reuse of its data for AI training or commercial purposes. Aggregators are design references
for *what to display*, never sources for *what is true*. Already recorded in the spec's
Assumptions and in `AGENTS.md`.

---

## D7 — Machine-readable access is thin, and that shapes ingestion

| Venue | Schedule format | Calendar format | Official API |
|---|---|---|---|
| SSE | PDF (Trading Rules) | HTML tables + HTML announcements | none found |
| NYSE | HTML + PDF fact sheets | HTML table + `Trading_Days.pdf` | none public for calendars |
| Binance | FAQ page | n/a (no non-trading days) | public REST (`fapi.binance.com`) |

**Consequence**: two of three venues are HTML/PDF only. Ingestion is therefore document capture
plus human-reviewed rule entry for SSE and NYSE, which the constitution already admits as
evidence provided it records who entered the rule, from which document, and when. This is not a
shortcut around Principle I — it is the path Principle I explicitly allows.

---

## Carried into Phase 1

Open items that must not be silently dropped:

1. Fetch and record `ice.com/terms-of-use` (NYSE governing terms) before any NYSE ingestion.
2. Re-fetch Binance terms with a JS-capable browser; record verbatim before any Binance ingestion.
3. Verify the exact NYSE early-close footnote text byte-for-byte (substance is established:
   13:00 close, day after Thanksgiving and Christmas Eve; exact wording needs one more pass).
4. Confirm `jiff` `Disambiguation::Reject` behaviour against a real spring-forward and
   fall-back instant as a golden vector, rather than trusting the documentation.

## Process note

One research agent reported that a tool call returned content from an unrelated workstream
that had leaked through a shared search index. The agent identified it as irrelevant and
disregarded it. No finding above rests on it. Recorded because a shared index that can serve
foreign content into a research pass is a real contamination channel for an evidence-based
product, and it should be a known risk rather than a surprise later.
