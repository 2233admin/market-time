//! The board as an interactive, self-contained HTML page.
//!
//! Same board, third surface — and, like [`crate::svg`], not a second drawing of it.
//! [`render_html`] embeds [`crate::svg::render_svg_with`]'s own output verbatim and layers
//! interaction on top of it, rather than re-deriving the picture in HTML/CSS: there is
//! exactly one place in this crate that decides where a segment sits on the page, and this
//! module is not it.
//!
//! # No network, ever
//!
//! The returned string is the whole page: inline `<style>`, inline `<script>`, an inline
//! JSON payload, no `<link>`, no external font, no CDN, no analytics beacon. A page that
//! phoned home would leak which venues an operator watches, which is exactly the kind of
//! fact this product exists to keep off the wire. `render_html`'s own tests assert this
//! mechanically: no `http://` or `https://` substring survives in the output except inside
//! an evidence source URL the dataset itself supplied, or the SVG namespace URI every
//! `<svg>` element carries by specification.
//!
//! # Nothing is derived twice
//!
//! Every fact the page can show on hover — a phase, a band state, an uncertainty, a source,
//! a derivation note — is read out of [`crate::inspect`] or the already-derived
//! [`market_time_core::SessionBand`]/[`market_time_core::BandOverlap`] the caller handed to
//! [`crate::BandSection`]. JavaScript here only looks values up in a JSON payload this
//! module built in Rust; it does not recompute a phase, an uncertainty, or a zone
//! conversion. In particular:
//!
//! - **Zone conversion happens once, in Rust**, against the pinned IANA database this
//!   workspace bundles, for every zone the caller offers in [`HtmlOptions::zones`]. The
//!   payload carries one precomputed label per zone for the axis ticks, the "now" line, and
//!   every segment's stretch; the zone selector only swaps which precomputed set is shown.
//!   There is no `Intl.DateTimeFormat` call and no zone-aware `Date` arithmetic anywhere in
//!   the script below, because the browser's zone data is neither pinned nor reproducible
//!   (Constitution Principle III) and this page's claims must be.
//! - **The live clock is the one deliberate exception**, and it is deliberately limited:
//!   when `view.now` came from a live host-clock read (never when it was supplied via
//!   `--at`), the script advances the *same* `<line id="mt-now-line">` this crate's SVG
//!   renderer already drew, using the browser's own `Date.now()` against the interval
//!   bounds — plain arithmetic on two already-known epoch instants, not a zone conversion.
//!   The line is permanently captioned "browser clock — discipline unmeasured" whenever it
//!   ticks, and it is never drawn with more weight than an evidenced phase boundary. A
//!   supplied `--at` draws no live line at all: the two are never allowed to be confused
//!   for each other.
//!
//! # Works with JavaScript disabled
//!
//! The board picture, the legend, the sources block, and the footer are static markup —
//! everything [`crate::svg::render_svg`] already draws, plus a static placeholder in the
//! evidence panel. Hovering for evidence, switching the axis zone, and the ticking clock
//! are enhancements layered on top; none of them is required to read the board itself.

use crate::svg::{self, SvgOptions};
use crate::{
    BoardView, ClockDiscipline, NowMarker, SegmentDetail, SourceRef, band_label, band_state_word,
    grouped, inspect, overlap_label, overlap_state_word,
};
use jiff::tz::TimeZone;
use market_time_core::{
    BandOverlap, BandSegment, Interval, OverlapSegment, SessionBand, Uncertainty, UtcInstant,
};
use std::fmt::Write as _;

/// One dataset revision named in the page's footer, so the picture is traceable to the
/// bytes it came from (Constitution Principle III).
///
/// Built by the caller from whatever loaded the dataset — the CLI reads it off
/// `market_time_core::Ruleset::revisions`, which `market_time_board` itself has no access
/// to (this crate never touches a dataset file; a [`BoardView`] is already fully resolved
/// by the time it reaches here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetRevisionFooter {
    /// The revision identifier.
    pub id: String,
    /// The IANA tzdb release the revision was prepared against, where recorded.
    pub iana_tzdb_version: Option<String>,
}

