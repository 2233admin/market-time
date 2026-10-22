//! Synthetic venues for the golden-vector corpus.
//!
//! **These are invented schedules, not any real venue's.** No venue dataset ships from
//! this repository (see `DATA-LICENSING.md`): every launch venue forbids commercial
//! redistribution of its published calendar, so real schedules are fetched at run time by
//! the operator. What the tests need is not real data — it is data that exercises the
//! structural cases, which invented venues do without a licensing question:
//!
//! - `SYNTH-AUCT` — auctions plus a mid-day break, on a zone with no daylight saving.
//! - `SYNTH-DST` — a daylight-saving zone, with an open that is a process rather than an
//!   instant, plus a holiday and a shortened session.
//! - `SYNTH-ALWAYS` — never closes, and carries scheduled events that are not phases.
//!
//! The time-zone rules these venues sit in are IANA data compiled into the build, which
//! is a different thing from a venue's schedule and carries no such restriction.

#![allow(dead_code)]

use jiff::civil::{Date, Time, Weekday, date};
use market_time_core::CoverageRange;
use market_time_core::Phase;
use market_time_core::Uncertainty;
use market_time_core::{AssetFamily, DatasetRevision, Ruleset, VenueProfile, VenueRuleset};
use market_time_core::{CivilDaySchedule, DateRange, Rule, RuleKind};
use market_time_core::{DatasetRevisionId, IanaZoneId, VenueId};
use market_time_core::{DerivationNote, EvidenceRef};
use market_time_core::{EventKind, EventRule};
use market_time_core::{Interval, UtcInstant};

/// The revision every synthetic rule cites.
#[must_use]
pub fn revision_id() -> DatasetRevisionId {
    DatasetRevisionId::new("synthetic-2026-07-30").expect("valid identifier")
}

#[must_use]
pub fn revision() -> DatasetRevision {
    DatasetRevision {
        id: revision_id(),
        supersedes: None,
        iana_tzdb_version: market_time_core::tzdata::iana_tzdb_version().map(ToOwned::to_owned),
        assembled_at: instant("2026-07-30T00:00:00Z"),
    }
}

/// Parses an RFC-3339 instant. Test-only convenience; the core takes nanoseconds.
///
/// # Panics
///
/// Panics when `text` is not a valid timestamp — a broken vector should fail loudly.
#[must_use]
pub fn instant(text: &str) -> UtcInstant {
    let timestamp: jiff::Timestamp = text.parse().expect("test vector holds a valid timestamp");
    UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond())
}

/// Builds an interval, panicking on an invalid one.
///
/// # Panics
///
/// Panics when `start` is not before `end`.
#[must_use]
pub fn interval(start: &str, end: &str) -> Interval {
    Interval::new(instant(start), instant(end)).expect("test vector holds a valid interval")
}

fn evidence(url: &str) -> EvidenceRef {
    EvidenceRef::new(url, instant("2026-07-29T00:00:00Z"), "2026-01-01")
        .expect("test evidence is complete")
}

fn weekdays() -> Vec<Weekday> {
    vec![
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
    ]
}

fn coverage(start: &str, end: &str) -> CoverageRange {
    CoverageRange::new(interval(start, end))
}

fn schedule(points: Vec<(Time, Phase)>) -> CivilDaySchedule {
    CivilDaySchedule::new(points).expect("test schedule tiles the day")
}

fn dates(start: Date, end: Date) -> DateRange {
    DateRange::new(start, end).expect("test date range runs forwards")
}

