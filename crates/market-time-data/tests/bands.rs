//! Session bands, loaded from a dataset file.
//!
//! The engine's own golden vectors (`market-time-core/tests/bands.rs`) prove `derive_band`
//! and `derive_overlap` are correct once given a `BandDefinition`. These prove the loader
//! builds that `BandDefinition` faithfully from JSON, and refuses everything the core or
//! the loader itself would refuse: a member venue the file does not declare, an empty or
//! self-duplicating member list, a blank reasoning or id, and two bands sharing one id.

use market_time_core::resolve_timeline;
use market_time_core::{BandState, OverlapState, VenueId, derive_band, derive_overlap};
use market_time_data::{LoadError, load_dataset, parse_dataset};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic-venues.json")
}

fn instant(text: &str) -> market_time_core::UtcInstant {
    let timestamp: jiff::Timestamp = text.parse().expect("test instant parses");
    market_time_core::UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond())
}

/// A two-venue dataset, both always trading, with `bands_json` spliced in as the file's
/// `bands` array — just enough scaffolding to exercise the band loader without dragging in
/// the fixture's more elaborate schedules.
fn dataset_with_bands(bands_json: &str) -> String {
    format!(
        r#"{{
          "revisions": [{{"id": "r1", "assembled_at": "2026-01-01T00:00:00Z"}}],
          "venues": [
            {{
              "venue": "V1", "home_zone": "UTC",
              "coverage": {{"start": "2026-01-01T00:00:00Z", "end": "2026-01-08T00:00:00Z"}},
              "rules": [{{
                "kind": {{"type": "weekly_pattern", "weekdays": ["mon","tue","wed","thu","fri","sat","sun"]}},
                "schedule": [{{"at": "00:00", "phase": "continuous_trading"}}],
                "applies": {{"start": "2026-01-01", "end": "2026-01-07"}},
                "boundary_uncertainty": {{"type": "exact"}},
                "evidence": {{"source_url": "https://synthetic.test/v1", "fetched_at": "2026-01-01T00:00:00Z", "effective_from": "2026-01-01"}},
                "revision": "r1"
              }}]
            }},
            {{
              "venue": "V2", "home_zone": "UTC",
              "coverage": {{"start": "2026-01-01T00:00:00Z", "end": "2026-01-08T00:00:00Z"}},
              "rules": [{{
                "kind": {{"type": "weekly_pattern", "weekdays": ["mon","tue","wed","thu","fri","sat","sun"]}},
                "schedule": [{{"at": "00:00", "phase": "continuous_trading"}}],
                "applies": {{"start": "2026-01-01", "end": "2026-01-07"}},
                "boundary_uncertainty": {{"type": "exact"}},
                "evidence": {{"source_url": "https://synthetic.test/v2", "fetched_at": "2026-01-01T00:00:00Z", "effective_from": "2026-01-01"}},
                "revision": "r1"
              }}]
            }}
          ],
          "bands": [{bands_json}]
        }}"#
    )
}

// --------------------------------------------------------------------- the shipped fixture

