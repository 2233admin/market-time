use market_time_core::{
    CoverageRange, DatasetRevision, DatasetRevisionId, EvidenceRef, IanaZoneId, Phase,
    PhaseBoundary, PhaseOutcome, PhaseSegment, PhaseTimeline, Ruleset, Uncertainty, UtcInstant,
    VenueId, VenueRuleset, resolve_phase,
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

fn ruleset() -> Ruleset {
    let revision = DatasetRevision::new(
        DatasetRevisionId::new("test-r1").expect("valid revision id"),
        "synthetic-test-schedule",
        instant(2),
        "Synthetic schedule owned by the Market Time test suite",
        None,
    )
    .expect("test revision is valid");
    let coverage =
        CoverageRange::closed_open(instant(0), instant(30)).expect("test coverage is valid");
    let timeline = PhaseTimeline::new(
        coverage,
        vec![
            segment(Phase::Closed, 0, 10),
            segment(Phase::ContinuousTrading, 10, 30),
        ],
    )
    .expect("test timeline is valid");
    let venue = VenueRuleset::new(
        VenueId::new("X-MT-TEST").expect("valid venue id"),
        IanaZoneId::new("UTC").expect("valid zone id"),
        timeline,
    );

    Ruleset::from_parts(vec![revision], vec![venue]).expect("test ruleset is valid")
}

#[test]
fn exact_boundary_belongs_to_following_phase_with_full_attribution() {
    let ruleset = ruleset();
    let outcome = resolve_phase(
        instant(10),
        VenueId::new("X-MT-TEST").expect("valid venue id"),
        &ruleset,
    );

    let PhaseOutcome::Known(answer) = outcome else {
        panic!("covered boundary must resolve to a known answer");
    };
    assert_eq!(answer.phase, Phase::ContinuousTrading);
    assert_eq!(answer.boundary_start.instant, instant(10));
    assert_eq!(answer.boundary_end.instant, instant(30));
    assert_eq!(answer.uncertainty, Uncertainty::exact());
    assert_eq!(
        answer.evidence[0].source_url,
        "https://example.com/market-time-test-source"
    );
    assert_eq!(
        answer.dataset_revisions,
        vec![DatasetRevisionId::new("test-r1").expect("valid revision id")]
    );
}
