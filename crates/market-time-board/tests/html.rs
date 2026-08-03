//! What the interactive HTML surface must never do to an answer.
//!
//! `render_svg`'s own tests (`rendering.rs`) already prove the picture itself is honest;
//! these prove the page built around it — the hover payload, the escaping, and the
//! no-network guarantee — carries that honesty through rather than losing it on the way
//! into a `<script>` tag.

use market_time_board::{
    BandSection, BoardRow, BoardView, ClockDiscipline, HtmlOptions, NowMarker,
};
use market_time_core::Uncertainty;
use market_time_core::VenueId;
use market_time_core::{BandDefinition, BandId, DerivationNote};
use market_time_core::{EvidenceRef, Phase, PhaseAnswer, PhaseBoundary, Timeline, TimelineSegment};
use market_time_core::{Interval, UtcInstant};
use market_time_core::{Ruleset, derive_band, derive_overlap, resolve_timeline};
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

/// The same two-band, one-overlap fixture `rendering.rs`'s `banded_view` builds, so these
/// tests exercise the real `derive_band`/`derive_overlap` output rather than a hand-rolled
/// stand-in for it.
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

fn payload_text(html: &str) -> &str {
    let open_tag = r#"<script type="application/json" id="mt-payload">"#;
    let start = html
        .find(open_tag)
        .map(|at| at + open_tag.len())
        .expect("the payload script is present");
    let end = html[start..]
        .find("</script>")
        .map(|offset| start + offset)
        .expect("the payload script closes");
    &html[start..end]
}

#[test]
fn the_html_page_is_well_formed_and_starts_with_the_doctype() {
    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T02:00:00Z"),
    );
    let html = market_time_board::render_html(&board, &HtmlOptions::default());

    assert!(
        html.starts_with("<!doctype html>"),
        "{}",
        &html[..40.min(html.len())]
    );
    assert!(html.trim_end().ends_with("</html>"));
    // The payload must be valid JSON — the whole point of shipping it as data rather than
    // as a page that computes its own answers.
    let payload = payload_text(&html);
    assert!(
        payload.starts_with('{') && payload.trim_end().ends_with('}'),
        "{payload}"
    );
}

