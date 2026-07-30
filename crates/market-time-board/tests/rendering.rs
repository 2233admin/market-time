//! What the board must never do to an answer.

use market_time_board::{BoardRow, BoardView, ClockDiscipline, NowMarker, glyph, render};
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
            .map(|venue| {
                BoardRow::new(
                    resolve_timeline(interval, venue, &ruleset),
                    ruleset.profile(venue),
                )
            })
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
        .find(|line| line.starts_with("Synthetic Daylight"))
        .expect("the board draws a row per venue, under its display name");

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
            left.timeline.segments.len(),
            right.timeline.segments.len(),
            "the same interval yields the same segments whatever the axis is labelled in"
        );
        for (a, b) in left
            .timeline
            .segments
            .iter()
            .zip(right.timeline.segments.iter())
        {
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
        rows: vec![BoardRow::new(timeline, None)],
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
        .find(|line| line.starts_with("Synthetic Auction"))
        .expect("row present");

    assert!(
        row.ends_with(answer.phase.as_str()),
        "the status the board shows is the phase the core returned: {row}"
    );
}

#[test]
fn a_segment_can_be_inspected_for_what_it_rests_on() {
    let ruleset = ruleset();
    let interval = Interval::new(
        instant("2026-11-27T00:00:00Z"),
        instant("2026-11-28T00:00:00Z"),
    )
    .expect("valid interval");
    let timeline = resolve_timeline(interval, &VenueId::new("SYNTH-DST").expect("id"), &ruleset);

    let detail = market_time_board::inspect(&timeline, instant("2026-11-27T19:00:00Z"))
        .expect("the row covers that instant");

    assert_eq!(detail.phase, Some(Phase::PostClose));
    assert!(
        detail.sources.iter().any(|s| s.url.contains("half-days")),
        "the segment reaches the document behind it: {detail:?}"
    );
    assert!(
        detail.derived_reasoning.is_some(),
        "a derived rule says so at the point of inspection, not only in the core"
    );
    assert!(detail.start_uncertainty.is_some());
    assert!(!detail.dataset_revisions.is_empty());
}

#[test]
fn inspecting_an_unknown_stretch_says_why_and_offers_no_phase() {
    let ruleset = ruleset();
    let interval = Interval::new(
        instant("2027-02-01T00:00:00Z"),
        instant("2027-02-02T00:00:00Z"),
    )
    .expect("valid interval");
    let timeline = resolve_timeline(interval, &VenueId::new("SYNTH-DST").expect("id"), &ruleset);

    let detail = market_time_board::inspect(&timeline, instant("2027-02-01T12:00:00Z"))
        .expect("the row covers that instant as an unknown");

    assert_eq!(detail.phase, None, "an unknown offers no phase to misread");
    assert!(
        detail
            .not_known_because
            .as_deref()
            .is_some_and(|reason| reason.contains("coverage")),
        "{detail:?}"
    );
    assert!(
        detail.sources.is_empty(),
        "there is no document to cite for a gap"
    );
}

#[test]
fn the_board_prints_the_documents_its_rows_rest_on() {
    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T02:00:00Z"),
    );
    let rendered = render(&board);

    assert!(rendered.contains("sources for SYNTH-AUCT:"), "{rendered}");
    assert!(
        rendered.contains("https://synthetic.test/auct/trading-rules"),
        "a viewer reaches the source without leaving the board: {rendered}"
    );
}

#[test]
fn the_svg_board_marks_unknown_with_a_pattern_not_a_paler_shade() {
    let board = view("UTC", "2027-02-01T00:00:00Z", "2027-02-02T00:00:00Z", None);
    let svg = market_time_board::render_svg(&board);

    assert!(svg.starts_with("<svg"), "a self-contained document");
    assert!(svg.ends_with("</svg>"));
    assert!(
        svg.contains(r#"<pattern id="not-known""#),
        "the hatch is defined"
    );
    assert!(
        svg.contains("url(#not-known)"),
        "and used for the out-of-coverage stretch"
    );
    assert!(
        svg.contains("an unknown is not a closed market"),
        "and said in words, because colour is never the only channel"
    );
}

#[test]
fn the_svg_board_softens_a_process_start_boundary() {
    let board = view(
        "UTC",
        "2026-07-15T00:00:00Z",
        "2026-07-16T00:00:00Z",
        Some("2026-07-15T14:00:00Z"),
    );
    let svg = market_time_board::render_svg(&board);

    assert!(
        svg.contains("url(#process-start)"),
        "SYNTH-DST's open is a process, and the edge says so"
    );
    assert!(
        svg.contains("spread not published"),
        "the hover text carries the uncertainty verbatim"
    );
}

#[test]
fn the_svg_board_carries_the_clock_note_and_the_sources() {
    let board = view(
        "Asia/Shanghai",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T02:00:00Z"),
    );
    let svg = market_time_board::render_svg(&board);

    assert!(svg.contains("not read from a clock"), "the clock note");
    assert!(svg.contains("axis in Asia/Shanghai"), "the axis zone");
    assert!(
        svg.contains("https://synthetic.test/auct/trading-rules"),
        "the documents the rows rest on"
    );
    assert!(svg.contains("continuous trading"), "the status in words");
}

#[test]
fn svg_text_from_data_cannot_close_a_tag() {
    let ruleset = ruleset();
    let interval = Interval::new(
        instant("2026-07-30T00:00:00Z"),
        instant("2026-07-31T00:00:00Z"),
    )
    .expect("valid interval");

    // A venue id nobody should ever use, which is exactly why the renderer must survive it.
    let hostile = VenueId::new("</text><script>x</script>").expect("non-empty id");
    let timeline = resolve_timeline(interval, &hostile, &ruleset);

    let svg = market_time_board::render_svg(&BoardView {
        interval,
        rows: vec![BoardRow::new(timeline, None)],
        now: None,
        axis_zone: "UTC".to_owned(),
        columns: 24,
    });

    assert!(!svg.contains("<script>"), "the tag never lands as markup");
    assert!(
        svg.contains("&lt;/text&gt;&lt;script&gt;"),
        "it lands as text"
    );
}
