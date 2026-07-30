//! End-to-end checks on the shell itself.
//!
//! The unit tests prove the engine answers correctly. These prove the thing a person
//! actually runs does not lose the answer on the way out: unknown stays unknown, evidence
//! stays attached, and a bad invocation fails loudly instead of printing something
//! plausible.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../market-time-data/fixtures/synthetic-venues.json")
        .display()
        .to_string()
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_market-time"))
        .args(args)
        .output()
        .expect("the binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("output is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("output is UTF-8")
}

#[test]
fn phase_answers_with_its_evidence_and_revisions() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-AUCT",
        "--at",
        "2026-07-30T02:00:00Z",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(text.contains("continuous_trading"), "{text}");
    assert!(
        text.contains("source   https://synthetic.test/auct/trading-rules"),
        "an answer reaches the document behind it: {text}"
    );
    assert!(
        text.contains("revision synthetic-2026-07-30"),
        "and names the revision that produced it: {text}"
    );
}

#[test]
fn a_mid_day_break_is_not_reported_as_closed() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-AUCT",
        "--at",
        "2026-07-30T04:00:00Z",
    ]);
    let text = stdout(&output);
    assert!(text.contains("mid_day_break"), "{text}");
    assert!(!text.contains(": closed"), "{text}");
}

#[test]
fn outside_coverage_the_shell_says_not_known() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-AUCT",
        "--at",
        "2030-01-01T00:00:00Z",
    ]);
    assert!(
        output.status.success(),
        "an unknown is an answer, not a failure"
    );

    let text = stdout(&output);
    assert!(text.contains("not known"), "{text}");
    assert!(
        text.contains("an unknown is not a closed market"),
        "the distinction survives to the surface: {text}"
    );
    assert!(!text.contains("closed\n"), "{text}");
}

#[test]
fn a_process_start_boundary_never_prints_as_exact() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-DST",
        "--at",
        "2026-07-15T14:00:00Z",
    ]);
    let text = stdout(&output);
    assert!(text.contains("continuous_trading"), "{text}");
    assert!(
        text.contains("spread not published"),
        "the open is a process, and the shell says so: {text}"
    );
}

#[test]
fn a_venue_published_bound_survives_to_the_surface() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-ALWAYS",
        "--at",
        "2026-06-14T08:00:00Z",
    ]);
    let text = stdout(&output);
    assert!(text.contains("funding_settlement"), "{text}");
    assert!(
        text.contains("venue-published bound 15s"),
        "the bound is the venue's, carried unchanged: {text}"
    );
}

#[test]
fn the_board_draws_a_row_per_venue_with_a_key() {
    let output = run(&[
        "board",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T02:00:00Z",
        "--zone",
        "Asia/Shanghai",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    for venue in ["SYNTH-ALWAYS", "SYNTH-AUCT", "SYNTH-DST"] {
        assert!(text.contains(venue), "{text}");
    }
    assert!(text.contains("axis: Asia/Shanghai"), "{text}");
    assert!(
        text.contains("? not known"),
        "the key names unknown: {text}"
    );
}

#[test]
fn the_timeline_tiles_the_window_it_was_asked_for() {
    let output = run(&[
        "timeline",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-AUCT",
        "--at",
        "2026-07-29T16:00:00Z",
        "--hours",
        "24",
    ]);
    let text = stdout(&output);
    assert!(text.contains("tiles the queried interval: true"), "{text}");
    assert!(text.contains("mid_day_break"), "{text}");
}

#[test]
fn venues_lists_what_the_dataset_covers() {
    let output = run(&["venues", "--dataset", &fixture()]);
    let text = stdout(&output);
    assert_eq!(text.trim().lines().count(), 3, "{text}");
}

#[test]
fn a_bad_invocation_fails_loudly() {
    let missing_dataset = run(&["phase"]);
    assert!(!missing_dataset.status.success());
    assert!(stderr(&missing_dataset).contains("--dataset"));

    let unknown_flag = run(&["phase", "--dataset", &fixture(), "--nonsense", "1"]);
    assert!(!unknown_flag.status.success());
    assert!(stderr(&unknown_flag).contains("unknown option"));

    let bad_instant = run(&["phase", "--dataset", &fixture(), "--at", "yesterday"]);
    assert!(!bad_instant.status.success());
    assert!(stderr(&bad_instant).contains("RFC 3339"));

    let blank_venue = run(&["phase", "--dataset", &fixture(), "--venue", "   "]);
    assert!(!blank_venue.status.success());
    assert!(
        stderr(&blank_venue).contains("must not be empty"),
        "a blank identifier is rejected rather than answered: {}",
        stderr(&blank_venue)
    );
}

#[test]
fn help_is_available_without_a_dataset() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("USAGE"));
}

