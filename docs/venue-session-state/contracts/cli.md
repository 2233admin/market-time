# Contract: `market-time-cli` command surface

Binding principle (Constitution IV): the CLI is a thin shell. It parses arguments, obtains an
instant, calls `core-api.md`'s functions, and formats what comes back. It holds no domain logic —
it does not decide phases, does not compute boundaries, and does not interpret coverage; it only
calls and renders. See `README.md` for the rules shared across all three contracts.

## Commands

| Command | Purpose | Core call |
|---|---|---|
| `market-time phase --venue <ID> --at <INSTANT>` | Single-venue phase query | `resolve_phase` |
| `market-time phases [--venues <ID,ID,...>] --at <INSTANT>` | Multi-venue query; defaults to all tracked venues | `resolve_phases` |
| `market-time venues` | List tracked venues and each one's declared coverage | none (reads `Ruleset` metadata only) |
| `market-time revisions` | List the dataset revisions the loaded `Ruleset` reports | none (reads `Ruleset` metadata only) |

Evidence and uncertainty are never behind a separate command or a flag — they are fields on the
`phase`/`phases` output, always present (see README rule 4). There is no "quiet" output mode that
omits them; `--format` (below) changes encoding, not content.

## `--at <INSTANT>` — where "now" comes from

`<INSTANT>` accepts either an explicit value or the literal `now`:

- **Explicit**: an RFC 3339 UTC instant (`2026-11-27T14:30:00.000000000Z`), or a civil
  local time plus `--zone <IANA_ZONE>` for cases that need to state a query in venue-local terms.
  A civil-time input that is ambiguous or nonexistent in that zone (DST fall-back / spring-forward)
  is rejected with a typed error (§ Exit codes) — the CLI does not guess, mirroring `jiff`'s
  `Disambiguation::Reject` (research D1).
- **`now`**: the CLI reads the host clock itself, once, converts it to a `UtcInstant`, and passes
  that value into `resolve_phase`/`resolve_phases` as an ordinary argument. This is the same
  pattern as any other `--at` value — the CLI reads the clock, core never does (README rule 1).

**Judgment call — clock discipline applies to the CLI, not only the board.** The constitution's
"where a shell surface reports 'now', it MUST source the host's own discipline bounds ... and
expose them as uncertainty" is written as a shell-surface rule, not a board-specific one
(`board.md` states the same obligation for the same reason). When `--at now` is used, CLI output
MUST carry the host clock's discipline bounds (offset and dispersion, sourced the same way the
board does) alongside the resolved instant — never a bare "now" presented as exact. This applies
only to the `now` path; an explicit `--at` value carries no clock-discipline uncertainty because
no host clock was consulted to produce it.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | A `PhaseOutcome` was produced and printed — `Known` **or** `Unknown`. Both are successful outcomes (README rule 3); unknown is not a failure. |
| `2` | Usage error: bad arguments, unrecognized flag, missing required argument. |
| `3` | Ambiguous or nonexistent local instant under `--zone` (DST typed error, ⁠§ above). Distinct from `2` so scripts can detect DST-input problems specifically rather than parsing text. |
| `4` | Ruleset load failure (`market-time-data` could not materialize the dataset revisions the CLI needs). Not a core error — core was never reached. |
| `1` | Any other failure. Reserved, not used for the above. |

There is no exit code for "venue outside coverage" or "venue unknown to the ruleset" — those are
`Unknown` outcomes rendered under exit code `0` (§ Output shapes), per `core-api.md` §4's
unknown-vs-error split.

## Output shapes

Two encodings of the same `PhaseOutcome` / `Vec<VenueOutcome>` values — `--format` changes
presentation, not which fields exist.

**`--format json`** (default for non-interactive use): a direct, stable serialization of the
core types in `core-api.md` — `phase`, `boundary_start`/`boundary_end` each with their own
`uncertainty`, `evidence`, `dataset_revisions`, and for an unknown outcome, a `coverage_gap`
object naming the venue, the queried instant, and the coverage boundary that was crossed.
`phases` output is an array of these, one per requested venue, in request order, unconditionally
— an unknown entry never causes the array to shrink or reorder (FR-020).

**`--format text`** (default for a terminal): human-readable, but every field the JSON form
carries has a rendered counterpart. Concretely, text output MUST NOT drop:

- the uncertainty qualifier on a phase boundary (e.g. a boundary derived from a process-start
  rather than an instant renders with a spread note, not a bare clock time — FR-011b);
- at least one evidence reference per answer (FR-009), even if abbreviated to source + date;
- an unknown venue's entry, rendered distinctly from "closed" and naming the coverage boundary
  it fell outside of, in a multi-venue listing that otherwise renders normally (FR-020).

An abbreviation that collapses "boundary with published-to-the-second uncertainty" and "boundary
with nanosecond-exact uncertainty" into the same text line is a contract violation — the whole
reason uncertainty is a field, not a footnote, is so it survives being made human-readable.

## Violations

The CLI is non-conforming if it: computes a phase, boundary, or coverage decision itself instead
of calling core; adds a way to suppress evidence or uncertainty from output; treats `Unknown` as
an error (nonzero exit) or as `Closed`; lets one venue's coverage gap in `phases` remove or
reorder other venues' entries; or presents `--at now` without attaching host clock-discipline
uncertainty.
