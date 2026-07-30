//! The SSE adapter, exercised on documents shaped like SSE's publications.
//!
//! **The times below are invented.** They are in the form SSE publishes — Chinese session
//! labels, `至` and dash separators, several ranges on the continuous-auction line — because
//! that is what the parser has to handle. They are not SSE's schedule, which is not in this
//! repository and is not going to be (`DATA-LICENSING.md`). A test that restated the real
//! table would be redistributing it in test clothing.

use market_time_core::{Phase, PhaseOutcome, UtcInstant, VenueId, resolve_phase};
use market_time_data::adapters::sse::{self, GapRuling, SseError};
use market_time_data::format::{DateRangeRecord, RuleKindRecord, RuleRecord, UncertaintyRecord};
use market_time_data::{FetchedDocument, RevisionAssembly, SourceRegistration};

fn document(body: &str) -> FetchedDocument {
    let source = SourceRegistration::new(
        "https://synthetic.test/sse-shaped/trading-rules",
        "invented for this test; no venue terms apply",
    )
    .expect("valid registration");
    FetchedDocument::new(
        source,
        instant("2026-07-30T00:00:00Z"),
        body.as_bytes().to_vec(),
    )
}

fn instant(text: &str) -> UtcInstant {
    let timestamp: jiff::Timestamp = text.parse().expect("test instant parses");
    UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond())
}

/// A document in SSE's published shape, with invented times.
const SHAPED_LIKE_SSE: &str = "\
第二章 交易时间

10:15 至 10:25 开盘集合竞价时间
10:35 至 12:00、13:30 至 15:30 连续竞价时间
15:30 至 15:40 收盘集合竞价时间
15:50 至 16:20 盘后固定价格交易时间
";

#[test]
fn published_sessions_are_read_with_their_own_labels() {
    let sessions = sse::parse_sessions(&document(SHAPED_LIKE_SSE)).expect("the document parses");

    let phases: Vec<Phase> = sessions.iter().map(|session| session.phase).collect();
    assert_eq!(
        phases,
        vec![
            Phase::OpeningAuction,
            Phase::ContinuousTrading,
            Phase::ContinuousTrading,
            Phase::ClosingAuction,
            Phase::PostClose,
        ],
        "the label mapping is the whole of what this adapter asserts about the venue"
    );
    assert!(
        sessions
            .iter()
            .any(|session| session.label == "开盘集合竞价"),
        "the venue's own wording is kept, not replaced by ours"
    );
}

#[test]
fn a_line_with_two_ranges_yields_two_sessions() {
    let sessions = sse::parse_sessions(&document(SHAPED_LIKE_SSE)).expect("parses");
    let continuous: Vec<_> = sessions
        .iter()
        .filter(|session| session.phase == Phase::ContinuousTrading)
        .collect();

    assert_eq!(
        continuous.len(),
        2,
        "the continuous line carries two windows"
    );
    assert!(
        continuous[0].end < continuous[1].start,
        "with a break between"
    );
}

#[test]
fn a_gap_between_published_sessions_is_refused_not_filled() {
    let sessions = sse::parse_sessions(&document(SHAPED_LIKE_SSE)).expect("parses");
    let error = sse::day_schedule(&sessions, &[]).expect_err("the table does not tile the day");

    let SseError::UnassignedIntervals { intervals } = &error else {
        panic!("expected unassigned intervals, got {error}");
    };
    assert!(
        intervals.iter().any(|interval| interval.contains("10:25")),
        "the interval between the opening auction and continuous trading is named: {intervals:?}"
    );
    assert!(
        error.to_string().contains("not to be filled by inference"),
        "{error}"
    );
    assert!(
        intervals.iter().any(|interval| interval.contains("12:00")),
        "even the lunch break needs a ruling: the venue publishes two continuous windows          and does not label what sits between them — {intervals:?}"
    );
}

#[test]
fn a_ruling_settles_a_gap_and_says_why() {
    let sessions = sse::parse_sessions(&document(SHAPED_LIKE_SSE)).expect("parses");
    let rulings = rulings();

    let schedule = sse::day_schedule(&sessions, &rulings).expect("every gap is ruled on");
    assert_eq!(
        schedule.first().map(|point| point.at.as_str()),
        Some("00:00")
    );

    let note = sse::derivation_note(&rulings).expect("rulings produce a note");
    assert!(note.contains("no order acceptance"), "{note}");
    assert!(
        note.contains("10:25"),
        "the note names the interval it settles: {note}"
    );
}