/// Layout and metadata for the interactive HTML surface.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HtmlOptions {
    /// The picture's own layout — reused unchanged, because the picture is reused
    /// unchanged.
    pub svg: SvgOptions,
    /// IANA zones the viewer may relabel the axis into, in the order offered.
    ///
    /// Ordinarily the dataset's venues' home zones, deduplicated, plus `"UTC"` — the CLI
    /// builds this list from `Ruleset`, which this crate cannot read itself. Defensively,
    /// [`render_html`] always adds `view.axis_zone` and `"UTC"` to whatever is supplied
    /// here if either is missing, so a caller that forgets one cannot ship a page whose own
    /// default zone isn't a choice in its own selector.
    pub zones: Vec<String>,
    /// The dataset revisions this render was produced from, named in the footer.
    pub dataset_revisions: Vec<DatasetRevisionFooter>,
}

/// Renders the board as a self-contained, interactive HTML page.
///
/// See the module docs for what "self-contained" and "interactive" mean here: no network
/// reference of any kind, one geometry (the embedded SVG), and nothing computed twice
/// between Rust and the browser except the live clock's own position, which is arithmetic
/// on two already-known instants rather than a zone conversion.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_html(view: &BoardView, options: &HtmlOptions) -> String {
    let zones = normalized_zones(&options.zones, &view.axis_zone);
    let svg_markup = svg::render_svg_with(view, &options.svg);

    let mut segments: Vec<(String, Json)> = Vec::new();
    let mut row_index: usize = 0;
    for (_family, members) in grouped(&view.rows) {
        for row in members {
            for (segment_index, segment) in row.timeline.segments.iter().enumerate() {
                // The start instant always belongs to this exact segment: intervals are
                // half-open `[start, end)`, so `inspect` at `start` cannot resolve to the
                // neighbour before it.
                if let Some(detail) = inspect(&row.timeline, segment.interval().start) {
                    segments.push((
                        format!("v{row_index}.{segment_index}"),
                        venue_segment_json(&detail, &row.label, &zones),
                    ));
                }
            }
            row_index += 1;
        }
    }
    for (band_index, band) in view.bands.bands.iter().enumerate() {
        for (segment_index, segment) in band.segments().iter().enumerate() {
            segments.push((
                format!("b{band_index}.{segment_index}"),
                band_segment_json(band, segment, &zones),
            ));
        }
    }
    for (overlap_index, overlap) in view.bands.overlaps.iter().enumerate() {
        for (segment_index, segment) in overlap.segments().iter().enumerate() {
            segments.push((
                format!("o{overlap_index}.{segment_index}"),
                overlap_segment_json(overlap, segment, &zones),
            ));
        }
    }

    let live_clock_eligible = matches!(
        view.now.as_ref().map(|now| &now.discipline),
        Some(ClockDiscipline::Unmeasured { .. } | ClockDiscipline::BoundedNanos { .. })
    );

    let payload = obj(vec![
        (
            "zones",
            Json::Arr(zones.iter().map(|zone| s(zone.clone())).collect()),
        ),
        ("defaultZone", s(view.axis_zone.clone())),
        ("axis", axis_labels_by_zone(&zones, view.interval)),
        (
            "interval",
            obj(vec![
                ("startMs", Json::Num(epoch_millis(view.interval.start))),
                ("endMs", Json::Num(epoch_millis(view.interval.end))),
            ]),
        ),
        ("now", now_json(view.now.as_ref(), &zones)),
        ("segments", Json::Obj(segments)),
    ]);
    let mut payload_text = String::new();
    payload.write(&mut payload_text);

    let mut page = String::new();
    page.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    page.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    page.push_str("<title>Mark Time — venue sessions</title>\n<style>\n");
    page.push_str(STYLE);
    page.push_str("\n</style>\n</head>\n<body>\n");

    page.push_str("<header class=\"mt-header\">\n<h1>Mark Time — venue sessions</h1>\n");
    page.push_str("<div class=\"mt-controls\">\n<label for=\"mt-zone-select\">axis zone</label>\n");
    let _ = write!(
        page,
        "<select id=\"mt-zone-select\">{}</select>\n</div>\n</header>\n",
        zone_options(&zones, &view.axis_zone)
    );

    page.push_str("<main class=\"mt-main\">\n<div class=\"mt-board\">\n");
    page.push_str(&svg_markup);
    page.push_str("\n</div>\n");
    page.push_str("<aside id=\"mt-evidence\" aria-live=\"polite\">\n");
    page.push_str(EVIDENCE_PLACEHOLDER);
    page.push_str("\n</aside>\n</main>\n");

    if live_clock_eligible {
        page.push_str("<p id=\"mt-live-clock\">browser clock — discipline unmeasured</p>\n");
    }

    let _ = writeln!(
        page,
        "<footer class=\"mt-footer\">{}</footer>",
        svg::escape(&footer_line(&options.dataset_revisions))
    );

    page.push_str("<script type=\"application/json\" id=\"mt-payload\">");
    page.push_str(&payload_text);
    page.push_str("</script>\n<script>\n");
    page.push_str(SCRIPT);
    page.push_str("\n</script>\n</body>\n</html>\n");

    page
}

