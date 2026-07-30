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
use market_time_core::{CivilDaySchedule, DateRange, Rule, RuleKind};
use market_time_core::{DatasetRevision, Ruleset, VenueRuleset};
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
