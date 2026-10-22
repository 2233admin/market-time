# Tasks: Global market coverage

Deferred. This change is specified now so the shape is settled before venue number four, and it
does not start until `venue-session-state` is delivered and archived. Every task below is gated
on a first-party source and its terms check — no venue is added on the strength of appearing on
someone else's board.

## Status (2026-07-31)

Tasks 1.1 and 1.3 landed early, out of order, because the board needed them: a board that
cannot group its rows is not the board this product is aiming at. `AssetFamily` is a closed
three-value vocabulary (equities, spot_and_fx, futures) and `VenueProfile` carries display
name, location, and family. The dataset format carries all three as optional fields, a
family outside the set fails to load, and both renderers group rows by family and report how
many venues are trading — with not-known counted separately, because "12 trading" alone
hides whether the rest are closed or simply unknown.

What is still open here is the part that needs venues: per-venue coverage declarations
beyond the launch set, and session segment roles.

Tasks 3.1-3.3 landed too, ahead of 1.4 and section 2, because they turned out not to
depend on either: `derive_band` and `derive_overlap` in `crates/market-time-core/src/bands.rs`
take `&[Timeline]` directly, so they needed no new coverage plumbing and no segment-role
field to exist first. A `SessionBand` cuts its members' timelines at every boundary and
classifies each slice — unknown wins if any member is unknown there (task 3.3's vector,
golden-tested in `tests/bands.rs`), otherwise trading wins if any member is trading, else
not-trading — and folds `Option<Uncertainty>` over the *known* contributors, `None` only
when every member is unknown for that slice. `derive_overlap` cuts two bands the same way;
an unknown band always wins there too, never read as "not overlapping". Both carry a
`DerivationNote` and are never presented as a published fact, per the spec.

The loader and both shells landed next, for the same reason the engine itself did: a band's
loader needs a declared venue set to check membership against, not per-venue coverage beyond
what is already loaded and not a segment-role field, so it did not have to wait on 1.4 or
section 2 either. `market-time-data` now reads band definitions from the dataset file next
to the venues they group — a `BandRecord` carries its members and a required reasoning
string, and the loader refuses a band naming a venue the file cannot answer for, so a band's
unknown stretches are never an artefact of the file rather than of the evidence. The
`bands` CLI command derives every band the dataset declares, or the ones `--band` selects,
over the same `--at`/`--hours` window as `timeline`, and every pairwise overlap between
them, printing each derivation note so no line of that output can be read as a schedule a
venue published. The board draws the same bands and overlaps beneath its venue rows, in a
vocabulary that is never the phase vocabulary — its own glyphs, its own key, the word
"derived" on every label, so a reader scanning quickly cannot mistake a band row for a venue
row — and `--no-bands` suppresses that section outright; a dataset with no bands declared
renders with no section either way, not an empty one. What is still open is exactly what was
open before: per-venue coverage declarations beyond the launch set (1.2, 1.4) and session
segment roles (all of section 2).

## 1. Catalog

- [x] 1.1 Define the venue catalog record — display name, location, home zone, asset-class
      family — in `crates/market-time-core/src/venue.rs`, with family as a closed enum
- [ ] 1.2 Contract test in `crates/market-time-core/tests/contract/catalog.rs`: adding a venue
      introduces no phase name and no venue-specific branch in resolution
- [x] 1.3 Move the launch three onto catalog records, with no behaviour change
- [ ] 1.4 Per-venue coverage declaration in `crates/market-time-data`, replacing any assumption
      that the launch set shares one range

## 2. Session segments

- [ ] 2.1 Golden vectors first: a venue day with a main block and a night block, and a block that
      crosses midnight, in `crates/market-time-core/tests/vectors/`
- [ ] 2.2 Add the block role to the segment type in `crates/market-time-core/src/phase.rs`,
      keeping the phase vocabulary closed and unchanged
- [ ] 2.3 Evidence test: a role the venue does not publish itself is rejected unless marked
      derived with reasoning

## 3. Session bands

- [x] 3.1 Derive regional bands from constituent venue schedules in
      `crates/market-time-core/src/bands.rs`, marked derived, carrying the venue set
- [x] 3.2 Compute overlap windows with uncertainty no narrower than the widest input
- [x] 3.3 Vector: one constituent venue out of coverage makes the band unknown for that stretch
      rather than silently narrowing it to the remaining venues
- [x] 3.4 Load band definitions from the dataset file in `crates/market-time-data`, next to the
      venues they group, refusing a band that names a venue the file does not declare
- [x] 3.5 A `bands` CLI command deriving the dataset's bands (or `--band`'s selection) and every
      pairwise overlap between them, printing each derivation note
- [x] 3.6 Draw derived bands and overlap windows on the board, in a glyph and key vocabulary
      distinct from the venue rows', suppressible with `--no-bands`

## 4. Source verification (blocks everything above it that touches data)

- [ ] 4.1 For each candidate venue: locate the first-party published schedule, record its terms
      at registration, and record the retrieval and effective dates
- [ ] 4.2 Reject any venue whose schedule is only available second-hand. A market that cannot be
      evidenced is not added, however conventional it is to show it

## Notes

- No venue dataset is committed, now or ever (see `DATA-LICENSING.md`). This change does not
  weaken that.
- Turnover and volume panels are out of scope: that is market data, not schedule data.
