//! Golden vectors: the primary correctness instrument (Principle V).
//!
//! Vectors are permanent. A failing vector means either the code is wrong or the vector
//! was wrong and its correction is itself evidenced — never that the vector should be
//! deleted to make a build pass.

mod common;

use common::{instant, interval, ruleset, venue_ids};
use market_time_core::event::EventKind;
use market_time_core::ids::VenueId;
use market_time_core::phase::Phase;
use market_time_core::uncertainty::Uncertainty;
use market_time_core::{PhaseOutcome, resolve_phase, resolve_phases, resolve_timeline};

fn phase_at(venue: &str, at: &str) -> PhaseOutcome {
    resolve_phase(instant(at), &VenueId::new(venue), &ruleset())
}

fn known(venue: &str, at: &str) -> market_time_core::PhaseAnswer {
    match phase_at(venue, at) {
        PhaseOutcome::Known(answer) => *answer,
        PhaseOutcome::Unknown(gap) => panic!("expected a phase, got unknown: {}", gap.describe()),
    }
}

// ---------------------------------------------------------------- auction venue

#[test]
fn v001_continuous_trading_mid_morning() {
    // 10:00 Shanghai on a Thursday.
    assert_eq!(
        known("SYNTH-AUCT", "2026-07-30T02:00:00Z").phase,
        Phase::ContinuousTrading
    );
}

#[test]
fn v002_midday_break_is_neither_open_nor_closed() {
    // 12:00 Shanghai. A two-state open/closed model cannot express this.
    let answer = known("SYNTH-AUCT", "2026-07-30T04:00:00Z");
    assert_eq!(answer.phase, Phase::MidDayBreak);
    assert_ne!(answer.phase, Phase::Closed);
    assert!(!answer.phase.is_trading());
}

#[test]
fn v003_opening_auction_is_its_own_phase() {
    // 09:27 Shanghai.
    assert_eq!(
        known("SYNTH-AUCT", "2026-07-30T01:27:00Z").phase,
        Phase::OpeningAuction
    );
}

#[test]
fn v004_closing_auction_then_post_close() {
    assert_eq!(
        known("SYNTH-AUCT", "2026-07-30T06:58:00Z").phase,
        Phase::ClosingAuction
    );
    assert_eq!(
        known("SYNTH-AUCT", "2026-07-30T07:10:00Z").phase,
        Phase::PostClose
    );
}

#[test]
fn v005_weekend_is_closed() {
    // Saturday 10:00 Shanghai.
    assert_eq!(
        known("SYNTH-AUCT", "2026-08-01T02:00:00Z").phase,
        Phase::Closed
    );
}

#[test]
fn v006_holiday_outranks_the_weekly_pattern() {
    // 1 October 2026 is a Thursday: the weekly pattern would say trading.
    let answer = known("SYNTH-AUCT", "2026-10-01T02:00:00Z");
    assert_eq!(answer.phase, Phase::Closed);
    assert!(
        answer
            .evidence
            .iter()
            .any(|e| e.source_url().contains("holidays-2026")),
        "the holiday notice is the evidence, not the weekly pattern"
    );
}

#[test]
fn v007_a_boundary_instant_belongs_to_exactly_one_phase() {
    // 09:30 Shanghai exactly: continuous trading starts, pre-open's auction ends.
    assert_eq!(
        known("SYNTH-AUCT", "2026-07-30T01:30:00Z").phase,
        Phase::ContinuousTrading
    );
    // One nanosecond earlier is still the auction.
    assert_eq!(
        known("SYNTH-AUCT", "2026-07-30T01:29:59.999999999Z").phase,
        Phase::OpeningAuction
    );
}

// -------------------------------------------------------------------- DST venue

#[test]
fn v008_daylight_saving_moves_the_open_in_utc() {
    // 09:30 New York is 14:30Z in winter and 13:30Z in summer. Same rule, same civil
    // time, different absolute instant — which is why rules are stored in civil time.
    assert_eq!(
        known("SYNTH-DST", "2026-01-15T14:30:00Z").phase,
        Phase::ContinuousTrading
    );
    assert_eq!(
        known("SYNTH-DST", "2026-01-15T13:30:00Z").phase,
        Phase::PreOpen
    );
    assert_eq!(
        known("SYNTH-DST", "2026-07-15T13:30:00Z").phase,
        Phase::ContinuousTrading
    );
}

