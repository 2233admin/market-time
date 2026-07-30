//! The shipped fixture must load, and a dataset that breaks the rules must not.

use market_time_core::UtcInstant;
use market_time_core::VenueId;
use market_time_core::{PhaseOutcome, resolve_phase};
use market_time_data::{LoadError, load_ruleset, parse_ruleset};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic-venues.json")
}

fn instant(text: &str) -> UtcInstant {
    let timestamp: jiff::Timestamp = text.parse().expect("test instant parses");
    UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond())
}

#[test]
fn the_shipped_fixture_loads_and_answers() {
    let ruleset = load_ruleset(&fixture_path()).expect("the shipped fixture is a valid dataset");
    assert_eq!(ruleset.venues().len(), 3);

    let outcome = resolve_phase(
        instant("2026-07-30T02:00:00Z"),
        &VenueId::new("SYNTH-AUCT").expect("valid identifier"),
        &ruleset,
    );
    assert_eq!(
        outcome.phase().map(|p| p.as_str()),
        Some("continuous_trading")
    );
}

#[test]
fn a_dataset_may_not_invent_a_phase_name() {
    let text = r#"{
      "revisions": [{"id": "r1", "assembled_at": "2026-01-01T00:00:00Z"}],
      "venues": [{
        "venue": "V", "home_zone": "UTC",
        "coverage": {"start": "2026-01-01T00:00:00Z", "end": "2026-01-02T00:00:00Z"},
        "rules": [{
          "kind": {"type": "weekly_pattern", "weekdays": ["thu"]},
          "schedule": [{"at": "00:00", "phase": "night_session"}],
          "applies": {"start": "2026-01-01", "end": "2026-01-01"},
          "boundary_uncertainty": {"type": "exact"},
          "evidence": {"source_url": "https://synthetic.test/v", "fetched_at": "2026-01-01T00:00:00Z", "effective_from": "2026-01-01"},
          "revision": "r1"
        }]
      }]
    }"#;

    assert_eq!(
        parse_ruleset(text).err(),
        Some(LoadError::UnknownPhase {
            phase: "night_session".to_owned()
        })
    );
}

#[test]
fn a_rule_without_evidence_does_not_load() {
    let text = r#"{
      "revisions": [{"id": "r1", "assembled_at": "2026-01-01T00:00:00Z"}],
      "venues": [{
        "venue": "V", "home_zone": "UTC",
        "coverage": {"start": "2026-01-01T00:00:00Z", "end": "2026-01-02T00:00:00Z"},
        "rules": [{
          "kind": {"type": "weekly_pattern", "weekdays": ["thu"]},
          "schedule": [{"at": "00:00", "phase": "closed"}],
          "applies": {"start": "2026-01-01", "end": "2026-01-01"},
          "boundary_uncertainty": {"type": "exact"},
          "evidence": {"source_url": "  ", "fetched_at": "2026-01-01T00:00:00Z", "effective_from": "2026-01-01"},
          "revision": "r1"
        }]
      }]
    }"#;

    assert!(
        matches!(parse_ruleset(text), Err(LoadError::Invalid(_))),
        "a rule with no source document must not load, however obviously correct it looks"
    );
}

#[test]
fn coverage_the_rules_cannot_answer_is_rejected() {
    // Coverage claims two days; the rules describe one.
    let text = r#"{
      "revisions": [{"id": "r1", "assembled_at": "2026-01-01T00:00:00Z"}],
      "venues": [{
        "venue": "V", "home_zone": "UTC",
        "coverage": {"start": "2026-01-01T00:00:00Z", "end": "2026-01-03T00:00:00Z"},
        "rules": [{
          "kind": {"type": "weekly_pattern", "weekdays": ["thu"]},
          "schedule": [{"at": "00:00", "phase": "closed"}],
          "applies": {"start": "2026-01-01", "end": "2026-01-01"},
          "boundary_uncertainty": {"type": "exact"},
          "evidence": {"source_url": "https://synthetic.test/v", "fetched_at": "2026-01-01T00:00:00Z", "effective_from": "2026-01-01"},
          "revision": "r1"
        }]
      }]
    }"#;

    assert!(
        matches!(parse_ruleset(text), Err(LoadError::Invalid(_))),
        "coverage that promises more than the rules can answer is rejected before any query"
    );
}

#[test]
fn an_unknown_outcome_survives_the_round_trip() {
    let ruleset = load_ruleset(&fixture_path()).expect("fixture loads");
    let outcome = resolve_phase(
        instant("2030-01-01T00:00:00Z"),
        &VenueId::new("SYNTH-AUCT").expect("valid identifier"),
        &ruleset,
    );
    assert!(matches!(outcome, PhaseOutcome::Unknown(_)));
}
