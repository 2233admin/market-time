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

Pre-alpha. The first explicit-UTC vertical slice now runs end to end:

`operator JSON ruleset → market-time-data validation → PhaseTimeline invariant →
resolve_phase() → CLI JSON/text output`.

It supports nanosecond-represented RFC 3339 UTC queries, half-open phase boundaries,
explicit out-of-coverage `unknown`, evidence, uncertainty, immutable dataset revision
attribution, source-terms registration, and the compiled IANA tzdata revision. Civil-time
input, `--at now`, rule precedence, real SSE/NYSE/Binance adapters, multi-venue queries,
events, and the board remain planned work. `--at now` is deliberately rejected until host
clock-discipline bounds can be surfaced honestly.

### Run the vertical slice

The checked-in ruleset is synthetic and contains no venue data:

```powershell
cargo run -p market-time-cli -- phase `
  --ruleset examples/synthetic-ruleset.json `
  --venue X-MT-DEMO `
  --at 1970-01-01T00:00:00.000000010Z `
  --format json
```

The answer is `continuous_trading`, owns the exact 10ns boundary under the half-open
convention, and carries its evidence, uncertainty, `synthetic-r1`, and the compiled IANA
tzdata revision. Querying `...000000030Z`, the exclusive coverage end, succeeds with an
explicit `unknown`.

Work continues through the spec-kit flow: `/speckit-specify` → `/speckit-plan` →
`/speckit-tasks` → `/speckit-implement`. Real venue data is fetched by operators at runtime
under their own source relationships and is never redistributed from this repository.

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

See [`.specify/memory/constitution.md`](.specify/memory/constitution.md) for the full
text, including Principles IV and V and the governance rules.

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
