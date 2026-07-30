//! What the board must never do to an answer.

use market_time_board::{BoardView, ClockDiscipline, NowMarker, glyph, render};
use market_time_core::Phase;
use market_time_core::VenueId;
use market_time_core::{Interval, UtcInstant};
use market_time_core::{PhaseOutcome, Ruleset, resolve_timeline};
use market_time_data::load_ruleset;
use std::path::PathBuf;

fn ruleset() -> Ruleset {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../market-time-data/fixtures/synthetic-venues.json");
    load_ruleset(&path).expect("the synthetic fixture loads")
}

fn instant(text: &str) -> UtcInstant {
    let timestamp: jiff::Timestamp = text.parse().expect("test instant parses");
    UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond())
}

fn view(zone: &str, start: &str, end: &str, now: Option<&str>) -> BoardView {
    let ruleset = ruleset();
    let interval = Interval::new(instant(start), instant(end)).expect("valid interval");
    BoardView {
        interval,
        rows: ruleset
            .venues()
            .iter()
            .map(|venue| resolve_timeline(interval, venue, &ruleset))
            .collect(),
        now: now.map(|text| NowMarker {
            instant: instant(text),
            discipline: ClockDiscipline::Given {
                source: "test".to_owned(),
            },
        }),
        axis_zone: zone.to_owned(),
        columns: 48,
    }
}

#[test]
fn unknown_never_renders_as_closed() {
    assert_ne!(
        glyph(Phase::Closed),
        '?',
        "the closed glyph must not be the unknown glyph"
    );

    // A window entirely past SYNTH-DST's coverage.
    let board = view("UTC", "2027-02-01T00:00:00Z", "2027-02-02T00:00:00Z", None);
    let rendered = render(&board);
    let dst_row = rendered
        .lines()
        .find(|line| line.starts_with("SYNTH-DST"))
        .expect("the board draws a row per venue");

    assert!(
        dst_row.contains('?'),
        "an out-of-coverage stretch renders as not-known: {dst_row}"
    );
    assert!(
        !dst_row.contains(glyph(Phase::Closed)),
        "and never as closed: {dst_row}"
    );
}

#[test]
fn changing_the_axis_zone_moves_no_segment() {
    let start = "2026-07-30T00:00:00Z";
    let end = "2026-07-31T00:00:00Z";

    let utc = view("UTC", start, end, None);
    let shanghai = view("Asia/Shanghai", start, end, None);

    for (left, right) in utc.rows.iter().zip(shanghai.rows.iter()) {
        assert_eq!(
            left.segments.len(),
            right.segments.len(),
            "the same interval yields the same segments whatever the axis is labelled in"
        );
        for (a, b) in left.segments.iter().zip(right.segments.iter()) {
            assert_eq!(
                a.interval(),
                b.interval(),
                "segments occupy the same instants"
            );
            assert_eq!(a.phase(), b.phase());
        }
    }

    // Only the labelling differs.
    assert_ne!(render(&utc), render(&shanghai));
}

#[test]
fn the_now_marker_reports_the_clock_bound() {
    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T02:00:00Z"),
    );
    let rendered = render(&board);

    assert!(
        rendered.contains("now:"),
        "the board says which instant it drew"
    );
    assert!(
        rendered.contains("not read from a clock"),
        "and how well that instant is known: {rendered}"
    );
}

#[test]
fn an_unmeasured_clock_is_never_reported_as_exact() {
    let describe = ClockDiscipline::Unmeasured {
        source: "host system clock".to_owned(),
    }
    .describe();

    assert!(describe.contains("unmeasured"));
    assert!(!describe.contains("0.0ms"), "silence is not a measurement");
}

#[test]
fn the_board_draws_only_what_the_core_returned() {
    // A venue the ruleset has no entry for produces a single unknown row: the board has
    // no schedule of its own to fall back on.
    let ruleset = ruleset();
    let interval = Interval::new(
        instant("2026-07-30T00:00:00Z"),
        instant("2026-07-31T00:00:00Z"),
    )
    .expect("valid interval");

    let timeline = resolve_timeline(
        interval,
        &VenueId::new("SYNTH-ABSENT").expect("valid identifier"),
        &ruleset,
    );
    assert!(timeline.segments.iter().all(|segment| segment.is_unknown()));

    let board = BoardView {
        interval,
        rows: vec![timeline],
        now: None,
        axis_zone: "UTC".to_owned(),
        columns: 24,
    };
    let rendered = render(&board);
    assert!(rendered.contains("SYNTH-ABSENT"));
    assert!(rendered.contains('?'));
}

#[test]
fn a_phase_answer_and_the_board_agree() {
    let ruleset = ruleset();
    let at = instant("2026-07-30T04:00:00Z");
    let outcome = market_time_core::resolve_phase(
        at,
        &VenueId::new("SYNTH-AUCT").expect("valid identifier"),
        &ruleset,
    );
    let PhaseOutcome::Known(answer) = outcome else {
        panic!("the fixture covers this instant");
    };

    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T04:00:00Z"),
    );
    let rendered = render(&board);
    let row = rendered
        .lines()
        .find(|line| line.starts_with("SYNTH-AUCT"))
        .expect("row present");

    assert!(
        row.ends_with(answer.phase.as_str()),
        "the status the board shows is the phase the core returned: {row}"
    );
}