#[test]
fn v009_spring_forward_is_carried_not_invented() {
    // 2026-03-08: New York clocks jump 02:00 -> 03:00. The pre-open boundary at 04:00
    // local still exists, so the day resolves; what must not happen is a silently
    // invented instant for a local time that does not exist.
    let answer = known("SYNTH-DST", "2026-03-09T13:30:00Z");
    assert_eq!(answer.phase, Phase::ContinuousTrading);
}

#[test]
fn v010_an_open_that_is_a_process_never_claims_an_instant() {
    let answer = known("SYNTH-DST", "2026-07-15T13:30:00Z");
    assert!(
        matches!(
            answer.boundary_start.uncertainty,
            Uncertainty::ProcessStart { .. }
        ),
        "the open is the scheduled start of a process, not a market-wide transition"
    );
    assert!(!answer.boundary_start.uncertainty.is_exact());
    assert_eq!(answer.boundary_start.uncertainty.bound_nanos(), None);
}

#[test]
fn v011_a_shortened_session_replaces_the_normal_one() {
    // 2026-11-27 is a Friday; the normal schedule would still be trading at 14:00 local.
    let answer = known("SYNTH-DST", "2026-11-27T19:00:00Z");
    assert_eq!(answer.phase, Phase::PostClose);
    assert!(
        answer.derived_reasoning.is_some(),
        "the post-close end was our reading, and derived is never presented as observed"
    );
}

// --------------------------------------------------------------- always-on venue

#[test]
fn v012_an_always_on_venue_is_never_closed() {
    for at in [
        "2026-01-01T00:00:00Z",
        "2026-06-14T03:17:42Z",
        "2026-12-31T23:59:59Z",
    ] {
        assert_eq!(
            known("SYNTH-ALWAYS", at).phase,
            Phase::ContinuousTrading,
            "always-on venue at {at}"
        );
    }
}

#[test]
fn v013_a_funding_settlement_is_an_event_not_a_phase() {
    let answer = known("SYNTH-ALWAYS", "2026-06-14T08:00:00Z");
    assert_eq!(
        answer.phase,
        Phase::ContinuousTrading,
        "the event does not replace the phase it sits inside"
    );
    assert!(
        answer
            .events
            .iter()
            .any(|e| e.kind == EventKind::FundingSettlement),
        "the settlement is reported as an event on the phase"
    );
}

#[test]
fn v014_a_venue_published_bound_is_carried_unchanged() {
    let answer = known("SYNTH-ALWAYS", "2026-06-14T08:00:00Z");
    let settlement = answer
        .events
        .iter()
        .find(|e| e.kind == EventKind::FundingSettlement)
        .expect("a settlement occurs in this phase");

    match &settlement.uncertainty {
        Uncertainty::PublishedBound { nanos, .. } => {
            assert_eq!(*nanos, 15 * market_time_core::instant::NANOS_PER_SECOND);
        }
        other => panic!("expected the venue's published bound, got {other}"),
    }
}

// ------------------------------------------------------------------- coverage

#[test]
fn v015_outside_coverage_is_unknown_not_closed() {
    let outcome = phase_at("SYNTH-AUCT", "2028-03-01T02:00:00Z");
    assert!(outcome.is_unknown());
    assert_eq!(outcome.phase(), None, "unknown is not a phase, closed is");

    let PhaseOutcome::Unknown(gap) = outcome else {
        unreachable!("asserted unknown above")
    };
    assert!(
        gap.describe().contains("coverage ends"),
        "the answer names the boundary it fell outside: {}",
        gap.describe()
    );
}

#[test]
fn v016_a_venue_with_no_data_has_zero_coverage() {
    let outcome = phase_at("SYNTH-NOT-TRACKED", "2026-07-30T02:00:00Z");
    assert!(
        outcome.is_unknown(),
        "an unknown venue is a query answer, not a call failure"
    );
}

// -------------------------------------------------------------- multi-venue

#[test]
fn v017_three_venues_three_states_at_one_instant() {
    // 2026-07-30T04:00:00Z: Shanghai on its mid-day break, New York closed overnight,
    // the always-on venue trading.
    let outcomes = resolve_phases(instant("2026-07-30T04:00:00Z"), &venue_ids(), &ruleset());
    let phases: Vec<Option<Phase>> = outcomes.iter().map(|o| o.outcome.phase()).collect();

    assert_eq!(
        phases,
        vec![
            Some(Phase::ContinuousTrading),
            Some(Phase::MidDayBreak),
            Some(Phase::Closed),
        ]
    );
}

