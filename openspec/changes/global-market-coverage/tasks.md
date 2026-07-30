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

- [ ] 3.1 Derive regional bands from constituent venue schedules in
      `crates/market-time-core/src/bands.rs`, marked derived, carrying the venue set
- [ ] 3.2 Compute overlap windows with uncertainty no narrower than the widest input
- [ ] 3.3 Vector: one constituent venue out of coverage makes the band unknown for that stretch
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