/// `options.zones` with `view.axis_zone` and `"UTC"` guaranteed present, in the order first
/// seen, and with no repeats. See [`HtmlOptions::zones`] for why this is defensive rather
/// than the only place the list is ever assembled — the CLI already does the real work of
/// reading each venue's home zone.
fn normalized_zones(requested: &[String], axis_zone: &str) -> Vec<String> {
    let mut zones: Vec<String> = Vec::with_capacity(requested.len() + 2);
    for zone in requested
        .iter()
        .cloned()
        .chain([axis_zone.to_owned(), "UTC".to_owned()])
    {
        if !zones.iter().any(|existing| existing == &zone) {
            zones.push(zone);
        }
    }
    zones
}

fn zone_options(zones: &[String], default_zone: &str) -> String {
    let mut out = String::new();
    for zone in zones {
        let selected = if zone == default_zone {
            " selected"
        } else {
            ""
        };
        let escaped = svg::escape(zone);
        let _ = write!(
            out,
            r#"<option value="{escaped}"{selected}>{escaped}</option>"#
        );
    }
    out
}

fn footer_line(revisions: &[DatasetRevisionFooter]) -> String {
    if revisions.is_empty() {
        return "no dataset revision was recorded for this render".to_owned();
    }
    let parts: Vec<String> = revisions
        .iter()
        .map(|revision| {
            let tzdb = revision
                .iana_tzdb_version
                .as_deref()
                .unwrap_or("not recorded");
            format!("{} (IANA tzdb {tzdb})", revision.id)
        })
        .collect();
    format!("rendered from dataset revision(s): {}", parts.join("; "))
}

/// Nanoseconds since the epoch, as whole milliseconds — precise enough to place a visual
/// marker, never claimed as more than that. `i128` round-trips through JSON as a bare
/// number literal without needing `f64` at all, so there is no float precision loss to
/// reason about here (unlike the SVG renderer's pixel math, which does convert through
/// `f64` and documents why that loss is harmless there).
fn epoch_millis(at: UtcInstant) -> String {
    (at.as_nanos_since_unix_epoch() / 1_000_000).to_string()
}

fn axis_labels_by_zone(zones: &[String], interval: Interval) -> Json {
    let pairs = zones
        .iter()
        .map(|zone_name| {
            let zone = TimeZone::get(zone_name).unwrap_or(TimeZone::UTC);
            let labels = (0..=svg::AXIS_TICKS)
                .map(|tick| {
                    let fraction = svg::count_as_f64(tick) / svg::count_as_f64(svg::AXIS_TICKS);
                    let at = svg::at_fraction(interval, fraction);
                    s(svg::format_hour(&zone, at))
                })
                .collect();
            (zone_name.clone(), Json::Arr(labels))
        })
        .collect();
    Json::Obj(pairs)
}

fn stretch_by_zone(zones: &[String], interval: Interval) -> Json {
    let pairs = zones
        .iter()
        .map(|zone_name| {
            let zone = TimeZone::get(zone_name).unwrap_or(TimeZone::UTC);
            let value = obj(vec![
                ("start", s(svg::format_instant(&zone, interval.start))),
                ("end", s(svg::format_instant(&zone, interval.end))),
            ]);
            (zone_name.clone(), value)
        })
        .collect();
    Json::Obj(pairs)
}