/// A venue with auctions and a mid-day break, in a zone with no daylight saving.
#[must_use]
pub fn auction_venue() -> VenueRuleset {
    let trading_day = schedule(vec![
        (Time::midnight(), Phase::Closed),
        (Time::constant(9, 15, 0, 0), Phase::PreOpen),
        (Time::constant(9, 25, 0, 0), Phase::OpeningAuction),
        (Time::constant(9, 30, 0, 0), Phase::ContinuousTrading),
        (Time::constant(11, 30, 0, 0), Phase::MidDayBreak),
        (Time::constant(13, 0, 0, 0), Phase::ContinuousTrading),
        (Time::constant(14, 57, 0, 0), Phase::ClosingAuction),
        (Time::constant(15, 0, 0, 0), Phase::PostClose),
        (Time::constant(15, 30, 0, 0), Phase::Closed),
    ]);

    VenueRuleset {
        venue: VenueId::new("SYNTH-AUCT").expect("valid identifier"),
        profile: VenueProfile {
            display_name: Some("Synthetic Auction Exchange".to_owned()),
            location: Some("Shanghai".to_owned()),
            family: Some(AssetFamily::Equities),
        },
        home_zone: IanaZoneId::new("Asia/Shanghai").expect("valid identifier"),
        coverage: coverage("2026-01-01T00:00:00Z", "2026-12-31T16:00:00Z"),
        rules: vec![
            Rule {
                kind: RuleKind::WeeklyPattern {
                    weekdays: weekdays(),
                },
                schedule: trading_day,
                applies: dates(date(2026, 1, 1), date(2026, 12, 31)),
                boundary_uncertainty: Uncertainty::minutes(1),
                evidence: evidence("https://synthetic.test/auct/trading-rules"),
                derived: None,
                revision: revision_id(),
            },
            Rule {
                kind: RuleKind::WeeklyPattern {
                    weekdays: vec![Weekday::Saturday, Weekday::Sunday],
                },
                schedule: schedule(vec![(Time::midnight(), Phase::Closed)]),
                applies: dates(date(2026, 1, 1), date(2026, 12, 31)),
                boundary_uncertainty: Uncertainty::minutes(1),
                evidence: evidence("https://synthetic.test/auct/trading-rules"),
                derived: None,
                revision: revision_id(),
            },
            Rule {
                kind: RuleKind::Holiday {
                    name: "Synthetic National Day".to_owned(),
                },
                schedule: schedule(vec![(Time::midnight(), Phase::Closed)]),
                applies: dates(date(2026, 10, 1), date(2026, 10, 1)),
                boundary_uncertainty: Uncertainty::minutes(1),
                evidence: evidence("https://synthetic.test/auct/holidays-2026"),
                derived: None,
                revision: revision_id(),
            },
        ],
        events: Vec::new(),
        evidence: vec![evidence("https://synthetic.test/auct/")],
    }
}

/// A venue in a daylight-saving zone, whose open is a process rather than an instant.
#[must_use]
pub fn dst_venue() -> VenueRuleset {
    let trading_day = schedule(vec![
        (Time::midnight(), Phase::Closed),
        (Time::constant(4, 0, 0, 0), Phase::PreOpen),
        (Time::constant(9, 30, 0, 0), Phase::ContinuousTrading),
        (Time::constant(16, 0, 0, 0), Phase::PostClose),
        (Time::constant(20, 0, 0, 0), Phase::Closed),
    ]);

    let early_close = schedule(vec![
        (Time::midnight(), Phase::Closed),
        (Time::constant(4, 0, 0, 0), Phase::PreOpen),
        (Time::constant(9, 30, 0, 0), Phase::ContinuousTrading),
        (Time::constant(13, 0, 0, 0), Phase::PostClose),
        (Time::constant(17, 0, 0, 0), Phase::Closed),
    ]);

    VenueRuleset {
        venue: VenueId::new("SYNTH-DST").expect("valid identifier"),
        profile: VenueProfile {
            display_name: Some("Synthetic Daylight Exchange".to_owned()),
            location: Some("New York".to_owned()),
            family: Some(AssetFamily::Equities),
        },
        home_zone: IanaZoneId::new("America/New_York").expect("valid identifier"),
        coverage: coverage("2026-01-01T05:00:00Z", "2026-12-31T05:00:00Z"),
        rules: vec![
            Rule {
                kind: RuleKind::WeeklyPattern {
                    weekdays: weekdays(),
                },
                schedule: trading_day,
                applies: dates(date(2026, 1, 1), date(2026, 12, 30)),
                boundary_uncertainty: Uncertainty::ProcessStart {
                    process: "the security-by-security opening process".to_owned(),
                },
                evidence: evidence("https://synthetic.test/dst/hours"),
                derived: None,
                revision: revision_id(),
            },
            Rule {
                kind: RuleKind::WeeklyPattern {
                    weekdays: vec![Weekday::Saturday, Weekday::Sunday],
                },
                schedule: schedule(vec![(Time::midnight(), Phase::Closed)]),
                applies: dates(date(2026, 1, 1), date(2026, 12, 30)),
                boundary_uncertainty: Uncertainty::minutes(1),
                evidence: evidence("https://synthetic.test/dst/hours"),
                derived: None,
                revision: revision_id(),
            },
            Rule {
                kind: RuleKind::ShortenedSession {
                    name: "Synthetic half day".to_owned(),
                },
                schedule: early_close,
                applies: dates(date(2026, 11, 27), date(2026, 11, 27)),
                boundary_uncertainty: Uncertainty::minutes(1),
                evidence: evidence("https://synthetic.test/dst/half-days"),
                derived: Some(
                    DerivationNote::new(
                        "the notice gives the close but not the post-close end; the ordinary \
                         four-hour post-close window is carried over",
                    )
                    .expect("reasoning is present"),
                ),
                revision: revision_id(),
            },
        ],
        events: Vec::new(),
        evidence: vec![evidence("https://synthetic.test/dst/")],
    }
}