#[test]
fn overlapping_sessions_are_an_error_rather_than_a_silent_pick() {
    let overlapping = "\
10:15 至 11:00 开盘集合竞价时间
10:30 至 12:00 连续竞价时间
";
    let error = sse::parse_sessions(&document(overlapping)).expect_err("overlap is refused");
    assert!(matches!(error, SseError::Overlapping { .. }), "{error}");
}

#[test]
fn a_document_that_changed_shape_asks_for_a_person() {
    let error = sse::parse_sessions(&document("欢迎访问本所网站")).expect_err("nothing to parse");
    assert!(matches!(error, SseError::NoSessions), "{error}");
    assert!(error.to_string().contains("a person"), "{error}");
}

#[test]
fn parsed_sessions_become_a_revision_that_answers_questions() {
    let fetched = document(SHAPED_LIKE_SSE);
    let sessions = sse::parse_sessions(&fetched).expect("parses");
    let rulings = rulings();
    let schedule = sse::day_schedule(&sessions, &rulings).expect("ruled");

    let assembly = RevisionAssembly::new("sse-shaped-2026-07", instant("2026-07-30T01:00:00Z"));
    let rule = RuleRecord {
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
        schedule,
        applies: DateRangeRecord {
            start: "2026-08-01".to_owned(),
            end: "2026-08-31".to_owned(),
        },
        boundary_uncertainty: UncertaintyRecord::PublishedGranularity { seconds: 60 },
        evidence: sse::evidence(&fetched, "2026-08-01"),
        derived_reasoning: sse::derivation_note(&rulings),
        revision: String::new(),
    };

    let venue = assembly.venue(
        "SSE-SHAPED",
        "Asia/Shanghai",
        (
            instant("2026-07-31T16:00:00Z"),
            instant("2026-08-31T16:00:00Z"),
        ),
        vec![sse::evidence(&fetched, "2026-08-01")],
        vec![rule],
        Vec::new(),
    );

    let json = assembly.with_venue(venue).to_json();
    let ruleset = market_time_data::parse_ruleset(&json).expect("the assembled revision loads");

    // 11:00 Shanghai on 3 August, inside the invented continuous window.
    let outcome = resolve_phase(
        instant("2026-08-03T03:00:00Z"),
        &VenueId::new("SSE-SHAPED").expect("valid id"),
        &ruleset,
    );
    let PhaseOutcome::Known(answer) = outcome else {
        panic!("the revision covers that instant");
    };
    assert_eq!(answer.phase, Phase::ContinuousTrading);
    assert!(
        answer.derived_reasoning.is_some(),
        "a day that needed a ruling says so, rather than passing as the venue's own wording"
    );
    assert!(
        answer
            .evidence
            .iter()
            .any(|evidence| evidence.source_url().contains("trading-rules")),
        "and the answer traces to the document the sessions were read from"
    );
}

/// Rulings in the shape the real ones take, with the reasoning carried.
///
/// The real SSE rulings — what 09:25–09:30 and 15:00–15:05 are, and why — are recorded in
/// `docs/venue-session-state/research.md` under D4b and D4c, sourced from the Trading
/// Rules. They are supplied by the operator at assembly time rather than compiled in here.
fn rulings() -> Vec<GapRuling> {
    vec![
        GapRuling {
            start: jiff::civil::Time::constant(12, 0, 0, 0),
            end: jiff::civil::Time::constant(13, 30, 0, 0),
            phase: Phase::MidDayBreak,
            reasoning: "the two continuous windows are published separately and nothing is                         accepted between them; the venue does not label the interval itself"
                .to_owned(),
        },
        GapRuling {
            start: jiff::civil::Time::constant(10, 25, 0, 0),
            end: jiff::civil::Time::constant(10, 35, 0, 0),
            phase: Phase::Closed,
            reasoning: "no order acceptance, no matching, no quotations — three provisions \
                        draw the boundary and none includes this interval"
                .to_owned(),
        },
        GapRuling {
            start: jiff::civil::Time::constant(15, 40, 0, 0),
            end: jiff::civil::Time::constant(15, 50, 0, 0),
            phase: Phase::PreOpen,
            reasoning: "orders are accepted and cancellable but not matched, ahead of the \
                        fixed-price session"
                .to_owned(),
        },
    ]
}