#[test]
fn evidence_reaches_the_document_and_says_when_a_rule_was_derived() {
    let output = run(&[
        "evidence",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-DST",
        "--at",
        "2026-11-27T19:00:00Z",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(text.contains("phase    post_close"), "{text}");
    assert!(
        text.contains("https://synthetic.test/dst/half-days"),
        "{text}"
    );
    assert!(
        text.contains("derived  the notice gives the close"),
        "derived is never presented as observed: {text}"
    );
    assert!(text.contains("revision synthetic-2026-07-30"), "{text}");
}

#[test]
fn evidence_for_an_unknown_stretch_offers_no_phase() {
    let output = run(&[
        "evidence",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-AUCT",
        "--at",
        "2030-01-01T00:00:00Z",
        "--format",
        "json",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json output parses");
    assert!(value["phase"].is_null(), "{value}");
    assert!(
        value["not_known_because"]
            .as_str()
            .is_some_and(|reason| reason.contains("coverage")),
        "{value}"
    );
    assert_eq!(value["sources"].as_array().map(Vec::len), Some(0));
}

#[test]
fn json_keeps_unknown_and_uncertainty_as_fields() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T02:00:00Z",
        "--format",
        "json",
    ]);
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json output parses");

    let venues = value["venues"].as_array().expect("a list of venues");
    assert_eq!(venues.len(), 3, "{value}");

    let dst = venues
        .iter()
        .find(|entry| entry["venue"] == "SYNTH-DST")
        .expect("SYNTH-DST is in the dataset");
    assert!(dst["uncertainty"].is_string(), "{dst}");
    assert!(
        dst["sources"].as_array().is_some_and(|s| !s.is_empty()),
        "{dst}"
    );

    let always = venues
        .iter()
        .find(|entry| entry["venue"] == "SYNTH-ALWAYS")
        .expect("SYNTH-ALWAYS is in the dataset");
    assert_eq!(always["phase"], "continuous_trading", "{always}");
}

#[test]
fn a_timeline_can_be_read_by_a_machine() {
    let output = run(&[
        "timeline",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-DST",
        "--at",
        "2026-12-30T12:00:00Z",
        "--hours",
        "48",
        "--format",
        "json",
    ]);
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json output parses");

    assert_eq!(value["tiles_interval"], true, "{value}");
    let segments = value["segments"].as_array().expect("segments");
    assert!(
        segments.iter().any(|segment| segment["known"] == false),
        "the stretch past coverage is a segment with known=false, not a hole: {value}"
    );
    assert!(
        segments
            .iter()
            .all(|segment| segment["phase"].is_string() || segment["phase"].is_null()),
        "{value}"
    );
}

#[test]
fn an_unusable_format_is_refused() {
    let output = run(&["phase", "--dataset", &fixture(), "--format", "yaml"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("is not text, json, or svg"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_wall_clock_time_can_be_read_in_a_zone() {
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-DST",
        "--at",
        "2026-07-15T09:30:00",
        "--at-zone",
        "America/New_York",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("continuous_trading"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_wall_clock_time_that_occurs_twice_is_refused_with_both_candidates() {
    // 2026-11-01: New York clocks go back at 02:00 local.
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-DST",
        "--at",
        "2026-11-01T01:30:00",
        "--at-zone",
        "America/New_York",
    ]);
    assert!(!output.status.success(), "a coin flip is not an answer");

    let message = stderr(&output);
    assert!(message.contains("occurs twice"), "{message}");
    assert!(message.contains("2026-11-01T05:30:00Z"), "{message}");
    assert!(message.contains("2026-11-01T06:30:00Z"), "{message}");
}

#[test]
fn a_wall_clock_time_that_never_happens_is_refused() {
    // 2026-03-08: New York clocks jump 02:00 -> 03:00.
    let output = run(&[
        "phase",
        "--dataset",
        &fixture(),
        "--venue",
        "SYNTH-DST",
        "--at",
        "2026-03-08T02:30:00",
        "--at-zone",
        "America/New_York",
    ]);
    assert!(!output.status.success());

    let message = stderr(&output);
    assert!(message.contains("does not exist"), "{message}");
    assert!(
        message.contains("No instant bears that reading"),
        "{message}"
    );
}

#[test]
fn the_board_can_be_written_as_svg() {
    let output = run(&[
        "board",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T02:00:00Z",
        "--zone",
        "Asia/Shanghai",
        "--format",
        "svg",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let svg = stdout(&output);
    assert!(
        svg.trim_start().starts_with("<svg"),
        "{}",
        &svg[..60.min(svg.len())]
    );
    assert!(svg.contains("</svg>"));
    assert!(
        svg.contains("Synthetic Auction Exchange"),
        "a row per venue, under its display name"
    );
    assert!(svg.contains("EQUITIES"), "grouped by asset family");
    assert!(
        svg.contains("an unknown is not a closed market"),
        "the honesty note travels with the picture"
    );
}

#[test]
fn bands_text_names_every_bands_derivation_and_reports_a_real_overlap() {
    let output = run(&[
        "bands",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T00:00:00Z",
        "--hours",
        "24",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(
        text.contains("band band-regional-equities: Regional Equities Session"),
        "{text}"
    );
    assert!(
        text.contains("not any real desk's grouping"),
        "the band's derivation note is printed, not just its id: {text}"
    );
    assert!(
        text.contains("band band-continuous-markets: Continuous Markets"),
        "{text}"
    );
    assert!(
        text.contains("overlaps"),
        "two bands means an overlap section: {text}"
    );
    assert!(
        text.contains("computed as the overlap of session bands"),
        "the overlap's own derivation note is printed too, so it cannot read as published: {text}"
    );
    assert!(
        text.contains("overlapping"),
        "the two synthetic day sessions plus the always-on venue must produce a real \
         overlap window: {text}"
    );
    assert!(
        text.contains("not overlapping"),
        "and real gaps too — this must not read as one 24-hour block: {text}"
    );
}

#[test]
fn bands_json_marks_every_band_and_overlap_derived_and_never_renders_a_null_uncertainty_as_exact() {
    let output = run(&[
        "bands",
        "--dataset",
        &fixture(),
        "--at",
        "2030-01-01T00:00:00Z",
        "--hours",
        "24",
        "--format",
        "json",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("json output parses");

    let bands = value["bands"].as_array().expect("a list of bands");
    assert_eq!(bands.len(), 2, "{value}");
    for band in bands {
        assert_eq!(band["derived"], true, "{band}");
        assert!(
            band["reasoning"].as_str().is_some_and(|r| !r.is_empty()),
            "{band}"
        );
    }

    let overlaps = value["overlaps"].as_array().expect("a list of overlaps");
    assert_eq!(
        overlaps.len(),
        1,
        "exactly one pair from two bands: {value}"
    );
    assert_eq!(overlaps[0]["derived"], true, "{value}");

    // 2030 is past every synthetic venue's declared coverage, so every segment here is
    // unknown, and every member is unknown too: `BandSegment::uncertainty` is `None` for
    // exactly that stretch (see its doc comment), which must reach the JSON as `null`,
    // never dropped and never printed as though it were exact.
    let regional = bands
        .iter()
        .find(|band| band["id"] == "band-regional-equities")
        .expect("band-regional-equities is in the output");
    let segments = regional["segments"].as_array().expect("segments");
    assert!(
        segments
            .iter()
            .any(|segment| segment["state"] == "unknown" && segment["uncertainty"].is_null()),
        "{value}"
    );
    assert!(
        !stdout(&output).contains("\"uncertainty\": \"exact\""),
        "at this instant every synthetic venue is out of coverage, so nothing here is \
         known precisely enough to say exact; a null uncertainty must never render as it"
    );
}

#[test]
fn selecting_one_band_prints_no_overlap_section() {
    let output = run(&[
        "bands",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T00:00:00Z",
        "--hours",
        "24",
        "--band",
        "band-continuous-markets",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(text.contains("band-continuous-markets"), "{text}");
    assert!(
        !text.contains("overlaps"),
        "a lone band has no pair to overlap with: {text}"
    );

    let json_output = run(&[
        "bands",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T00:00:00Z",
        "--hours",
        "24",
        "--band",
        "band-continuous-markets",
        "--format",
        "json",
    ]);
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&json_output)).expect("json output parses");
    assert_eq!(value["bands"].as_array().map(Vec::len), Some(1), "{value}");
    assert_eq!(
        value["overlaps"].as_array().map(Vec::len),
        Some(0),
        "{value}"
    );
}

#[test]
fn the_board_shows_the_dataset_bands_by_default() {
    let output = run(&[
        "board",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T04:00:00Z",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(text.contains("DERIVED BANDS"), "{text}");
    assert!(text.contains("DERIVED OVERLAPS"), "{text}");
    assert!(text.contains("band-regional-equities"), "{text}");
    assert!(text.contains("band-continuous-markets"), "{text}");
    assert!(text.contains("(derived)"), "{text}");
    assert!(
        text.contains("not any real desk's grouping"),
        "the band's own derivation note reaches the board, not just its id: {text}"
    );
}

#[test]
fn no_bands_suppresses_the_band_section_entirely() {
    let output = run(&[
        "board",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T04:00:00Z",
        "--no-bands",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let text = stdout(&output);
    assert!(!text.contains("DERIVED"), "{text}");
    assert!(!text.contains("band-regional-equities"), "{text}");
    assert!(!text.contains("band-continuous-markets"), "{text}");
}

#[test]
fn the_svg_board_shows_the_band_section_and_stays_well_formed() {
    let output = run(&[
        "board",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T04:00:00Z",
        "--format",
        "svg",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let svg = stdout(&output);
    assert!(svg.trim_start().starts_with("<svg"), "{svg}");
    assert!(svg.contains("</svg>"), "{svg}");
    assert!(svg.contains("DERIVED SESSION BANDS"), "{svg}");
    assert!(svg.contains("DERIVED OVERLAPS"), "{svg}");
    assert!(svg.contains("(derived)"), "{svg}");
}

#[test]
fn no_bands_suppresses_the_svg_band_section_too() {
    let output = run(&[
        "board",
        "--dataset",
        &fixture(),
        "--at",
        "2026-07-30T04:00:00Z",
        "--format",
        "svg",
        "--no-bands",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let svg = stdout(&output);
    assert!(svg.trim_start().starts_with("<svg"), "{svg}");
    assert!(svg.contains("</svg>"), "{svg}");
    assert!(!svg.contains("DERIVED"), "{svg}");
}

#[test]
fn an_unknown_band_id_fails_loudly() {
    let output = run(&["bands", "--dataset", &fixture(), "--band", "no-such-band"]);
    assert!(
        !output.status.success(),
        "an unknown band must not be silently ignored"
    );
    assert!(
        stderr(&output).contains("no band named"),
        "{}",
        stderr(&output)
    );
}
