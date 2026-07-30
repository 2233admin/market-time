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