#[test]
fn the_fixtures_bands_load_and_derive_a_real_overlap() {
    let dataset = load_dataset(&fixture_path()).expect("the shipped fixture is a valid dataset");
    assert_eq!(dataset.bands.len(), 2, "the fixture declares two bands");

    let regional = dataset
        .bands
        .iter()
        .find(|band| band.id().as_str() == "band-regional-equities")
        .expect("band-regional-equities is declared");
    assert_eq!(regional.display_name(), "Regional Equities Session");
    assert_eq!(regional.members().len(), 2);
    assert!(!regional.derivation().reasoning().is_empty());

    let continuous = dataset
        .bands
        .iter()
        .find(|band| band.id().as_str() == "band-continuous-markets")
        .expect("band-continuous-markets is declared");
    assert_eq!(
        continuous.members(),
        &[VenueId::new("SYNTH-ALWAYS").expect("valid identifier")]
    );

    // Derive both bands over one day and confirm the overlap is neither empty nor the
    // whole day: SYNTH-AUCT and SYNTH-DST never trade at the same UTC hour, so
    // band-regional-equities has real closed stretches, and since SYNTH-ALWAYS never
    // stops, the overlap lands exactly on band-regional-equities' own trading windows.
    let interval = market_time_core::Interval::new(
        instant("2026-07-30T00:00:00Z"),
        instant("2026-07-31T00:00:00Z"),
    )
    .expect("valid interval");

    let regional_timelines: Vec<_> = regional
        .members()
        .iter()
        .map(|venue| resolve_timeline(interval, venue, &dataset.ruleset))
        .collect();
    let regional_band = derive_band(regional, &regional_timelines).expect("band derives");

    let continuous_timelines: Vec<_> = continuous
        .members()
        .iter()
        .map(|venue| resolve_timeline(interval, venue, &dataset.ruleset))
        .collect();
    let continuous_band = derive_band(continuous, &continuous_timelines).expect("band derives");

    assert!(
        regional_band
            .segments()
            .iter()
            .any(|segment| segment.state == BandState::NotTrading),
        "the two day-session venues leave real gaps: this is not a 24-hour band"
    );

    let overlap = derive_overlap(&regional_band, &continuous_band).expect("overlap derives");
    let windows: Vec<_> = overlap.windows().collect();
    assert!(
        !windows.is_empty(),
        "the fixture's bands must produce a genuine, non-empty overlap"
    );
    assert!(
        windows.len() < overlap.segments().len(),
        "the overlap must not simply be the whole queried day"
    );
    assert!(
        overlap
            .segments()
            .iter()
            .all(|segment| segment.state != OverlapState::Unknown),
        "both bands are fully covered on this date; nothing here should read unknown"
    );
}

// -------------------------------------------------------------------------- rejections

#[test]
fn a_band_naming_an_unknown_venue_is_rejected() {
    let text = dataset_with_bands(
        r#"{"id": "b1", "display_name": "B1", "members": ["V1", "GHOST"], "derived_reasoning": "test"}"#,
    );

    assert_eq!(
        parse_dataset(&text).err(),
        Some(LoadError::UnknownBandMember {
            band: "b1".to_owned(),
            venue: "GHOST".to_owned(),
        })
    );
}

#[test]
fn a_band_with_no_members_is_rejected() {
    let text = dataset_with_bands(
        r#"{"id": "b1", "display_name": "B1", "members": [], "derived_reasoning": "test"}"#,
    );

    assert!(
        matches!(parse_dataset(&text), Err(LoadError::Invalid(_))),
        "a band with nothing to derive from is not a band"
    );
}

#[test]
fn a_band_with_a_duplicate_member_is_rejected() {
    let text = dataset_with_bands(
        r#"{"id": "b1", "display_name": "B1", "members": ["V1", "V1"], "derived_reasoning": "test"}"#,
    );

    assert!(
        matches!(parse_dataset(&text), Err(LoadError::Invalid(_))),
        "the same venue counted twice would double its vote toward the band"
    );
}

#[test]
fn a_band_with_blank_reasoning_is_rejected() {
    let text = dataset_with_bands(
        r#"{"id": "b1", "display_name": "B1", "members": ["V1"], "derived_reasoning": "   "}"#,
    );

    assert!(
        matches!(parse_dataset(&text), Err(LoadError::Invalid(_))),
        "a band with no stated reasoning cannot be expressed, same as DerivationNote::new"
    );
}

#[test]
fn a_band_with_a_blank_id_is_rejected() {
    let text = dataset_with_bands(
        r#"{"id": "   ", "display_name": "B1", "members": ["V1"], "derived_reasoning": "test"}"#,
    );

    assert!(
        matches!(parse_dataset(&text), Err(LoadError::Invalid(_))),
        "a blank identifier names nothing, same as any other id in this format"
    );
}

#[test]
fn two_bands_sharing_an_id_are_rejected() {
    let text = dataset_with_bands(
        r#"
        {"id": "b1", "display_name": "First", "members": ["V1"], "derived_reasoning": "test"},
        {"id": "b1", "display_name": "Second", "members": ["V2"], "derived_reasoning": "test"}
        "#,
    );

    assert_eq!(
        parse_dataset(&text).err(),
        Some(LoadError::DuplicateBandId {
            id: "b1".to_owned()
        })
    );
}

#[test]
fn a_dataset_with_an_empty_bands_array_loads_with_no_bands() {
    let text = dataset_with_bands("");
    let dataset = parse_dataset(&text).expect("a dataset with an empty bands array still loads");
    assert!(dataset.bands.is_empty());
}
