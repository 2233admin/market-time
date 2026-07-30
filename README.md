# Mark Time

Open-source, versioned and auditable time infrastructure for global financial markets.

## What it is

Mark Time answers one question with auditable precision: what time is it, and what is
open, where. It serves global city clocks, exchange trading sessions and phases, and
crypto funding and maintenance windows.

What sets it apart from an ordinary time/calendar library:

- **Every answer is traceable to dated source evidence.** A session boundary, a holiday,
  a funding window — each one points back to the document it came from and when that
  document was read.
- **Instants are nanosecond-precision and unambiguous about their frame** — an absolute
  epoch instant, or a civil time bound to a named zone, never an implicit mixture of the
  two.
- **Uncertainty is stated, not guessed.** Where the data only supports a partial or
  provisional answer, Mark Time says so. Outside its verified coverage, it returns an
  explicit unknown rather than extrapolating a schedule.

## Who it is for

Machines first. Mark Time is infrastructure for **autonomous agents that trade**, and for other
systems that need to know what is open, where, and how far that answer can be trusted. People
reading the board are a legitimate second audience, not the design centre.

That ordering has a concrete consequence rather than being a slogan. A person looking at a board
can see that a number is fuzzy. **An agent cannot.** So uncertainty and unknown are not
presentation concerns here — they are part of the returned value, and the type system makes them
impossible to skip past. An agent that never inspects an answer's uncertainty still cannot
mistake an unknown for a closed market, because the two are different values rather than the same
value rendered differently. Anything a human would infer from how a screen looks has to survive
as data.

## Status

Pre-alpha, and it runs. The phase engine, the timeline query, the dataset loader, the CLI, and
the board are implemented and tested; what is not here, and never will be, is venue data. The
specification of this slice is in
[`openspec/changes/venue-session-state/`](openspec/changes/venue-session-state/); the research,
data model, and interface contracts behind it are in
[`docs/venue-session-state/`](docs/venue-session-state/).

## Try it

The repository ships a **synthetic** dataset — three invented venues that exercise the three
structural cases (auctions plus a mid-day break, a daylight-saving zone whose open is a process,
an always-on venue with scheduled events). It is not any real venue's calendar, and it exists so
the tool can be run without data you may not be allowed to have.

```bash
cargo run -p market-time-cli -- board --dataset crates/market-time-data/fixtures/synthetic-venues.json --zone Asia/Shanghai --at 2026-07-30T02:00:00Z
```

```text
               10:00       14:00       18:00       22:00       02:00       06:00
SYNTH-ALWAYS  [|#######################################################################]  continuous_trading
SYNTH-AUCT    [|####::::######__.....................................................-#]  continuous_trading
SYNTH-DST     [|.................-----------------###################____________......]  closed

  axis: Asia/Shanghai (72 columns)
  now:  2026-07-30 10:00:00 — instant supplied, not read from a clock (--at on the command line)
  key:  # trading  = auction  - pre-open  : break  _ post-close  . closed  ! halt  ? not known
```

`market-time phase` answers one venue with its evidence, its dataset revisions, and the
uncertainty on each boundary. `market-time timeline` prints the segments a board row is drawn
from, including the stretches that fall outside coverage — those come back as "not known", never
as a schedule someone extrapolated.

To answer for a real venue, assemble a dataset revision from that venue's own published schedule
under your own relationship with it, and point `--dataset` at it. See
[`DATA-LICENSING.md`](DATA-LICENSING.md) — this repository is a client, not a redistributor.

The board that slice ships is a timeline: one row per venue, that venue's phases laid out across
the day on a shared axis, a marker on the instant you are looking at. That is the shape global
trading-hours boards already established. What it adds is the layer they leave out — every
segment reaches its source document, and an out-of-coverage stretch reads as "not known" rather
than being quietly drawn as closed. Coverage beyond the three launch venues is specified
separately in
[`openspec/changes/global-market-coverage/`](openspec/changes/global-market-coverage/) and is
deliberately not part of the first slice.

Work proceeds through [OpenSpec](https://github.com/Fission-AI/OpenSpec): a change proposes
requirement deltas, the deltas get implemented, and archiving the change folds them into
`openspec/specs/` as current truth.

## Principles

Mark Time is governed by a ratified constitution. Three principles in particular
constrain how consumers may trust its output:

- **Evidence-Backed Rules.** Every rule that shapes an answer — a holiday, a session
  boundary, a DST transition, a funding interval — is stored with the evidence that
  justifies it (source, fetch time, effective date). Nothing derived is presented as
  observed.
- **Explicit Instants, Explicit Uncertainty.** Instants are nanosecond-precision and never
  ambiguous about their frame or their time scale — UTC is the published scale, and input
  on another scale (TAI, GNSS system time) is tagged and converted leap-second-aware, never
  by a hardcoded offset. Nanosecond representation is an arithmetic guarantee, not an
  accuracy claim: where a venue publishes a boundary to the second or deliberately
  randomises it, the answer's uncertainty says so. Queries outside known coverage return an
  explicit unknown rather than a guess.
- **Reproducible Rule Data.** Time-zone data, exchange calendars, and venue schedules ship
  as immutable, versioned dataset revisions — a correction produces a new revision, never
  an edit in place. Every build reports the revisions it runs against, so a past answer can
  be replayed against the rule set that was actually in force when it was given.

See [`CONSTITUTION.md`](CONSTITUTION.md) for the full text, including Principles IV and V and
the governance rules.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.

### Data licensing

The dual license above covers **code only**. Rule data — exchange calendars, holiday tables,
session definitions — is sourced from upstream publishers and is not relicensed by this project.
Each record carries its own source and terms. See [DATA-LICENSING.md](DATA-LICENSING.md).

**This is not a formality.** All three launch venues were checked, and none of them permits
commercial redistribution of its published schedule, nor carves out factual or calendar data:

| Venue | Governing text | Position |
|---|---|---|
| SSE | Trading Rules Art. 5.1.3 | use or publication requires Exchange permission |
| NYSE | ICE Terms of Use | personal, non-commercial only; "systematic retrieval to create collections, compilations, databases" is named explicitly |
| Binance | ADGM Terms cl. 27 | non-commercial personal or internal business use only |

So **this repository ships no venue data, and never will.** What is open source here is the
model, the venue-to-vocabulary mappings, the schema, the evidence structure, and the code.
Schedules are fetched at run time by the operator, under the operator's own relationship with
each venue — Mark Time is a client, not a redistributor.

Operators are responsible for their own compliance with each venue's terms. Where a venue offers
a permission path, taking it is the intended route rather than a nicety; SSE states one
explicitly.
