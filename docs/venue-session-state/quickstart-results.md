# Quickstart validation — recorded results

**Run**: 2026-07-30, against `crates/market-time-data/fixtures/synthetic-venues.json`
(three synthetic venues; see `DATA-LICENSING.md` for why no real venue data is present).

**What this run can and cannot establish.** The scenarios in
[`quickstart.md`](./quickstart.md) test two different things at once: that the engine
behaves correctly, and that the launch venues' data is right. Only the first is testable
today. Where a scenario asks about SSE, NYSE, or Binance specifically, the structural case
it exercises was run against the synthetic venue built for that case, and the row says so.
Nothing below is claimed for a real venue.

| # | Scenario | Criterion | Result |
|---|---|---|---|
| 1 | Single-venue query with phase boundaries | SC-001 | **Pass, structurally.** Boundaries returned with their own uncertainty. Real-venue accuracy is untested. |
| 2 | Every answer carries an openable source | SC-002, SC-008 | **Pass.** |
| 3 | Out-of-coverage returns an explicit unknown | SC-003 | **Pass.** |
| 4 | Multi-venue board, one venue unknown | SC-004 | **Pass.** |
| 5 | Reproducibility: same revisions, same query, same answer | SC-005 | **Pass, byte-identical.** |
| 6 | Shared vocabulary, zero venue-specific phase names | SC-006 | **Pass.** |
| 7 | Stated uncertainty never narrower than the source's precision | SC-007 | **Pass.** |
| 8 | Reaching the source from one answer | SC-008 | **Pass.** |

## 1 — Single-venue query with boundaries

```text
$ market-time phase --dataset <fixture> --venue SYNTH-AUCT --at 2026-07-30T04:00:00Z
SYNTH-AUCT: mid_day_break
  starts   2026-07-30T03:30:00Z (published to 1min)
  ends     2026-07-30T05:00:00Z (published to 1min)
```

The mid-day break is its own phase rather than "closed", and each boundary carries the
publication granularity of the rule behind it rather than claiming the nanosecond it is
represented to.

## 2, 8 — Evidence is reachable from an answer

```text
$ market-time evidence --dataset <fixture> --venue SYNTH-DST --at 2026-11-27T19:00:00Z
SYNTH-DST at 2026-11-27T19:00:00Z
  phase    post_close
  derived  the notice gives the close but not the post-close end; the ordinary
           four-hour post-close window is carried over
  source   https://synthetic.test/dst/half-days (fetched 2026-07-29T00:00:00Z, effective from 2026-01-01)
  revision synthetic-2026-07-30
```

The derived rule is marked derived and carries its reasoning, so the reading is not
presented as the venue's wording. The board prints the same documents in a sources block
under its rows, and both the CLI and the board obtain them from
`market_time_board::inspect`, so the two cannot disagree.

## 3 — Out of coverage

```text
$ market-time phase --dataset <fixture> --venue SYNTH-AUCT --at 2030-01-01T00:00:00Z
SYNTH-AUCT: not known
  reason   after SYNTH-AUCT coverage ends at 2026-12-31T16:00:00Z
  note     an unknown is not a closed market
exit status 0
```

Exit status 0 is deliberate: an unknown is an answer, not a failure. In `--format json`
the same outcome is `"phase": null` with `not_known_because` — never `"closed"`.

## 4 — Multi-venue, one venue unknown

```text
$ market-time board --dataset <fixture> --at 2026-12-31T06:00:00Z --hours 12 --columns 36
               06:00 08:00 10:00 12:00 14:00 16:00
SYNTH-ALWAYS  [|###################################]  continuous_trading
SYNTH-AUCT    [|##__.........................??????]  continuous_trading
SYNTH-DST     [|???????????????????????????????????]  not known
```

One venue is entirely outside its declared coverage and reports so without suppressing the
others; a second crosses its coverage edge mid-window and the stretch past the edge renders
as not-known rather than as a continued schedule. `?` is not a shade of the closed glyph.

## 5 — Reproducibility

Two consecutive `phase --format json` runs against the same revision produced byte-identical
output (`cmp` reported no difference).

## 6 — Shared vocabulary

Enforced at two points rather than checked by reading. The vocabulary is a closed
`#[non_exhaustive]` enum owned by the core, so no shell can extend it; and the loader
rejects a dataset that names a phase of its own —
`crates/market-time-data/tests/fixture_loads.rs::a_dataset_may_not_invent_a_phase_name`
asserts that `"night_session"` fails to load rather than being accepted.

## 7 — Uncertainty floor

```text
$ market-time phase --dataset <fixture> --venue SYNTH-ALWAYS --at 2026-06-14T08:00:00Z --format json
  event uncertainty: venue-published bound 15s (the venue publishes a 15-second deviation)
```

The venue's own published deviation is carried unchanged rather than narrowed, and
`Uncertainty::widest` treats an unbounded case (a process start, an ambiguous local time)
as wider than any number, so combining never sharpens an answer.

## What this run does not establish

- **SC-001, SC-004, SC-006, SC-007 for the launch venues.** Those require SSE, NYSE, and
  Binance data, which cannot be committed here. The engine behaviour each criterion depends
  on is exercised above against a venue built for that structural case.
- **Any claim about a real venue's schedule.** There is none in this repository.

Test suite at the time of this run: 76 tests, `cargo fmt`, `clippy -D warnings`, and
`cargo test --workspace` all clean.
