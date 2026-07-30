# Contract: `market-time-board` — the global board's contract with core

Binding principle (Constitution IV): the board is a renderer over `core-api.md`, matching the
shape of a conventional global trading-hours board while carrying evidence and uncertainty all
the way to the surface (spec, Resolved Scope Decisions). It holds no domain logic. See
`README.md` for the rules shared across all three contracts.

## What the board requests

One call per render pass:

```rust
let outcomes: Vec<VenueOutcome> = resolve_phases(now, tracked_venues, &ruleset);
```

- `now` is obtained exactly once per render, by the board reading the host clock and converting
  it to a `UtcInstant` — the same read-then-pass pattern as the CLI (`cli.md`), never a clock
  read inside core. Every venue tile in a single render is computed from that one `now` value, so
  the board's "point in time" is coherent across venues, not a composite of moments read
  microseconds apart (`core-api.md` §5).
- `tracked_venues` is the full launch set; the board does not omit a venue because it expects
  trouble with it — coverage gaps are rendered (§ below), not filtered out beforehand.

## What the board renders, per venue

For each `VenueOutcome`:

- **`Known`**: the phase name (shared vocabulary only — the board has no label of its own for a
  phase), the start and end boundary instants each with their carried uncertainty, and a visible
  evidence reference (at minimum a source indicator a viewer can follow to `cli.md`'s or a
  detail view's full evidence — the board is not required to inline the entire evidence record
  on the tile, but it MUST NOT omit access to it entirely).
- **`Unknown`**: rendered as its own distinct visual state — not blank, not styled as `Closed` —
  naming the coverage boundary that was crossed, in the same tile position the venue would
  otherwise occupy (§ Unknown rendering below).

## The clock-discipline rule

This is the constitution's Domain and Data Constraints clause, binding here by name: **a surface
that reports "now" MUST source the host's own clock discipline bounds (e.g. the offset and
dispersion an NTP or PTP daemon already reports) and expose them as uncertainty, rather than
presenting the host clock as exact.** Wide-area time sync does not reach nanoseconds, and no
surface may imply that it does.

Concretely, whatever the board displays as "as of `<time>`" MUST carry a discipline-bounds
annotation sourced from the host (e.g. "± dispersion reported by the host's time sync"), not a
bare timestamp. If the host exposes no discipline information at all (no reachable NTP/PTP
daemon), the board MUST NOT fall back to displaying the clock as exact — it must render an
explicit "clock discipline unavailable" state instead, itself a form of honest uncertainty about
the "now" being shown, not silence about it.

This bound is a property of the *board's own act of displaying "now"* — it is not part of any
individual venue's `Uncertainty` value and does not come from `core-api.md`. Core never reads a
clock and so has no discipline bounds to report; the board acquires and attaches this
information itself, entirely on its own side of the boundary.

## Unknown rendering

A multi-venue render where one tracked venue is outside coverage at `now` MUST still render every
other venue normally (FR-020, mirrored from `core-api.md` §5's invariant that `resolve_phases`
never suppresses entries). The unknown venue's tile:

- names the venue and states plainly that its state is not known at this instant;
- names the coverage boundary that was crossed (from `CoverageGap`), so a viewer can tell "not
  loaded yet" from "the calendar we have doesn't reach this far" without guessing;
- is visually distinguishable from every `Phase` state, including `Closed` — an unknown venue
  and a closed venue must not be representable by the same visual state, or the board has
  silently turned an honest unknown into a guessed answer by omission.

## Uncertainty, in principle

This is a contract about what must remain visible, not a pixel design (deferred to
implementation/design work outside this spec). The governing rule is the spec's own: *if a
display cannot express an honest answer, the display changes — the answer does not.* Concretely,
that rule forbids the board from:

- rounding a boundary to a clean-looking time and dropping the uncertainty that made it inexact;
- displaying a process-start boundary (e.g. NYSE's opening process, FR-011b) with the same visual
  treatment as an exact boundary — a spread is not an instant, and the board must not make it
  look like one;
- conveying uncertainty by color or icon alone with no text/alt-text equivalent — a viewer who
  can't perceive the color distinction, or a machine reading the rendered page, must still be
  able to recover the uncertainty statement;
- hiding the uncertainty annotation behind an interaction (hover, click) as the *only* way to see
  it — it may be elaborated on interaction, but its presence must be legible in the default view.

**Judgment call**: the spec does not prescribe how uncertainty looks, only that it survives.
Where this contract says "distinguishable" or "legible," it is deliberately not saying "red
badge" or "asterisk" — that choice belongs to design work outside this document. What is fixed
here is that no rendering choice may result in an uncertain or unknown answer being
indistinguishable from a confident one.

## The board holds no domain logic — what that forbids, concretely

The board MUST NOT:

- decide what counts as a coverage gap, a holiday, a shortened session, or a DST boundary — all
  of that is `resolve_phases`'s answer, consumed as-is;
- compute or re-derive a phase boundary, an uncertainty bound, or a coverage range itself (e.g.
  it must not independently compute Binance's ±15s funding deviation — it renders the bound
  `core-api.md` returned, verbatim);
- substitute a default or fallback phase (e.g. rendering `Closed`) when the core returns
  `Unknown` — that substitution is exactly the "guessed answer" Principle II forbids, relocated
  into a shell where review is less likely to catch it;
- cache, store, or accumulate rule data or evidence records of its own between renders — each
  render's tiles are populated only from that render's `resolve_phases` call;
- introduce a phase label, abbreviation, or icon that isn't a rendering of a `Phase` variant from
  `core-api.md` — no venue-specific or board-specific phase vocabulary (README rule 5).

What the board MAY do, because it is presentation and not decision: choose layout, color mapping
from `Phase` variant to a visual style, locale/timezone formatting of an instant for the viewer's
convenience (formatting, not recomputing), and ordering of venue tiles.

## Derived session bands and overlaps

The board may also draw a `BandSection`: the caller's already-derived `SessionBand`s and
`BandOverlap`s (both `market-time-core` types — the board does not derive them itself, the
same "no domain logic" rule that governs everything else here). This is additive, not a
second mode: `BandSection::default()` is empty, and an empty section renders nothing —
no heading, no placeholder row — so a caller that never mentions bands gets exactly the
board this contract already describes.

A band and an overlap are not a venue's published schedule (`market-time-core`'s `bands`
module docs: a band is derived from its members, never itself evidenced). Where the board
draws one, it MUST make that unmistakable:

- every band row and overlap row is labelled "derived," not merely grouped under a heading
  that says so — a reader must not be able to mistake a band row for a venue row above it;
- the glyph and colour vocabulary for a band's or an overlap's state (trading-like / not /
  unknown) MUST NOT be the vocabulary `glyph`/`phase_fill` use for a `Phase` — a band state
  is not a phase, and reusing that vocabulary would let a band row be misread as a venue's
  own published state;
- an `Unknown` band or overlap stretch MUST use the same not-known hatch treatment the
  venue rows use for an out-of-coverage stretch — that visual promise ("an unknown is not
  a closed market") is the product's, not the venue section's alone.

`BandSegment::uncertainty` and `OverlapSegment::uncertainty` are `Option<Uncertainty>`,
`None` exactly when every contributing member is itself unknown for that stretch. The
board MUST NOT render that `None` as "exact" or drop it silently; it renders as its own
stated absence ("no schedule known for this stretch"), same as everywhere else in this
product that a `None` uncertainty appears — and that statement is a different claim from
the hatch a plain `Unknown` state already draws: the hatch says the state is not known,
the `None` uncertainty says precision is not a meaningful question to ask of the stretch at
all, because nothing at all is known about it. A hover title alone does not satisfy this:
the absence MUST also be visible in the default, un-hovered view — a modest marker on the
affected row plus a line in the section's own key or footnote — because the hover text a
person never triggers is, for that person, the same as it not existing. The marker MAY be
small and MUST NOT be drawn, nor the footnote line printed, for a render where no segment's
uncertainty is `None`.

## Violations

The board is non-conforming if it: reads the clock more than once per render or lets venues in
one render disagree on "now"; displays "now" without a clock-discipline annotation, or falls back
to an exact-looking clock when discipline data is unavailable; renders an unknown venue as
`Closed`, blank, or otherwise indistinguishable from a confident answer; computes any value
`core-api.md` was responsible for producing; persists rule data/evidence across renders
instead of taking it fresh from each `resolve_phases` call; draws a band or overlap row
without labelling it derived; draws a band's or overlap's state in the same glyph or colour
vocabulary as a `Phase`; renders an `Unknown` band or overlap stretch with anything other
than the venue section's own not-known hatch; renders an empty `BandSection` as anything
other than exactly the board this contract describes with no bands at all; or leaves a
`None` `BandSegment`/`OverlapSegment` uncertainty stated only in a hover title, with nothing
in the default, un-hovered view to show it.
