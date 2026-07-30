//! What the board must never do to an answer.

use market_time_board::{
    BandSection, BoardRow, BoardView, ClockDiscipline, NowMarker, band_glyph, glyph, overlap_glyph,
    render,
};
use market_time_core::Phase;
use market_time_core::VenueId;
use market_time_core::{BandDefinition, BandId, BandState, DerivationNote, OverlapState};
use market_time_core::{Interval, UtcInstant};
use market_time_core::{PhaseOutcome, Ruleset, derive_band, derive_overlap, resolve_timeline};
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
        bands: BandSection::default(),
    }
}

/// A board over the fixture's three venues, with two real derived bands and their
/// overlap attached — SYNTH-AUCT and SYNTH-DST grouped as one band, SYNTH-ALWAYS alone as
/// the other, exactly as the CLI's own fixture bands are shaped (see
/// `synthetic-venues.json`'s `_comment`), so these tests exercise the same derivation the
/// core already proves correct rather than a hand-rolled stand-in for it.
fn banded_view(zone: &str, start: &str, end: &str, now: Option<&str>) -> BoardView {
    let mut board = view(zone, start, end, now);
    let ruleset = ruleset();

    let auct = VenueId::new("SYNTH-AUCT").expect("valid identifier");
    let dst = VenueId::new("SYNTH-DST").expect("valid identifier");
    let always = VenueId::new("SYNTH-ALWAYS").expect("valid identifier");

    let regional = BandDefinition::new(
        BandId::new("band-test-regional").expect("valid identifier"),
        "Test Regional",
        vec![auct.clone(), dst.clone()],
        DerivationNote::new("test fixture grouping, not any real desk's").expect("non-empty"),
    )
    .expect("valid definition");
    let continuous = BandDefinition::new(
        BandId::new("band-test-continuous").expect("valid identifier"),
        "Test Continuous",
        vec![always.clone()],
        DerivationNote::new("test fixture single-member band").expect("non-empty"),
    )
    .expect("valid definition");

    let regional_timelines = vec![
        resolve_timeline(board.interval, &auct, &ruleset),
        resolve_timeline(board.interval, &dst, &ruleset),
    ];
    let continuous_timelines = vec![resolve_timeline(board.interval, &always, &ruleset)];

    let regional_band =
        derive_band(&regional, &regional_timelines).expect("the fixture's timelines derive");
    let continuous_band =
        derive_band(&continuous, &continuous_timelines).expect("the fixture's timelines derive");
    let overlap =
        derive_overlap(&regional_band, &continuous_band).expect("two distinct bands overlap");

    board.bands = BandSection {
        bands: vec![regional_band, continuous_band],
        overlaps: vec![overlap],
    };
    board
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
        bands: BandSection::default(),
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
        bands: BandSection::default(),
    });

    assert!(!svg.contains("<script>"), "the tag never lands as markup");
    assert!(
        svg.contains("&lt;/text&gt;&lt;script&gt;"),
        "it lands as text"
    );
}

#[test]
fn a_view_with_zero_bands_renders_exactly_as_before_the_band_field_existed() {
    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T02:00:00Z"),
    );
    assert!(board.bands.bands.is_empty(), "the helper carries no bands");

    let text = render(&board);
    let svg = market_time_board::render_svg(&board);

    // The guarantee this test exists to pin down: a caller that never mentions bands gets
    // a board byte-for-byte the same as it always was. Nothing about the band feature may
    // leak into a zero-band render.
    assert!(!text.contains("DERIVED"), "{text}");
    assert!(!text.contains("bands key"), "{text}");
    assert!(!svg.contains("DERIVED"), "{svg}");

    // And rebuilding the same board with an explicitly empty `BandSection` — the only
    // other way to spell "no bands" — must render identically, not merely similarly.
    let mut rebuilt = board.clone();
    rebuilt.bands = BandSection {
        bands: Vec::new(),
        overlaps: Vec::new(),
    };
    assert_eq!(
        text,
        render(&rebuilt),
        "empty is empty however it was built"
    );
    assert_eq!(
        svg,
        market_time_board::render_svg(&rebuilt),
        "same for the SVG renderer"
    );
}

#[test]
fn band_and_overlap_glyphs_never_collide_with_a_phase_glyph() {
    let phases = [
        Phase::Closed,
        Phase::PreOpen,
        Phase::OpeningAuction,
        Phase::ClosingAuction,
        Phase::ContinuousTrading,
        Phase::MidDayBreak,
        Phase::PostClose,
        Phase::NonTradingInterruption,
    ];
    let band_glyphs = [
        band_glyph(BandState::Trading),
        band_glyph(BandState::NotTrading),
        band_glyph(BandState::Unknown),
    ];
    let overlap_glyphs = [
        overlap_glyph(OverlapState::Overlapping),
        overlap_glyph(OverlapState::NotOverlapping),
        overlap_glyph(OverlapState::Unknown),
    ];

    for derived_glyph in band_glyphs.iter().chain(overlap_glyphs.iter()) {
        assert_ne!(
            *derived_glyph, '?',
            "not-known's glyph is reserved for a venue's own coverage gap, never a band's"
        );
        for phase in phases {
            assert_ne!(
                *derived_glyph,
                glyph(phase),
                "a band/overlap glyph must never read as a venue's phase glyph: {derived_glyph:?} vs {phase:?}"
            );
        }
    }
}

#[test]
fn band_rows_are_labelled_derived_and_show_their_own_reasoning() {
    let board = banded_view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T04:00:00Z"),
    );
    let rendered = render(&board);

    assert!(
        rendered.contains("DERIVED BANDS"),
        "a heading marks the section: {rendered}"
    );
    assert!(
        rendered.contains("DERIVED OVERLAPS"),
        "and the overlap section too: {rendered}"
    );

    let band_line = rendered
        .lines()
        .find(|line| line.contains("band-test-regional"))
        .expect("the band row is printed");
    assert!(
        band_line.contains("(derived)"),
        "a band row cannot be mistaken for a venue row: {band_line}"
    );
    assert!(
        rendered.contains("test fixture grouping, not any real desk's"),
        "the band's own derivation note is printed, not just its id: {rendered}"
    );
    assert!(
        rendered.contains("computed as the overlap of session bands"),
        "the overlap's own derivation note is printed too: {rendered}"
    );
}

#[test]
fn the_svg_board_hatches_an_unknown_band_stretch_with_the_same_pattern() {
    // Past every synthetic venue's declared coverage: both members of the regional band
    // are unknown throughout, so the whole band — and its overlap — reads Unknown here.
    let board = banded_view("UTC", "2027-02-01T00:00:00Z", "2027-02-02T00:00:00Z", None);
    let svg = market_time_board::render_svg(&board);

    let heading_at = svg
        .find("DERIVED SESSION BANDS")
        .expect("the band section is drawn");
    let hatch_at = svg
        .rfind("url(#not-known)")
        .expect("the hatch pattern is used somewhere in the document");

    assert!(
        hatch_at > heading_at,
        "an unknown band stretch is hatched with the venue rows' own not-known pattern, \
         inside the band section itself: {svg}"
    );
}
