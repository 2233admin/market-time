//! The operator path, end to end: register a source, fetch it, assemble a revision from
//! what came back, load that revision, and answer a question with it.
//!
//! This is the shape the licensing position forces (`DATA-LICENSING.md`): no venue data
//! lives here, the operator fetches under their own relationship with the venue, and the
//! tooling turns what they fetched into something evidenced. The "venue" below is invented
//! for the test, as everything venue-shaped in this repository is.

use market_time_core::UtcInstant;
use market_time_core::{PhaseOutcome, VenueId, resolve_phase};
use market_time_data::format::{
    ChangePointRecord, DateRangeRecord, RuleKindRecord, RuleRecord, UncertaintyRecord,
};
use market_time_data::{
    AssemblyError, FileFetcher, RevisionAssembly, SourceFetcher, SourceRegistration, evidence_from,
    load_ruleset,
};
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("market-time-operator-path-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

fn instant(text: &str) -> UtcInstant {
    let timestamp: jiff::Timestamp = text.parse().expect("test instant parses");
    UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond())
}

fn day_rule() -> RuleRecord {
    RuleRecord {
        kind: RuleKindRecord::WeeklyPattern {
            weekdays: vec![
                "mon".to_owned(),
                "tue".to_owned(),
                "wed".to_owned(),
                "thu".to_owned(),
                "fri".to_owned(),
                "sat".to_owned(),
                "sun".to_owned(),
            ],
        },
        schedule: vec![
            ChangePointRecord {
                at: "00:00".to_owned(),
                phase: "closed".to_owned(),
            },
            ChangePointRecord {
                at: "09:30".to_owned(),
                phase: "continuous_trading".to_owned(),
            },
            ChangePointRecord {
                at: "16:00".to_owned(),
                phase: "post_close".to_owned(),
            },
        ],
        applies: DateRangeRecord {
            start: "2026-01-01".to_owned(),
            end: "2026-01-31".to_owned(),
        },
        boundary_uncertainty: UncertaintyRecord::PublishedGranularity { seconds: 60 },
        evidence: placeholder_evidence(),
        derived_reasoning: None,
        revision: String::new(), // filled in by the assembly
    }
}

fn placeholder_evidence() -> market_time_data::format::EvidenceRecord {
    market_time_data::format::EvidenceRecord {
        source_url: "https://operator.invalid/placeholder".to_owned(),
        fetched_at: "2026-01-01T00:00:00Z".to_owned(),
        effective_from: "2026-01-01".to_owned(),
        publisher_last_changed: None,
        terms: None,
        digest: None,
    }
}

#[test]
fn fetch_assemble_load_answer() {
    let dir = scratch("happy");

    // 1. The operator already has the document, however they obtained it.
    std::fs::write(dir.join("hours.txt"), "09:30-16:00, every day\n").expect("write document");

    // 2. Register the source with the terms it may be used under, before fetching.
    let source = SourceRegistration::new(
        "https://synthetic.test/operator/hours.txt",
        "invented for this test; no venue terms apply",
    )
    .expect("valid registration")
    .with_note("terms recorded at registration, per DATA-LICENSING.md");

    // 3. Fetch, recording when.
    let fetched = FileFetcher::new(&dir)
        .fetch(&source, instant("2026-01-02T00:00:00Z"))
        .expect("the document is where the operator put it");
    assert!(fetched.digest().starts_with("sha256:"));
    assert_eq!(fetched.text().expect("utf-8"), "09:30-16:00, every day\n");

    // 4. Assemble a revision whose evidence comes from the retrieval, not from typing.
    let assembly = RevisionAssembly::new("operator-2026-01", instant("2026-01-02T00:05:00Z"));
    let mut rule = day_rule();
    rule.evidence = evidence_from(&fetched, "2026-01-01");

    let venue = assembly.venue(
        "OPER-1",
        "UTC",
        (
            instant("2026-01-01T00:00:00Z"),
            instant("2026-02-01T00:00:00Z"),
        ),
        vec![evidence_from(&fetched, "2026-01-01")],
        vec![rule],
        Vec::new(),
    );
    let assembly = assembly.with_venue(venue);

    let path = dir.join("revision.json");
    assembly.write(&path).expect("the revision is valid");

    // 5. Load it back and answer a question with it.
    let ruleset = load_ruleset(&path).expect("the written revision loads");
    let outcome = resolve_phase(
        instant("2026-01-05T10:00:00Z"),
        &VenueId::new("OPER-1").expect("valid id"),
        &ruleset,
    );

    let PhaseOutcome::Known(answer) = outcome else {
        panic!("the revision covers that instant");
    };
    assert_eq!(answer.phase.as_str(), "continuous_trading");
    assert!(
        answer
            .evidence
            .iter()
            .any(|evidence| evidence.source_url().contains("operator/hours.txt")),
        "the answer traces back to the document that was fetched"
    );
}