fn now_json(now: Option<&NowMarker>, zones: &[String]) -> Json {
    let Some(now) = now else {
        return obj(vec![("present", Json::Bool(false))]);
    };
    let supplied = matches!(now.discipline, ClockDiscipline::Given { .. });
    let info_pairs = zones
        .iter()
        .map(|zone_name| {
            let zone = TimeZone::get(zone_name).unwrap_or(TimeZone::UTC);
            let text = format!(
                "{} · {}",
                svg::format_instant(&zone, now.instant),
                now.discipline.describe()
            );
            (zone_name.clone(), s(text))
        })
        .collect();
    obj(vec![
        ("present", Json::Bool(true)),
        ("supplied", Json::Bool(supplied)),
        ("discipline", s(now.discipline.describe())),
        ("info", Json::Obj(info_pairs)),
    ])
}

fn source_json(source: &SourceRef) -> Json {
    obj(vec![
        ("url", s(source.url.clone())),
        ("fetchedAt", s(source.fetched_at.clone())),
        ("effectiveFrom", s(source.effective_from.clone())),
        (
            "publisherLastChanged",
            opt_s(source.publisher_last_changed.clone()),
        ),
    ])
}

/// A venue segment's hover payload, read from [`crate::inspect`]'s answer — never a second
/// reading of the timeline.
fn venue_segment_json(detail: &SegmentDetail, venue_label: &str, zones: &[String]) -> Json {
    obj(vec![
        ("kind", s("venue")),
        ("venue", s(detail.venue.to_string())),
        ("label", s(venue_label.to_owned())),
        (
            "phase",
            detail.phase.map_or(Json::Null, |phase| s(phase.as_str())),
        ),
        ("notKnownBecause", opt_s(detail.not_known_because.clone())),
        ("stretch", stretch_by_zone(zones, detail.interval)),
        ("startUncertainty", opt_s(detail.start_uncertainty.clone())),
        ("endUncertainty", opt_s(detail.end_uncertainty.clone())),
        ("derivedReasoning", opt_s(detail.derived_reasoning.clone())),
        (
            "sources",
            Json::Arr(detail.sources.iter().map(source_json).collect()),
        ),
        (
            "datasetRevisions",
            Json::Arr(
                detail
                    .dataset_revisions
                    .iter()
                    .map(|revision| s(revision.clone()))
                    .collect(),
            ),
        ),
    ])
}

/// A derived band segment's hover payload. A band is never itself evidenced (see
/// `market_time_core::bands`' module docs), so there are no `sources` here — only the
/// band's own derivation note, already computed by `derive_band` and read verbatim.
fn band_segment_json(band: &SessionBand, segment: &BandSegment, zones: &[String]) -> Json {
    obj(vec![
        ("kind", s("band")),
        ("label", s(band_label(band))),
        ("state", s(band_state_word(segment.state))),
        ("stretch", stretch_by_zone(zones, segment.interval)),
        (
            "uncertainty",
            s(uncertainty_words(segment.uncertainty.as_ref())),
        ),
        (
            "derivedReasoning",
            s(band.definition().derivation().reasoning().to_owned()),
        ),
        (
            "tradingVenues",
            Json::Arr(
                segment
                    .venues_trading
                    .iter()
                    .map(|venue| s(venue.to_string()))
                    .collect(),
            ),
        ),
        (
            "unknownVenues",
            Json::Arr(
                segment
                    .venues_unknown
                    .iter()
                    .map(|venue| s(venue.to_string()))
                    .collect(),
            ),
        ),
    ])
}

/// The [`OverlapSegment`] counterpart of [`band_segment_json`].
fn overlap_segment_json(overlap: &BandOverlap, segment: &OverlapSegment, zones: &[String]) -> Json {
    obj(vec![
        ("kind", s("overlap")),
        ("label", s(overlap_label(overlap))),
        ("state", s(overlap_state_word(segment.state))),
        ("stretch", stretch_by_zone(zones, segment.interval)),
        (
            "uncertainty",
            s(uncertainty_words(segment.uncertainty.as_ref())),
        ),
        (
            "derivedReasoning",
            s(overlap.derivation().reasoning().to_owned()),
        ),
    ])
}