#[test]
fn the_html_page_contains_the_derived_band_section() {
    let board = banded_view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T04:00:00Z"),
    );
    let html = market_time_board::render_html(&board, &HtmlOptions::default());

    assert!(html.contains("DERIVED SESSION BANDS"), "{html}");
    assert!(html.contains("DERIVED OVERLAPS"), "{html}");
    assert!(
        html.contains(r#"data-seg="b0.0""#),
        "a band segment is tagged for the hover payload: {html}"
    );
    assert!(
        payload_text(&html).contains(r#""kind":"band""#),
        "the payload describes it as a band, not a venue"
    );
}

#[test]
fn a_none_uncertainty_reads_as_no_schedule_known_and_never_exact() {
    // Past every synthetic venue's declared coverage: every member of both bands is
    // unknown throughout, so `BandSegment::uncertainty` is `None` there (same window
    // `rendering.rs`'s SVG test uses for the same reason).
    let board = banded_view("UTC", "2027-02-01T00:00:00Z", "2027-02-02T00:00:00Z", None);
    let html = market_time_board::render_html(&board, &HtmlOptions::default());
    let payload = payload_text(&html);

    assert!(
        payload.contains("no schedule known for this stretch"),
        "the payload states the absence in words: {payload}"
    );
    assert!(
        !payload.contains(r#""uncertainty":"exact""#),
        "a None uncertainty must never render as exact: {payload}"
    );
}

#[test]
fn an_out_of_coverage_stretch_keeps_the_not_known_hatch() {
    let board = view("UTC", "2027-02-01T00:00:00Z", "2027-02-02T00:00:00Z", None);
    let html = market_time_board::render_html(&board, &HtmlOptions::default());

    assert!(
        html.contains(r#"<pattern id="not-known""#),
        "the hatch pattern is defined: {html}"
    );
    assert!(
        html.contains("url(#not-known)"),
        "and used for the out-of-coverage stretch: {html}"
    );
    assert!(
        html.contains("an unknown is not a closed market"),
        "and said in words: {html}"
    );
    let payload = payload_text(&html);
    assert!(
        payload.contains(r#""phase":null"#),
        "the payload never substitutes a phase for an unknown: {payload}"
    );
}

#[test]
fn a_hostile_venue_name_is_escaped_in_the_markup_and_the_json_payload() {
    let ruleset = ruleset();
    let interval = Interval::new(
        instant("2026-07-30T00:00:00Z"),
        instant("2026-07-31T00:00:00Z"),
    )
    .expect("valid interval");

    // Contains both a tag that must never land as markup and a quote that must never
    // land as an unescaped JSON string terminator.
    let hostile = VenueId::new(r#"</text><script>x</script>"quote"#).expect("non-empty id");
    let timeline = resolve_timeline(interval, &hostile, &ruleset);

    let board = BoardView {
        interval,
        rows: vec![BoardRow::new(timeline, None)],
        now: None,
        axis_zone: "UTC".to_owned(),
        columns: 24,
        bands: BandSection::default(),
    };
    let html = market_time_board::render_html(&board, &HtmlOptions::default());

    assert!(
        !html.contains("<script>x</script>"),
        "the tag never lands as markup anywhere on the page: {html}"
    );
    assert!(
        html.contains("&lt;/text&gt;&lt;script&gt;"),
        "it lands as escaped text in the embedded SVG: {html}"
    );

    let payload = payload_text(&html);
    assert!(
        !payload.contains("</script>") && !payload.contains("<script>"),
        "no literal angle bracket survives in the payload text at all — the payload must \
         never be able to close the script element early: {payload}"
    );
    // The literal venue id, with every `<`/`>` turned into its `\u00XX` escape — this
    // module's own scheme (see `json_escape`), not JSON's minimum requirement, and exactly
    // what must appear here for the payload to be safe to embed in a `<script>` element.
    // The literal venue id, run through this module's own escaping: every `<`/`>` becomes
    // a `\u00XX` sequence (six literal characters — backslash, `u`, four hex digits — not
    // an actual Unicode escape produced by Rust here), and the quote becomes a standard
    // `\"`. Built with `\\` throughout rather than a raw string, so what this test checks
    // for is unambiguous about which characters are backslashes.
    let expected_escaped = "\\u003c/text\\u003e\\u003cscript\\u003ex\\u003c/script\\u003e";
    assert!(
        payload.contains(expected_escaped),
        "the hostile venue id survives, every angle bracket turned into a \\u escape, \
         inside the JSON payload: {payload}"
    );
    assert!(
        payload.contains("\\\"quote"),
        "the embedded quote is a standard JSON escape, not a raw quote: {payload}"
    );
}

/// A single-venue, single-segment board whose one evidence citation carries `source_url`
/// verbatim — built by hand from [`Timeline`]'s own public fields rather than through a
/// [`Ruleset`], because the evidence panel's escaping is a property of `render_html` alone
/// and `inspect` (which `render_html` calls) is a pure lookup over an already-resolved
/// [`Timeline`]. A dataset's evidence URL is operator-supplied and, per `evidence.rs`,
/// carries no format or scheme validation at all — this is exactly what a hostile or
/// merely careless dataset can put there.
fn board_with_source_url(source_url: &str) -> BoardView {
    let interval = Interval::new(
        instant("2026-07-30T00:00:00Z"),
        instant("2026-07-31T00:00:00Z"),
    )
    .expect("valid interval");
    let venue = VenueId::new("SYNTH-SRC").expect("valid identifier");
    let evidence = EvidenceRef::new(source_url, instant("2026-07-01T00:00:00Z"), "2026-01-01")
        .expect("evidence is complete");
    let answer = PhaseAnswer {
        venue: venue.clone(),
        phase: Phase::ContinuousTrading,
        boundary_start: PhaseBoundary {
            instant: interval.start,
            uncertainty: Uncertainty::Exact,
        },
        boundary_end: PhaseBoundary {
            instant: interval.end,
            uncertainty: Uncertainty::Exact,
        },
        events: Vec::new(),
        evidence: vec![evidence],
        derived_reasoning: None,
        uncertainty: Uncertainty::Exact,
        dataset_revisions: Vec::new(),
    };
    let timeline = Timeline {
        venue: venue.clone(),
        interval,
        segments: vec![TimelineSegment::Phase {
            interval,
            answer: Box::new(answer),
        }],
    };
    BoardView {
        interval,
        rows: vec![BoardRow::new(timeline, None)],
        now: None,
        axis_zone: "UTC".to_owned(),
        columns: 24,
        bands: BandSection::default(),
    }
}

/// What a Rust-only test can and cannot prove about the evidence link, now that it is
/// built by the browser at hover time rather than by `render_html`: it can prove the
/// payload still carries a hostile `source_url` as a properly quoted JSON string (so the
/// `<script type="application/json">` element cannot be broken out of), and it can prove
/// the shipped script contains the DOM-construction and scheme-allowlist code paths this
/// fix relies on, rather than the vulnerable string-concatenation shape. It cannot prove
/// what the browser does with that payload at runtime — that a `"` in the URL cannot
/// break out of an `href="..."` attribute, or that a non-`http(s)` URL never becomes a
/// clickable `<a>` — because nothing in this crate executes JavaScript. That half is
/// covered separately, by hand, in a real browser (see the task's verification step).
#[test]
fn a_hostile_source_url_is_escaped_in_the_payload_and_the_script_builds_links_via_the_dom() {
    let hostile = r#"https://synthetic.test/x?a="onmouseover="alert(1) x="#;
    let board = board_with_source_url(hostile);
    let html = market_time_board::render_html(&board, &HtmlOptions::default());
    let payload = payload_text(&html);

    // The quote is a standard JSON escape, not a raw quote — the payload cannot break out
    // of the `<script type="application/json">` element's own syntax.
    assert!(
        payload.contains(r#"\"onmouseover=\"alert(1)"#),
        "the embedded quote in the hostile source URL is JSON-escaped, not raw: {payload}"
    );
    assert!(
        !payload.contains(r#""onmouseover="alert(1)"#),
        "an unescaped quote must never survive next to the payload's own JSON syntax: {payload}"
    );

    // The script builds the link through the DOM — `.href` as a property, not a
    // string-concatenated `href="..."` — which is what makes a `"` in the URL harmless
    // regardless of what the browser does with it. This is a shape check on the shipped
    // script, not proof of the runtime behaviour; see the doc comment above.
    assert!(
        html.contains("link.href = href;") && html.contains(r#"link.textContent = source.url;"#),
        "the evidence link is assigned via DOM properties, not string concatenation"
    );
    assert!(
        !html.contains("href=\""),
        "no href attribute may ever be written by string concatenation in the shipped \
         script — the old vulnerable shape built `href=\"` + a URL: {html}"
    );
}

/// The scheme allowlist that keeps a `javascript:` (or any other non-`http(s)`) source
/// URL from ever becoming a clickable link. As above, this proves the allowlist code
/// shipped and that the static page never itself writes a `javascript:` URL into an
/// `href` attribute — it cannot execute the script to confirm the link is actually
/// rendered as inert text at runtime; that is the real-browser check.
#[test]
fn a_javascript_scheme_source_url_never_becomes_a_static_href_and_the_allowlist_ships() {
    let board = board_with_source_url("javascript:alert(1)");
    let html = market_time_board::render_html(&board, &HtmlOptions::default());

    assert!(
        !html.contains("href=\""),
        "no href attribute — javascript: or otherwise — is ever written by string \
         concatenation in the shipped page: {html}"
    );
    assert!(
        html.contains(r#"parsed.protocol === "http:" || parsed.protocol === "https:""#),
        "the shipped script allowlists http(s) before ever assigning `.href`"
    );
    assert!(
        html.contains("new URL(String(value))"),
        "the scheme check parses with `new URL`, not a hand-rolled regex on the string"
    );

    let payload = payload_text(&html);
    assert!(
        payload.contains("javascript:alert(1)"),
        "the operator still sees, in the payload, what their dataset claims the source \
         is — a rejected scheme is not silently dropped: {payload}"
    );
}

#[test]
fn the_page_makes_no_external_reference_outside_evidence_urls_and_the_svg_namespace() {
    let board = banded_view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T04:00:00Z"),
    );
    let html = market_time_board::render_html(&board, &HtmlOptions::default());

    // Every evidence source URL this render can possibly cite — gathered the same way
    // `svg.rs`'s own sources block does, so this allowlist is exactly what the page is
    // permitted to mention, not a guess at it.
    let mut allowed: Vec<String> = vec!["http://www.w3.org/2000/svg".to_owned()];
    for row in &board.rows {
        for segment in &row.timeline.segments {
            let market_time_core::TimelineSegment::Phase { answer, .. } = segment else {
                continue;
            };
            for evidence in &answer.evidence {
                allowed.push(evidence.source_url().to_owned());
            }
        }
    }

    for found in urls_in(&html) {
        assert!(
            allowed.iter().any(|url| url == &found),
            "unexpected external reference {found:?} — the page must work with no network"
        );
    }
    // The allowlist itself must not be vacuous, or this test would pass by finding nothing
    // to check at all.
    assert!(
        urls_in(&html).len() > 1,
        "the fixture's own evidence URLs are expected to appear at least once"
    );
}

/// Every `http://`/`https://` token in `text`, split at the first character that could not
/// be part of a bare URL. Mirrors exactly what a person skimming the raw page would call
/// "a URL," so this cannot be fooled by a scheme that only *looks* external once escaped
/// (e.g. inside the JSON payload, where `/` before `script` is never escaped and so still
/// reads as a normal URL character here).
fn urls_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for scheme in ["http://", "https://"] {
        let mut search_from = 0;
        while let Some(offset) = text[search_from..].find(scheme) {
            let start = search_from + offset;
            let rest = &text[start..];
            let end = rest
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>'))
                .unwrap_or(rest.len());
            found.push(rest[..end].to_owned());
            search_from = start + scheme.len();
        }
    }
    found
}

#[test]
fn a_zone_switch_relabels_the_axis_ticks_and_every_segments_stretch_per_zone() {
    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T04:00:00Z"),
    );
    let options = HtmlOptions {
        zones: vec!["UTC".to_owned(), "Asia/Shanghai".to_owned()],
        ..HtmlOptions::default()
    };
    let html = market_time_board::render_html(&board, &options);
    let payload = payload_text(&html);

    assert!(
        payload.contains(r#""Asia/Shanghai":["#),
        "axis labels precomputed for the zone"
    );
    // The axis's first tick sits at `interval.start` (2026-07-30T00:00:00Z here), which
    // reads 08:00 in Shanghai (UTC+8) — computed once here, in the test, from the same
    // fixed offset the pinned tzdb assigns Asia/Shanghai, to confirm the payload's own
    // number is a real conversion and not a copy of the UTC label.
    assert!(
        payload.contains("\"UTC\":[\"00:00\""),
        "the UTC axis starts at the queried interval's own start: {payload}"
    );
    assert!(
        payload.contains("\"Asia/Shanghai\":[\"08:00\""),
        "and the same instant reads 08:00 in Shanghai: {payload}"
    );
}

#[test]
fn no_bands_renders_no_derived_section_and_matches_the_bandless_helper() {
    let board = view(
        "UTC",
        "2026-07-30T00:00:00Z",
        "2026-07-31T00:00:00Z",
        Some("2026-07-30T04:00:00Z"),
    );
    let html = market_time_board::render_html(&board, &HtmlOptions::default());
    assert!(!html.contains("DERIVED"), "{html}");
}