#[test]
fn the_terms_and_the_digest_reach_the_written_revision() {
    let dir = scratch("provenance");
    std::fs::write(dir.join("hours.txt"), "whatever").expect("write document");

    let source = SourceRegistration::new(
        "https://synthetic.test/operator/hours.txt",
        "personal, non-commercial use only",
    )
    .expect("valid registration");
    let fetched = FileFetcher::new(&dir)
        .fetch(&source, instant("2026-01-02T00:00:00Z"))
        .expect("fetch");

    let evidence = evidence_from(&fetched, "2026-01-01");
    assert_eq!(
        evidence.terms.as_deref(),
        Some("personal, non-commercial use only"),
        "the terms recorded at registration travel into the dataset"
    );
    assert_eq!(evidence.digest.as_deref(), Some(fetched.digest()));

    let assembly = RevisionAssembly::new("operator-2026-01", instant("2026-01-02T00:05:00Z"));
    let mut rule = day_rule();
    rule.evidence = evidence.clone();
    let venue = assembly.venue(
        "OPER-1",
        "UTC",
        (
            instant("2026-01-01T00:00:00Z"),
            instant("2026-02-01T00:00:00Z"),
        ),
        vec![evidence],
        vec![rule],
        Vec::new(),
    );

    let json = assembly.with_venue(venue).to_json();
    assert!(json.contains("personal, non-commercial use only"), "{json}");
    assert!(json.contains("sha256:"), "{json}");
}

#[test]
fn a_revision_the_loader_would_reject_is_never_written() {
    let dir = scratch("invalid");
    let path = dir.join("revision.json");

    // Coverage claims a month; the rules describe a single day.
    let assembly = RevisionAssembly::new("operator-2026-01", instant("2026-01-02T00:00:00Z"));
    let mut rule = day_rule();
    rule.applies = DateRangeRecord {
        start: "2026-01-01".to_owned(),
        end: "2026-01-01".to_owned(),
    };
    let venue = assembly.venue(
        "OPER-1",
        "UTC",
        (
            instant("2026-01-01T00:00:00Z"),
            instant("2026-02-01T00:00:00Z"),
        ),
        Vec::new(),
        vec![rule],
        Vec::new(),
    );

    let result = assembly.with_venue(venue).write(&path);
    assert!(
        matches!(result, Err(AssemblyError::Invalid(_))),
        "{result:?}"
    );
    assert!(
        !path.exists(),
        "a dataset the engine refuses to read is worse than no dataset, because it looks \
         like data"
    );
}

#[test]
fn rules_cite_the_revision_they_were_assembled_into() {
    let assembly = RevisionAssembly::new("operator-2026-02", instant("2026-02-01T00:00:00Z"))
        .superseding("operator-2026-01");
    let venue = assembly.venue(
        "OPER-1",
        "UTC",
        (
            instant("2026-01-01T00:00:00Z"),
            instant("2026-02-01T00:00:00Z"),
        ),
        Vec::new(),
        vec![day_rule()],
        Vec::new(),
    );

    assert_eq!(venue.rules[0].revision, "operator-2026-02");
    assert!(
        assembly
            .to_json()
            .contains("\"supersedes\": \"operator-2026-01\"")
    );
}

#[test]
fn a_missing_document_names_where_the_fetcher_looked() {
    let dir = scratch("missing");
    let source = SourceRegistration::new("https://synthetic.test/operator/absent.txt", "terms")
        .expect("valid registration");

    let error = FileFetcher::new(&dir)
        .fetch(&source, instant("2026-01-02T00:00:00Z"))
        .expect_err("the document is not there");

    assert!(error.to_string().contains("absent.txt"), "{error}");
}
