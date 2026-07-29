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

## Status

Pre-alpha. No code has been released yet. This repository currently contains only
spec-kit workflow scaffolding (`.specify/`) and the ratified project constitution. Work
proceeds through the spec-kit flow: `/speckit-specify` → `/speckit-plan` →
`/speckit-tasks` → `/speckit-implement`. The first planned slice covers SSE, NYSE, and
Binance.

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
- **Versioned Rule Data, Never Hardcoded.** Time-zone data, exchange calendars, and venue
  schedules are versioned data artifacts with pinned identifiers, never literals embedded
  in source code. Every build reports the data versions it was loaded against.

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

The dual license above covers **code only**. Rule data — exchange calendars, holiday
tables, session definitions — is sourced from upstream publishers and is not relicensed
by this project. Each record carries its own source and terms. See
[DATA-LICENSING.md](DATA-LICENSING.md) for details.