/// A venue that never closes, carrying scheduled events that are not phases.
#[must_use]
pub fn always_on_venue() -> VenueRuleset {
    VenueRuleset {
        venue: VenueId::new("SYNTH-ALWAYS").expect("valid identifier"),
        profile: VenueProfile {
            display_name: Some("Synthetic Always-On Venue".to_owned()),
            location: Some("Global".to_owned()),
            family: Some(AssetFamily::Futures),
        },
        home_zone: IanaZoneId::new("UTC").expect("valid identifier"),
        coverage: coverage("2026-01-01T00:00:00Z", "2027-01-01T00:00:00Z"),
        rules: vec![Rule {
            kind: RuleKind::WeeklyPattern {
                weekdays: vec![
                    Weekday::Monday,
                    Weekday::Tuesday,
                    Weekday::Wednesday,
                    Weekday::Thursday,
                    Weekday::Friday,
                    Weekday::Saturday,
                    Weekday::Sunday,
                ],
            },
            schedule: CivilDaySchedule::all_day(Phase::ContinuousTrading)
                .expect("an all-day schedule tiles the day"),
            applies: dates(date(2026, 1, 1), date(2026, 12, 31)),
            boundary_uncertainty: Uncertainty::Exact,
            evidence: evidence("https://synthetic.test/always/trading-rules"),
            derived: None,
            revision: revision_id(),
        }],
        events: vec![EventRule {
            kind: EventKind::FundingSettlement,
            times: vec![
                Time::midnight(),
                Time::constant(8, 0, 0, 0),
                Time::constant(16, 0, 0, 0),
            ],
            applies: dates(date(2026, 1, 1), date(2026, 12, 31)),
            uncertainty: Uncertainty::PublishedBound {
                nanos: 15 * market_time_core::NANOS_PER_SECOND,
                published_as: "the venue publishes a 15-second deviation".to_owned(),
            },
            evidence: evidence("https://synthetic.test/always/funding"),
            revision: revision_id(),
        }],
        evidence: vec![evidence("https://synthetic.test/always/")],
    }
}

/// All three synthetic venues, validated.
///
/// # Panics
///
/// Panics when the fixtures fail validation — which is itself the thing under test in
/// `ruleset_validation.rs`, so a panic here means the fixtures drifted.
#[must_use]
pub fn ruleset() -> Ruleset {
    Ruleset::from_parts(
        vec![revision()],
        vec![auction_venue(), dst_venue(), always_on_venue()],
    )
    .expect("synthetic fixtures are a valid ruleset")
}

/// The identifiers of the three synthetic venues, in catalog order.
#[must_use]
pub fn venue_ids() -> Vec<VenueId> {
    vec![
        VenueId::new("SYNTH-ALWAYS").expect("valid identifier"),
        VenueId::new("SYNTH-AUCT").expect("valid identifier"),
        VenueId::new("SYNTH-DST").expect("valid identifier"),
    ]
}

// ------------------------------------------------------------- band fixtures (task 3)

/// A minimal venue for [`bands`](../bands.rs) tests: UTC, no daylight saving, no auctions,
/// trading every day of the week so a single calendar day is enough to exercise it. Trades
/// 08:00-12:00Z.
///
/// Kept separate from `auction_venue`/`dst_venue`/`always_on_venue` and their zone and
/// daylight-saving detail: the band-derivation tests want two venues whose trading windows
/// touch and overlap in a way that is easy to state in UTC, not another DST case.
///
/// Coverage runs the whole day, unlike [`band_a_gap_venue`], which is the same schedule cut
/// short.
#[must_use]
pub fn band_a_venue() -> VenueRuleset {
    band_a_venue_with(
        "SYNTH-BAND-A",
        coverage("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z"),
    )
}

/// The same schedule as [`band_a_venue`], but with coverage ending at 09:00Z — inside its
/// own trading window (08:00-12:00) and, crucially, *before* [`band_b_gap_venue`]'s own
/// cutoff at 11:00Z. Pairing this with `band_b_gap_venue` gives a stretch (09:00-11:00) with
/// exactly one member unknown, and a stretch (11:00 onward) where *every* member is
/// unknown — the only combination that can make [`crate::BandSegment::uncertainty`] read
/// `None`.
#[must_use]
pub fn band_a_gap_venue() -> VenueRuleset {
    band_a_venue_with(
        "SYNTH-BAND-A-GAP",
        coverage("2026-01-01T00:00:00Z", "2026-01-01T09:00:00Z"),
    )
}