#[test]
fn v018_one_venue_out_of_coverage_does_not_void_the_others() {
    let venues = vec![
        VenueId::new("SYNTH-ALWAYS"),
        VenueId::new("SYNTH-NOT-TRACKED"),
        VenueId::new("SYNTH-AUCT"),
    ];
    let outcomes = resolve_phases(instant("2026-07-30T02:00:00Z"), &venues, &ruleset());

    assert_eq!(
        outcomes.len(),
        3,
        "one outcome per requested venue, in order"
    );
    assert!(!outcomes[0].outcome.is_unknown());
    assert!(outcomes[1].outcome.is_unknown());
    assert!(!outcomes[2].outcome.is_unknown());
}

// ---------------------------------------------------------------- timelines

#[test]
fn v019_a_trading_day_tiles_without_gaps() {
    let timeline = resolve_timeline(
        interval("2026-07-30T00:00:00Z", "2026-07-31T00:00:00Z"),
        &VenueId::new("SYNTH-AUCT"),
        &ruleset(),
    );

    assert!(
        timeline.tiles_interval(),
        "segments must tile the queried interval"
    );
    assert!(
        timeline
            .segments
            .iter()
            .any(|s| s.phase() == Some(Phase::MidDayBreak)),
        "the mid-day break is its own segment, not merged into trading"
    );
}

#[test]
fn v020_a_timeline_crossing_the_coverage_edge_answers_both_sides() {
    let timeline = resolve_timeline(
        interval("2026-12-31T00:00:00Z", "2027-01-02T00:00:00Z"),
        &VenueId::new("SYNTH-ALWAYS"),
        &ruleset(),
    );

    assert!(timeline.tiles_interval());
    assert!(
        timeline.segments.iter().any(|s| !s.is_unknown()),
        "the covered part still answers"
    );
    assert!(
        timeline.segments.iter().any(TimelineSegmentExt::unknown),
        "the part past coverage is an explicit unknown segment, not a hole"
    );
}

trait TimelineSegmentExt {
    fn unknown(&self) -> bool;
}

impl TimelineSegmentExt for market_time_core::TimelineSegment {
    fn unknown(&self) -> bool {
        self.is_unknown()
    }
}

#[test]
fn v021_an_always_on_venue_is_one_segment_not_seven() {
    let timeline = resolve_timeline(
        interval("2026-06-01T00:00:00Z", "2026-06-08T00:00:00Z"),
        &VenueId::new("SYNTH-ALWAYS"),
        &ruleset(),
    );

    assert!(timeline.tiles_interval());
    assert_eq!(
        timeline.segments.len(),
        1,
        "consecutive identical phases join rather than showing a boundary each midnight"
    );
}

#[test]
fn v022_every_minute_of_a_day_belongs_to_exactly_one_segment() {
    let day = interval("2026-07-30T00:00:00Z", "2026-07-31T00:00:00Z");
    let timeline = resolve_timeline(day, &VenueId::new("SYNTH-DST"), &ruleset());

    let minute_nanos = 60 * market_time_core::instant::NANOS_PER_SECOND;
    let mut at = day.start;
    while at < day.end {
        let hits = timeline
            .segments
            .iter()
            .filter(|segment| segment.interval().contains(at))
            .count();
        assert_eq!(hits, 1, "exactly one segment owns {at}");
        at = at.saturating_add_nanos(minute_nanos);
    }
}

// ------------------------------------------------------------ reproducibility

#[test]
fn v023_the_same_query_returns_the_same_answer() {
    let first = phase_at("SYNTH-AUCT", "2026-07-30T02:00:00Z");
    let second = phase_at("SYNTH-AUCT", "2026-07-30T02:00:00Z");
    assert_eq!(first, second);
}

#[test]
fn v024_answers_name_the_revisions_that_produced_them() {
    let answer = known("SYNTH-AUCT", "2026-07-30T02:00:00Z");
    assert!(
        answer
            .dataset_revisions
            .iter()
            .any(|r| r.as_str() == "synthetic-2026-07-30"),
        "an answer is attributable to the revisions behind it"
    );
    assert!(
        !answer.evidence.is_empty(),
        "no answer is returned without a source a person can open"
    );
}
