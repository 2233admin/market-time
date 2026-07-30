use market_time_core::{
    CoverageRange, DatasetRevisionId, EvidenceRef, Phase, PhaseBoundary, PhaseSegment,
    PhaseTimeline, PhaseTimelineError, Uncertainty, UtcInstant,
};

fn instant(nanos: i128) -> UtcInstant {
    UtcInstant::from_nanos_since_unix_epoch(nanos)
}

fn evidence() -> EvidenceRef {
    EvidenceRef::observed(
        "https://example.com/market-time-test-source",
        instant(1),
        instant(0),
        None,
    )
    .expect("test evidence is valid")
}

fn segment(phase: Phase, start: i128, end: i128) -> PhaseSegment {
    PhaseSegment::new(
        phase,
        PhaseBoundary::new(instant(start), Uncertainty::exact()),
        PhaseBoundary::new(instant(end), Uncertainty::exact()),
        vec![evidence()],
        vec![DatasetRevisionId::new("test-r1").expect("valid revision id")],
    )
    .expect("test segment is valid")
}

#[test]
fn phase_timeline_rejects_a_gap_even_when_it_is_only_one_nanosecond() {
    let coverage =
        CoverageRange::closed_open(instant(0), instant(30)).expect("test coverage is valid");
    let segments = vec![
        segment(Phase::Closed, 0, 10),
        segment(Phase::ContinuousTrading, 11, 30),
    ];

    let error = PhaseTimeline::new(coverage, segments).expect_err("gap must be rejected");

    assert_eq!(
        error,
        PhaseTimelineError::Discontinuity {
            expected_start: instant(10),
            actual_start: instant(11),
        }
    );
}
