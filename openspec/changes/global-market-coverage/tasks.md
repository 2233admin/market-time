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
beyond the launch set, session segment roles, and the derived session bands.

Tasks 3.1-3.3 landed too, ahead of 1.4 and section 2, because they turned out not to
depend on either: `derive_band` and `derive_overlap` in `crates/market-time-core/src/bands.rs`
take `&[Timeline]` directly, so they needed no new coverage plumbing and no segment-role
field to exist first. A `SessionBand` cuts its members' timelines at every boundary and
classifies each slice — unknown wins if any member is unknown there (task 3.3's vector,
golden-tested in `tests/bands.rs`), otherwise trading wins if any member is trading, else
not-trading — and folds `Option<Uncertainty>` over the *known* contributors, `None` only
when every member is unknown for that slice. `derive_overlap` cuts two bands the same way;
an unknown band always wins there too, never read as "not overlapping". Both carry a
`DerivationNote` and are never presented as a published fact, per the spec. What is still
open: band *definitions* are not yet loadable from a dataset — every band in the test suite
is constructed by hand in the test itself — and nothing wires a band into `market-time-data`
or a shell. That is next, once 1.4 and section 2 give it real per-venue coverage and segment
roles to draw on.

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

## 4. Source verification (blocks everything above it that touches data)

- [ ] 4.1 For each candidate venue: locate the first-party published schedule, record its terms
      at registration, and record the retrieval and effective dates
- [ ] 4.2 Reject any venue whose schedule is only available second-hand. A market that cannot be
      evidenced is not added, however conventional it is to show it

## Notes

- No venue dataset is committed, now or ever (see `DATA-LICENSING.md`). This change does not
  weaken that.
- Turnover and volume panels are out of scope: that is market data, not schedule data.
