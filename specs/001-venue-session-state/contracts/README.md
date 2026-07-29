# Contracts: Venue Session State

**Feature**: `001-venue-session-state` | **Constitution**: v1.2.0

## What these are

Three interface contracts, one per crate boundary in `plan.md`'s Project Structure:

| File | Crate boundary | What it fixes |
|---|---|---|
| `core-api.md` | `market-time-core` | The pure library's public surface — the only thing any shell may call to get a phase answer. |
| `cli.md` | `market-time-cli` | The command-line shell's surface — a thin caller of core-api, nothing more. |
| `board.md` | `market-time-board` | The global board's contract with core-api — a thin renderer, nothing more. |

`market-time-data` (loaders) and `market-time-scales` (the `jiff`/`hifitime` seam, research D1a) are
not covered here. They are ingestion- and conversion-side boundaries, not decision-surface
contracts — their job is to produce the values these contracts consume (a materialized `Ruleset`,
a `UtcInstant`), not to expose a query surface of their own.

These are pre-implementation interface contracts: shapes, signatures, and invariants, not
tutorials and not implementations. Each file states what would constitute a violation of it.

## Rules that bind all three

These are cross-cutting constraints from the constitution and the spec. No individual contract
below restates them in full; each defers to this list and states only its own specific
consequence of it.

1. **`now` is a parameter everywhere, never an internal read, in core.** Core never contains a
   clock read in its decision path (Principle IV). A shell *may* read the host clock — that is
   how a shell obtains "now" at all — but reading the clock and deciding a phase are two
   different steps done by different code, and the boundary between them is exactly the
   function call into core-api.
2. **A shell surface presenting "now" to a human MUST expose the host's clock discipline bounds
   as uncertainty**, not present the host clock as exact (constitution, Domain and Data
   Constraints). This binds any shell that displays "now" — the constitution's clause is not
   scoped to the board alone, and `cli.md` states this explicitly for that reason.
3. **Unknown is a data outcome, not an error and not "closed."** Every interface that can return
   a phase must be able to return unknown, and must be able to do so for one item in a
   multi-venue request without discarding the others (FR-002, FR-019, FR-020).
4. **Evidence and uncertainty are mandatory, not optional-for-tidiness.** No contract in this
   directory permits an interface to drop either field to make output shorter or cleaner
   (FR-009, FR-011; spec's "if a display cannot express an honest answer, the display changes").
5. **One phase vocabulary, a closed set owned by the core.** No shell, no board, and no venue
   introduces a phase name of its own (FR-005, FR-006).
6. **Every answer is attributable to the immutable dataset revisions that produced it**
   (Principle III, FR-016, FR-017). A contract that lets an answer go out without its revision
   set is non-conforming regardless of which shell it came from.
7. **Domain logic lives only in core.** Shells format, transmit, and render; they do not decide
   (Principle IV). Each shell contract states concretely what this forbids for that shell.

A change to any of these seven rules is a constitutional or spec change, not a contract change —
if a contract below appears to relax one, that is a bug in the contract, not a sanctioned
exception.