fn band_a_venue_with(id: &str, coverage_range: CoverageRange) -> VenueRuleset {
    let trading_day = schedule(vec![
        (Time::midnight(), Phase::Closed),
        (Time::constant(8, 0, 0, 0), Phase::ContinuousTrading),
        (Time::constant(12, 0, 0, 0), Phase::Closed),
    ]);

    VenueRuleset {
        venue: VenueId::new(id).expect("valid identifier"),
        profile: VenueProfile {
            display_name: Some("Synthetic Band Venue A".to_owned()),
            location: Some("Global".to_owned()),
            family: Some(AssetFamily::Equities),
        },
        home_zone: IanaZoneId::new("UTC").expect("valid identifier"),
        coverage: coverage_range,
        rules: vec![Rule {
            kind: RuleKind::WeeklyPattern {
                weekdays: vec![
                    Weekday::Monday,
                    Weekday::Tuesday,
                    Weekday::Wednesday,
                    Weekday::Thursday,
                    Weekday::Friday,
                    Weekday::Saturday,
                    Weekday::Sunday,
                ],
            },
            schedule: trading_day,
            applies: dates(date(2026, 1, 1), date(2026, 1, 1)),
            boundary_uncertainty: Uncertainty::minutes(1),
            evidence: evidence("https://synthetic.test/band-a/trading-rules"),
            derived: None,
            revision: revision_id(),
        }],
        events: Vec::new(),
        evidence: vec![evidence("https://synthetic.test/band-a/")],
    }
}

/// A second minimal UTC venue, whose trading window (10:00-14:00Z) overlaps
/// `band_a_venue`'s (08:00-12:00Z) rather than merely touching it: 08:00-10:00 is A alone,
/// 10:00-12:00 is both, 12:00-14:00 is B alone. Its published boundary uncertainty
/// (10 minutes) is wider than A's (1 minute), so a band or overlap covering the shared
/// stretch must never be narrower than 10 minutes.
///
/// Coverage runs the whole day, unlike [`band_b_gap_venue`], which is the same schedule cut
/// short to exercise task 3.3's out-of-coverage vector.
#[must_use]
pub fn band_b_venue() -> VenueRuleset {
    band_b_venue_with(
        "SYNTH-BAND-B",
        coverage("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z"),
    )
}

/// The same schedule as [`band_b_venue`], but with coverage ending at 11:00Z — inside its
/// own trading window and inside `band_a_venue`'s trading window too. A band or overlap
/// derived across the two must go unknown for the stretch from 11:00Z onward, never
/// narrowed to what `band_a_venue` alone is doing (task 3.3).
#[must_use]
pub fn band_b_gap_venue() -> VenueRuleset {
    band_b_venue_with(
        "SYNTH-BAND-B-GAP",
        coverage("2026-01-01T00:00:00Z", "2026-01-01T11:00:00Z"),
    )
}

fn band_b_venue_with(id: &str, coverage_range: CoverageRange) -> VenueRuleset {
    let trading_day = schedule(vec![
        (Time::midnight(), Phase::Closed),
        (Time::constant(10, 0, 0, 0), Phase::ContinuousTrading),
        (Time::constant(14, 0, 0, 0), Phase::Closed),
    ]);

    VenueRuleset {
        venue: VenueId::new(id).expect("valid identifier"),
        profile: VenueProfile {
            display_name: Some("Synthetic Band Venue B".to_owned()),
            location: Some("Global".to_owned()),
            family: Some(AssetFamily::Equities),
        },
        home_zone: IanaZoneId::new("UTC").expect("valid identifier"),
        coverage: coverage_range,
        rules: vec![Rule {
            kind: RuleKind::WeeklyPattern {
                weekdays: vec![
                    Weekday::Monday,
                    Weekday::Tuesday,
                    Weekday::Wednesday,
                    Weekday::Thursday,
                    Weekday::Friday,
                    Weekday::Saturday,
                    Weekday::Sunday,
                ],
            },
            schedule: trading_day,
            applies: dates(date(2026, 1, 1), date(2026, 1, 1)),
            boundary_uncertainty: Uncertainty::minutes(10),
            evidence: evidence("https://synthetic.test/band-b/trading-rules"),
            derived: None,
            revision: revision_id(),
        }],
        events: Vec::new(),
        evidence: vec![evidence("https://synthetic.test/band-b/")],
    }
}

/// A ruleset holding only the band-test venues, isolated from [`ruleset`] so that growing
/// this fixture can never perturb `vectors.rs`'s golden vectors.
///
/// # Panics
///
/// Panics when the fixtures fail validation, which would itself mean the fixtures drifted.
#[must_use]
pub fn band_ruleset() -> Ruleset {
    Ruleset::from_parts(
        vec![revision()],
        vec![
            band_a_venue(),
            band_a_gap_venue(),
            band_b_venue(),
            band_b_gap_venue(),
        ],
    )
    .expect("band fixtures are a valid ruleset")
}

/// The full day the band fixtures are queried over: 2026-01-01T00:00:00Z, a Thursday.
#[must_use]
pub fn band_day() -> Interval {
    interval("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z")
}