/// The phrase for a slice with no uncertainty to report at all — the same "no schedule
/// known for this stretch" wording `svg.rs` and the CLI already use, and never "exact":
/// nothing here narrows to a claim that precise (see `BandSegment::uncertainty`'s doc
/// comment).
fn uncertainty_words(uncertainty: Option<&Uncertainty>) -> String {
    match uncertainty {
        Some(uncertainty) => uncertainty.to_string(),
        None => "no schedule known for this stretch".to_owned(),
    }
}

/// A minimal JSON value, written by hand rather than through `serde_json`.
///
/// Two reasons: this crate otherwise has no JSON dependency at all (matching `svg.rs`'s own
/// "no dependency" stance on its output), and the escaping this payload needs is stricter
/// than a general-purpose JSON writer's — every `<`, `>`, `&`, and `'` is escaped as a
/// `\u00XX` sequence, not just the characters JSON syntax requires, because this text is
/// embedded as the content of a `<script>` element and must never be able to end that
/// element early however hostile the operator-supplied data inside it is.
enum Json {
    Null,
    Bool(bool),
    /// A pre-formatted numeric literal, guaranteed valid JSON by its only two call sites
    /// ([`epoch_millis`]'s `i128::to_string`).
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Num(text) => out.push_str(text),
            Self::Str(text) => {
                out.push('"');
                out.push_str(&json_escape(text));
                out.push('"');
            }
            Self::Arr(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Obj(pairs) => {
                out.push('{');
                for (index, (key, value)) in pairs.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push('"');
                    out.push_str(&json_escape(key));
                    out.push_str("\":");
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn s(text: impl Into<String>) -> Json {
    Json::Str(text.into())
}

fn opt_s(text: Option<String>) -> Json {
    text.map_or(Json::Null, Json::Str)
}

fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Obj(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// Escapes a string for embedding as a JSON string literal inside a `<script>` element's
/// text content. Standard JSON escaping (quote, backslash, and the C0 control characters)
/// plus `<`, `>`, `&`, and `'` as `\u00XX` — defence in depth against the one sequence that
/// actually matters here (`</script`), applied to every occurrence of the characters that
/// could form it rather than to that literal substring, so there is no clever splitting of
/// the tag name that gets through unescaped.
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\'' => out.push_str("\\u0027"),
            control if (u32::from(control)) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(control));
            }
            other => out.push(other),
        }
    }
    out
}

const EVIDENCE_PLACEHOLDER: &str = "<p class=\"mt-evidence-placeholder\">Hover or focus a segment on the board for its evidence.</p>";

/// Theme-aware page chrome. The embedded SVG stays the fixed dark picture `svg.rs` always
/// draws — like a screenshot, it does not repaint for the viewer's theme — but the panel,
/// controls, and footer around it do, because those are this page's own markup and the
/// constitution's "colour is never the only channel" stance extends to legibility in a
/// browser that forces light mode.
const STYLE: &str = r#"
:root {
  color-scheme: dark light;
  --mt-bg: #0d1117;
  --mt-panel: #161b22;
  --mt-border: #30363d;
  --mt-text: #c9d1d9;
  --mt-title: #e6edf3;
  --mt-muted: #8b949e;
  --mt-warn: #f78166;
  --mt-link: #58a6ff;
}
@media (prefers-color-scheme: light) {
  :root {
    --mt-bg: #f6f8fa;
    --mt-panel: #ffffff;
    --mt-border: #d0d7de;
    --mt-text: #1f2328;
    --mt-title: #0b0f14;
    --mt-muted: #57606a;
    --mt-warn: #cf222e;
    --mt-link: #0969da;
  }
}
* { box-sizing: border-box; }
body {
  margin: 0;
  padding: 24px;
  max-width: 1240px;
  margin-inline: auto;
  background: var(--mt-bg);
  color: var(--mt-text);
  font-family: ui-sans-serif, -apple-system, "Segoe UI", system-ui, sans-serif;
}
.mt-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 12px;
}
.mt-header h1 { font-size: 18px; color: var(--mt-title); margin: 0; }
.mt-controls { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--mt-muted); }
.mt-controls select {
  background: var(--mt-panel);
  color: var(--mt-text);
  border: 1px solid var(--mt-border);
  border-radius: 4px;
  padding: 2px 6px;
}
.mt-main { display: flex; flex-wrap: wrap; gap: 16px; align-items: flex-start; }
.mt-board { flex: 1 1 640px; min-width: 0; max-width: 100%; overflow-x: auto; border-radius: 6px; }
.mt-board svg { max-width: 100%; height: auto; display: block; }
#mt-evidence {
  flex: 0 0 320px;
  max-width: 100%;
  background: var(--mt-panel);
  border: 1px solid var(--mt-border);
  border-radius: 6px;
  padding: 12px 14px;
  font-size: 13px;
  line-height: 1.5;
  min-height: 120px;
  overflow-wrap: anywhere;
}
#mt-evidence p { margin: 0 0 8px; }
#mt-evidence .mt-evidence-title { font-weight: 600; color: var(--mt-title); }
#mt-evidence .mt-evidence-placeholder { color: var(--mt-muted); font-style: italic; }
.mt-sources { margin: 0 0 8px; padding-left: 18px; }
.mt-sources a { color: var(--mt-link); word-break: break-all; }
#mt-live-clock { margin: 12px 0 0; font-size: 12px; color: var(--mt-warn); }
.mt-footer {
  margin-top: 20px;
  font-size: 11px;
  color: var(--mt-muted);
  border-top: 1px solid var(--mt-border);
  padding-top: 10px;
}
[data-seg] { cursor: pointer; }
"#;

/// The page's one script: a JSON payload lookup for hover, a zone-label swap driven
/// entirely by precomputed strings, and — only when eligible — a live clock line that
/// moves the already-drawn `now` marker with plain arithmetic on two epoch instants. See
/// the module docs for what each of those deliberately does not do.
const SCRIPT: &str = r##"(function () {
  "use strict";
  var payloadEl = document.getElementById("mt-payload");
  if (!payloadEl) { return; }
  var payload;
  try {
    payload = JSON.parse(payloadEl.textContent || "{}");
  } catch (error) {
    return;
  }

  var svgRoot = document.querySelector(".mt-board svg");
  var evidenceEl = document.getElementById("mt-evidence");
  var zoneSelect = document.getElementById("mt-zone-select");
  var state = { zone: payload.defaultZone, activeId: null };

  function escapeText(value) {
    var holder = document.createElement("div");
    holder.textContent = value === undefined || value === null ? "" : String(value);
    return holder.innerHTML;
  }

  function renderSources(sources) {
    if (!sources || sources.length === 0) { return ""; }
    var out = '<ul class="mt-sources">';
    for (var i = 0; i < sources.length; i += 1) {
      var source = sources[i];
      out += "<li><a href=\"" + escapeText(source.url) + "\" rel=\"noopener noreferrer\">"
        + escapeText(source.url) + "</a> — fetched " + escapeText(source.fetchedAt)
        + ", effective from " + escapeText(source.effectiveFrom);
      if (source.publisherLastChanged) {
        out += ", publisher last changed " + escapeText(source.publisherLastChanged);
      }
      out += "</li>";
    }
    out += "</ul>";
    return out;
  }

  function renderDetail(detail) {
    var stretch = detail.stretch ? detail.stretch[state.zone] : null;
    var html = '<p class="mt-evidence-title">' + escapeText(detail.label) + "</p>";
    if (detail.kind === "venue") {
      html += "<p>" + (detail.phase
        ? "phase: " + escapeText(String(detail.phase).replace(/_/g, " "))
        : "not known") + "</p>";
      if (detail.notKnownBecause) {
        html += "<p>" + escapeText(detail.notKnownBecause)
          + " — an unknown is not a closed market.</p>";
      }
    } else {
      html += "<p>state: " + escapeText(detail.state)
        + " (derived — not a venue's published schedule)</p>";
    }
    if (stretch) {
      html += "<p>stretch: " + escapeText(stretch.start) + " – " + escapeText(stretch.end)
        + " (" + escapeText(state.zone) + ")</p>";
    }
    if (detail.kind === "venue") {
      html += "<p>start uncertainty: " + escapeText(detail.startUncertainty || "not known") + "</p>";
      html += "<p>end uncertainty: " + escapeText(detail.endUncertainty || "not known") + "</p>";
    } else {
      html += "<p>uncertainty: " + escapeText(detail.uncertainty) + "</p>";
    }
    if (detail.derivedReasoning) {
      html += "<p>derived: " + escapeText(detail.derivedReasoning) + "</p>";
    }
    html += renderSources(detail.sources);
    if (detail.datasetRevisions && detail.datasetRevisions.length) {
      html += "<p>revisions: " + escapeText(detail.datasetRevisions.join(", ")) + "</p>";
    }
    return html;
  }

  function showDetail(id) {
    var detail = payload.segments ? payload.segments[id] : null;
    if (!detail || !evidenceEl) { return; }
    state.activeId = id;
    evidenceEl.innerHTML = renderDetail(detail);
  }

  function clearDetail() {
    state.activeId = null;
    if (evidenceEl) {
      evidenceEl.innerHTML = '<p class="mt-evidence-placeholder">Hover or focus a segment '
        + "on the board for its evidence.</p>";
    }
  }

  function segOf(node) {
    return node && node.closest ? node.closest("[data-seg]") : null;
  }

  if (svgRoot) {
    svgRoot.addEventListener("mouseover", function (event) {
      var target = segOf(event.target);
      if (target) { showDetail(target.getAttribute("data-seg")); }
    });
    svgRoot.addEventListener("mouseout", function (event) {
      var leaving = segOf(event.target);
      var entering = segOf(event.relatedTarget);
      if (leaving && !entering) { clearDetail(); }
    });
    svgRoot.addEventListener("focusin", function (event) {
      var target = segOf(event.target);
      if (target) { showDetail(target.getAttribute("data-seg")); }
    });
    svgRoot.addEventListener("focusout", function () { clearDetail(); });
  }

  function applyZone(zone) {
    state.zone = zone;
    var axisLabels = payload.axis ? payload.axis[zone] : null;
    if (axisLabels && svgRoot) {
      var ticks = svgRoot.querySelectorAll("[data-tick]");
      for (var i = 0; i < ticks.length; i += 1) {
        var index = Number(ticks[i].getAttribute("data-tick"));
        if (axisLabels[index] !== undefined) { ticks[i].textContent = axisLabels[index]; }
      }
    }
    var zoneLabelEl = svgRoot ? svgRoot.querySelector("#mt-axis-zone-label") : null;
    if (zoneLabelEl) { zoneLabelEl.textContent = "axis in " + zone; }
    var nowInfoEl = svgRoot ? svgRoot.querySelector("#mt-now-info") : null;
    if (nowInfoEl && payload.now && payload.now.present && payload.now.info
        && payload.now.info[zone] !== undefined) {
      nowInfoEl.textContent = payload.now.info[zone];
    }
    if (state.activeId) { showDetail(state.activeId); }
  }

  if (zoneSelect) {
    zoneSelect.addEventListener("change", function () { applyZone(zoneSelect.value); });
  }

  // The live clock: only when `now` came from an unmeasured or host-bound clock read —
  // never when it was supplied via `--at`. A supplied instant is a fact the page states;
  // the browser's own clock has no business moving it.
  var nowLine = document.getElementById("mt-now-line");
  var track = document.getElementById("mt-track");
  var liveClockEl = document.getElementById("mt-live-clock");
  if (payload.now && payload.now.present && !payload.now.supplied
      && nowLine && track && payload.interval) {
    var trackLeft = Number(track.getAttribute("x"));
    var trackSpan = Number(track.getAttribute("width"));
    var startMs = Number(payload.interval.startMs);
    var endMs = Number(payload.interval.endMs);
    var tick = function () {
      var now = Date.now();
      var fraction = (now - startMs) / (endMs - startMs);
      if (fraction < 0) { fraction = 0; }
      if (fraction > 1) { fraction = 1; }
      var x = trackLeft + trackSpan * fraction;
      nowLine.setAttribute("x1", String(x));
      nowLine.setAttribute("x2", String(x));
      if (liveClockEl) {
        liveClockEl.textContent = "browser clock — discipline unmeasured — "
          + new Date(now).toISOString();
      }
    };
    tick();
    setInterval(tick, 1000);
  }

  applyZone(state.zone);
})();"##;
