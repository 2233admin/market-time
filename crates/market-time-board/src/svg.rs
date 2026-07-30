//! The board as SVG.
//!
//! Same board, different surface. The text renderer proves the layout in a terminal; this
//! one produces the thing people recognise as a global trading-hours board — one row per
//! venue, segments laid across a shared axis, a marker on the instant being viewed.
//!
//! # What this renderer will not do
//!
//! Colour is never the only channel. A conventional board leans on hue alone, which is
//! fine when the reader is a person with unimpaired colour vision and useless when it is
//! anyone else, or a machine. So:
//!
//! - every row carries its status in words at the right-hand edge;
//! - an out-of-coverage stretch is hatched **and** named, not merely a paler shade of
//!   closed, because an unknown and a closed market are different claims;
//! - a boundary that is the scheduled start of a process gets a soft edge rather than a
//!   hairline, because the venue did not publish an instant;
//! - the documents behind the rows are printed under the board, so the picture is not the
//!   end of the trail.
//!
//! No dependency, no font loading, no script: the output is a self-contained SVG string
//! that opens anywhere and diffs cleanly.

use crate::{
    BoardRow, BoardView, NowMarker, band_label, band_state_word, grouped, overlap_label,
    overlap_state_word, trading_count,
};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use market_time_core::{
    BandState, Interval, OverlapState, Phase, Timeline, TimelineSegment, Uncertainty, UtcInstant,
};
use std::fmt::Write as _;

/// Colours, named once. Keeping them out of the markup strings also keeps those strings
/// free of the `"#` sequence, which is why they need only one raw-string hash.
mod palette {
    pub const BACKGROUND: &str = "#0d1117";
    pub const TRACK: &str = "#161b22";
    pub const GRID: &str = "#21262d";
    pub const TITLE: &str = "#e6edf3";
    pub const LABEL: &str = "#c9d1d9";
    pub const MUTED: &str = "#8b949e";
    pub const FAINT: &str = "#6e7681";
    pub const ATTENTION: &str = "#d29922";
    pub const NOW: &str = "#f78166";
    pub const TRADING: &str = "#2f81f7";
    pub const AUCTION: &str = "#a371f7";
    pub const PRE_OPEN: &str = "#39c5cf";
    pub const POST_CLOSE: &str = "#1158c7";
    pub const BREAK: &str = "#6e7681";
    pub const CLOSED: &str = "#30363d";
    pub const HALTED: &str = "#f85149";
    pub const UNHANDLED: &str = "#db6d28";
    // The derived band/overlap section's own colours — deliberately not the phase palette
    // above, so a band row is distinguishable from a venue row by colour too, not only by
    // its shorter track and its "(derived)" label.
    pub const BAND_TRADING: &str = "#3fb950";
    pub const BAND_NOT_TRADING: &str = "#21262d";
    pub const OVERLAP_TRADING: &str = "#d2a8ff";
}

/// Layout constants for the derived band section, shared between the height calculation
/// and the drawing pass so the two cannot drift apart — a mismatch there would either
/// clip a row or leave dead canvas beneath it.
const BAND_ROW_SCALE: f64 = 0.72;
const BAND_SECTION_TOP_GAP: f64 = 24.0;
const BAND_HEADING_HEIGHT: f64 = 20.0;
const BAND_OVERLAP_GAP: f64 = 6.0;
const BAND_SECTION_NOTE_GAP: f64 = 24.0;

/// Layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgOptions {
    /// Overall width in pixels.
    pub width: u32,
    /// Height of one venue row.
    pub row_height: u32,
    /// Width of the left-hand venue label column.
    pub label_width: u32,
    /// Width of the right-hand status column.
    pub status_width: u32,
    /// Whether to print the sources block beneath the board.
    pub show_sources: bool,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            width: 1180,
            row_height: 34,
            label_width: 200,
            status_width: 190,
            show_sources: true,
        }
    }
}

/// Renders the board as a self-contained SVG document.
#[must_use]
pub fn render_svg(view: &BoardView) -> String {
    render_svg_with(view, &SvgOptions::default())
}

/// Renders the board as SVG with explicit layout.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn render_svg_with(view: &BoardView, options: &SvgOptions) -> String {
    let zone = TimeZone::get(&view.axis_zone).unwrap_or(TimeZone::UTC);

    let track_left = f64::from(options.label_width);
    let track_span = f64::from(
        options
            .width
            .saturating_sub(options.label_width + options.status_width),
    )
    .max(120.0);
    let header = 96.0;
    let row_height = f64::from(options.row_height);
    let groups = grouped(&view.rows);
    let heading_height = 22.0;
    let all_rows =
        row_height * count_as_f64(view.rows.len()) + heading_height * count_as_f64(groups.len());
    let sources = if options.show_sources {
        source_lines(&view.rows)
    } else {
        Vec::new()
    };
    // Zero when there is nothing to draw, so a zero-band view's canvas is exactly the
    // height it always was — not a pixel added for a section that never appears.
    let band_extra = band_section_height(view, options);
    let height = header + all_rows + 78.0 + 15.0 * count_as_f64(sources.len()) + band_extra;

    let mut svg = String::new();
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height:.0}" width="{width}" height="{height:.0}" font-family="ui-sans-serif, -apple-system, Segoe UI, system-ui, sans-serif">"#,
        width = options.width
    );
    svg.push_str(&defs());
    let _ = write!(
        svg,
        r#"<rect width="100%" height="100%" fill="{bg}"/>"#,
        bg = palette::BACKGROUND
    );

    let _ = write!(
        svg,
        r#"<text x="24" y="34" fill="{fill}" font-size="17" font-weight="600">Mark Time — venue sessions</text>"#,
        fill = palette::TITLE
    );
    if let Some(now) = &view.now {
        let _ = write!(
            svg,
            r#"<text x="24" y="54" fill="{fill}" font-size="12">{when} · {clock}</text>"#,
            fill = palette::MUTED,
            when = escape(&format_instant(&zone, now.instant)),
            clock = escape(&now.discipline.describe())
        );
    }
    if let Some(now) = &view.now {
        let _ = write!(
            svg,
            r#"<text x="{x:.0}" y="34" fill="{fill}" font-size="12" text-anchor="end">{count}</text>"#,
            x = f64::from(options.width) - 24.0,
            fill = palette::LABEL,
            count = escape(&trading_count(&view.rows, now.instant).describe())
        );
    }
    let _ = write!(
        svg,
        r#"<text x="{x:.0}" y="54" fill="{fill}" font-size="12" text-anchor="end">axis in {zone_label}</text>"#,
        x = f64::from(options.width) - 24.0,
        fill = palette::MUTED,
        zone_label = escape(&view.axis_zone)
    );

    // Axis.
    let ticks = 8_usize;
    for tick in 0..=ticks {
        let fraction = count_as_f64(tick) / count_as_f64(ticks);
        let x = track_left + track_span * fraction;
        let _ = write!(
            svg,
            r#"<line x1="{x:.1}" y1="{top:.1}" x2="{x:.1}" y2="{bottom:.1}" stroke="{stroke}" stroke-width="1"/>"#,
            top = header - 12.0,
            bottom = header + all_rows,
            stroke = palette::GRID
        );
        let _ = write!(
            svg,
            r#"<text x="{x:.1}" y="{y:.0}" fill="{fill}" font-size="11" text-anchor="{anchor}">{label}</text>"#,
            y = header - 16.0,
            fill = palette::FAINT,
            // The first and last labels would otherwise hang over the label and status
            // columns, which is where the venue name and the status word live.
            anchor = match tick {
                0 => "start",
                t if t == ticks => "end",
                _ => "middle",
            },
            label = escape(&format_hour(&zone, at_fraction(view.interval, fraction)))
        );
    }

    // Rows, grouped the way conventional boards group them.
    let mut cursor_y = header;
    for (family, members) in &groups {
        let _ = write!(
            svg,
            r#"<text x="24" y="{y:.1}" fill="{fill}" font-size="11" font-weight="600" letter-spacing="0.6">{heading}</text>"#,
            y = cursor_y + 14.0,
            fill = palette::FAINT,
            heading = escape(
                &family
                    .map_or("OTHER", |family| family.heading())
                    .to_uppercase()
            )
        );
        cursor_y += heading_height;

        for row in members {
            let bar_y = cursor_y + 5.0;
            let bar_height = row_height - 12.0;
            let text_y = bar_y + bar_height * 0.72;

            // Name on one line, place on the next. A venue name long enough to collide
            // with its own city is the common case, not the exception.
            let name_y = if row.location.is_some() {
                text_y - 6.0
            } else {
                text_y
            };
            let _ = write!(
                svg,
                r#"<text x="24" y="{name_y:.1}" fill="{fill}" font-size="13">{venue}</text>"#,
                fill = palette::LABEL,
                venue = escape(&row.label)
            );
            if let Some(location) = &row.location {
                let _ = write!(
                    svg,
                    r#"<text x="24" y="{y:.1}" fill="{fill}" font-size="10">{location}</text>"#,
                    y = name_y + 13.0,
                    fill = palette::FAINT,
                    location = escape(location)
                );
            }
            let _ = write!(
                svg,
                r#"<rect x="{track_left:.1}" y="{bar_y:.1}" width="{track_span:.1}" height="{bar_height:.1}" fill="{fill}" rx="3"/>"#,
                fill = palette::TRACK
            );

            for segment in &row.timeline.segments {
                let Some((x, width)) =
                    placement(view.interval, segment.interval(), track_left, track_span)
                else {
                    continue;
                };

                let (fill, hover) = match segment {
                    TimelineSegment::Phase { answer, .. } => (
                        phase_fill(answer.phase).to_owned(),
                        format!(
                            "{phase} · starts {start} · ends {end}",
                            phase = answer.phase,
                            start = answer.boundary_start.uncertainty,
                            end = answer.boundary_end.uncertainty
                        ),
                    ),
                    TimelineSegment::Unknown { gap, .. } => {
                        ("url(#not-known)".to_owned(), gap.describe())
                    }
                };

                let _ = write!(
                    svg,
                    r#"<rect x="{x:.2}" y="{bar_y:.1}" width="{width:.2}" height="{bar_height:.1}" fill="{fill}" rx="2"><title>{hover}</title></rect>"#,
                    hover = escape(&hover)
                );

                // A boundary the venue publishes as the start of a process is not a hairline.
                if let TimelineSegment::Phase { answer, .. } = segment
                    && matches!(
                        answer.boundary_start.uncertainty,
                        Uncertainty::ProcessStart { .. }
                    )
                {
                    let _ = write!(
                        svg,
                        r#"<rect x="{x:.2}" y="{bar_y:.1}" width="{fade:.2}" height="{bar_height:.1}" fill="url(#process-start)"/>"#,
                        fade = width.min(16.0)
                    );
                }
            }

            let status = status_text(&row.timeline, view.now.as_ref());
            if !status.is_empty() {
                let _ = write!(
                    svg,
                    r#"<text x="{x:.0}" y="{text_y:.1}" fill="{fill}" font-size="12">{status}</text>"#,
                    x = track_left + track_span + 14.0,
                    fill = if status == "not known" {
                        palette::ATTENTION
                    } else {
                        palette::MUTED
                    },
                    status = escape(&status)
                );
            }
            cursor_y += row_height;
        }
    }

    // The instant being viewed.
    if let Some(now) = &view.now
        && view.interval.contains(now.instant)
    {
        let x = track_left + track_span * fraction_of(view.interval, now.instant);
        let _ = write!(
            svg,
            r#"<line x1="{x:.2}" y1="{top:.1}" x2="{x:.2}" y2="{bottom:.1}" stroke="{stroke}" stroke-width="1.5"/>"#,
            top = header - 12.0,
            bottom = header + all_rows + 4.0,
            stroke = palette::NOW
        );
    }

    // Legend.
    let legend_y = header + all_rows + 32.0;
    let mut legend_x = 24.0;
    for (fill, label) in legend_entries() {
        let _ = write!(
            svg,
            r#"<rect x="{legend_x:.0}" y="{y:.0}" width="11" height="11" rx="2" fill="{fill}"/>"#,
            y = legend_y - 9.0
        );
        let _ = write!(
            svg,
            r#"<text x="{x:.0}" y="{legend_y:.0}" fill="{text_fill}" font-size="11">{label}</text>"#,
            x = legend_x + 16.0,
            text_fill = palette::MUTED
        );
        legend_x += 22.0 + 6.9 * count_as_f64(label.len());
    }
    let _ = write!(
        svg,
        r#"<rect x="{legend_x:.0}" y="{y:.0}" width="11" height="11" rx="2" fill="url(#not-known)"/>"#,
        y = legend_y - 9.0
    );
    let _ = write!(
        svg,
        r#"<text x="{x:.0}" y="{legend_y:.0}" fill="{fill}" font-size="11">not known</text>"#,
        x = legend_x + 16.0,
        fill = palette::ATTENTION
    );

    let mut note_y = legend_y + 24.0;
    let _ = write!(
        svg,
        r#"<text x="24" y="{note_y:.0}" fill="{fill}" font-size="11">a hatched stretch is outside declared coverage — an unknown is not a closed market</text>"#,
        fill = palette::FAINT
    );

    for line in sources {
        note_y += 15.0;
        let _ = write!(
            svg,
            r#"<text x="24" y="{note_y:.0}" fill="{fill}" font-size="11">{line}</text>"#,
            fill = palette::FAINT,
            line = escape(&line)
        );
    }

    if !view.bands.is_empty() {
        let band_top = height - band_extra;
        render_band_section(&mut svg, view, options, band_top, track_left, track_span);
    }

    svg.push_str("</svg>");
    svg
}

/// The extra canvas height the derived band section needs, or exactly `0.0` with no bands
/// to draw.
///
/// Kept in lock-step with [`render_band_section`]'s own cursor arithmetic through the
/// shared `BAND_*` constants, so the canvas this function sizes and the section that
/// function draws can never disagree about how tall it is.
fn band_section_height(view: &BoardView, options: &SvgOptions) -> f64 {
    if view.bands.bands.is_empty() {
        return 0.0;
    }
    let band_row_height = f64::from(options.row_height) * BAND_ROW_SCALE;
    let mut height = BAND_SECTION_TOP_GAP + BAND_HEADING_HEIGHT;
    height += band_row_height * count_as_f64(view.bands.bands.len());
    if !view.bands.overlaps.is_empty() {
        height += BAND_OVERLAP_GAP + BAND_HEADING_HEIGHT;
        height += band_row_height * count_as_f64(view.bands.overlaps.len());
    }
    height + BAND_SECTION_NOTE_GAP
}

/// Renders the derived band and overlap rows beneath the venue section, as a fully
/// separate pass appended after everything the board already draws.
///
/// A zero-band view never calls this function at all (see the call site above), so it
/// cannot add a byte of markup or a pixel of height to a board that has nothing derived to
/// show. Where it is called, three independent signals mark every row as derived rather
/// than a venue's published schedule, in case a viewer misses one: the section heading
/// says DERIVED, the row's own label carries a "(derived)" tag, and the track is drawn
/// shorter than a venue row's. An unknown stretch still uses `url(#not-known)` — the same
/// hatch pattern the venue rows use — because that visual promise ("an unknown is not a
/// closed market") is the product's, not the venue section's alone, and a band's unknown
/// stretch deserves exactly the same honesty.
fn render_band_section(
    svg: &mut String,
    view: &BoardView,
    options: &SvgOptions,
    top: f64,
    track_left: f64,
    track_span: f64,
) {
    let band_row_height = f64::from(options.row_height) * BAND_ROW_SCALE;
    let mut cursor_y = top + BAND_SECTION_TOP_GAP;

    let _ = write!(
        svg,
        r#"<line x1="24" y1="{top:.1}" x2="{x2:.1}" y2="{top:.1}" stroke="{stroke}" stroke-width="1" stroke-dasharray="3,3"/>"#,
        x2 = f64::from(options.width) - 24.0,
        stroke = palette::GRID
    );
    let _ = write!(
        svg,
        r#"<text x="24" y="{y:.1}" fill="{fill}" font-size="11" font-weight="600" letter-spacing="0.6">DERIVED SESSION BANDS — never a venue's published schedule</text>"#,
        y = cursor_y + 4.0,
        fill = palette::ATTENTION
    );
    cursor_y += BAND_HEADING_HEIGHT;

    for band in &view.bands.bands {
        let geometry = BarGeometry {
            track_left,
            track_span,
            bar_y: cursor_y + 4.0,
            bar_height: (band_row_height - 8.0).max(4.0),
        };
        let segments = band.segments().iter().map(|segment| {
            let fill = match segment.state {
                BandState::Unknown => "url(#not-known)".to_owned(),
                BandState::Trading => palette::BAND_TRADING.to_owned(),
                BandState::NotTrading => palette::BAND_NOT_TRADING.to_owned(),
                _ => palette::UNHANDLED.to_owned(),
            };
            let hover =
                band_hover_text(band_state_word(segment.state), segment.uncertainty.as_ref());
            (segment.interval, fill, hover)
        });
        render_derived_row(svg, &band_label(band), segments, view.interval, &geometry);
        cursor_y += band_row_height;
    }

    if !view.bands.overlaps.is_empty() {
        cursor_y += BAND_OVERLAP_GAP;
        let _ = write!(
            svg,
            r#"<text x="24" y="{y:.1}" fill="{fill}" font-size="11" font-weight="600" letter-spacing="0.6">DERIVED OVERLAPS</text>"#,
            y = cursor_y + 4.0,
            fill = palette::ATTENTION
        );
        cursor_y += BAND_HEADING_HEIGHT;

        for overlap in &view.bands.overlaps {
            let geometry = BarGeometry {
                track_left,
                track_span,
                bar_y: cursor_y + 4.0,
                bar_height: (band_row_height - 8.0).max(4.0),
            };
            let segments = overlap.segments().iter().map(|segment| {
                let fill = match segment.state {
                    OverlapState::Unknown => "url(#not-known)".to_owned(),
                    OverlapState::Overlapping => palette::OVERLAP_TRADING.to_owned(),
                    OverlapState::NotOverlapping => palette::BAND_NOT_TRADING.to_owned(),
                    _ => palette::UNHANDLED.to_owned(),
                };
                let hover = band_hover_text(
                    overlap_state_word(segment.state),
                    segment.uncertainty.as_ref(),
                );
                (segment.interval, fill, hover)
            });
            render_derived_row(
                svg,
                &overlap_label(overlap),
                segments,
                view.interval,
                &geometry,
            );
            cursor_y += band_row_height;
        }
    }

    let _ = write!(
        svg,
        r#"<text x="24" y="{y:.1}" fill="{fill}" font-size="11">{note}</text>"#,
        y = cursor_y + 16.0,
        fill = palette::FAINT,
        note = escape(&format!(
            "bands and overlaps are computed groupings, never a venue's published hours — {}",
            crate::band_legend()
        ))
    );
}

/// Where and how tall one derived row's track is drawn.
///
/// Bundled so [`render_derived_row`] takes one geometry argument instead of four loose
/// numbers — the same track position and bar height a band row and an overlap row share,
/// just computed fresh per row as `cursor_y` advances.
struct BarGeometry {
    track_left: f64,
    track_span: f64,
    bar_y: f64,
    bar_height: f64,
}

/// Draws one band or overlap row: its label with the "(derived)" tag, a track drawn with a
/// dashed border (never a venue row's plain one), and each segment placed exactly as a
/// venue row's segments are — through the same [`placement`] — and filled with whatever
/// colour and hover text the caller already resolved for its state.
///
/// Shared by both loops in [`render_band_section`] because the layout is identical between
/// a band row and an overlap row; only the state-to-colour mapping differs, and that stays
/// with each caller rather than becoming a parameter this function would have to branch on.
fn render_derived_row(
    svg: &mut String,
    label: &str,
    segments: impl Iterator<Item = (Interval, String, String)>,
    interval: Interval,
    geometry: &BarGeometry,
) {
    let &BarGeometry {
        track_left,
        track_span,
        bar_y,
        bar_height,
    } = geometry;

    let _ = write!(
        svg,
        r#"<text x="24" y="{text_y:.1}" fill="{fill}" font-size="12">{label} <tspan fill="{muted}" font-size="10">(derived)</tspan></text>"#,
        text_y = bar_y + bar_height * 0.85,
        fill = palette::LABEL,
        label = escape(label),
        muted = palette::FAINT,
    );
    let _ = write!(
        svg,
        r#"<rect x="{track_left:.1}" y="{bar_y:.1}" width="{track_span:.1}" height="{bar_height:.1}" fill="{fill}" stroke="{stroke}" stroke-width="1" stroke-dasharray="2,2" rx="2"/>"#,
        fill = palette::TRACK,
        stroke = palette::FAINT
    );

    for (segment_interval, fill, hover) in segments {
        let Some((x, width)) = placement(interval, segment_interval, track_left, track_span) else {
            continue;
        };
        let _ = write!(
            svg,
            r#"<rect x="{x:.2}" y="{bar_y:.1}" width="{width:.2}" height="{bar_height:.1}" fill="{fill}" rx="2"><title>{hover}</title></rect>"#,
            hover = escape(&hover)
        );
    }
}

/// The hover title for one band or overlap segment: its state in words, then how
/// precisely it is known — or, when nothing is known at all (`BandSegment::uncertainty`'s
/// `None` case), a sentence saying so rather than a blank that could be misread as exact.
fn band_hover_text(state_word: &str, uncertainty: Option<&Uncertainty>) -> String {
    match uncertainty {
        Some(uncertainty) => format!("{state_word} · {uncertainty}"),
        None => format!("{state_word} · no schedule known for this stretch"),
    }
}

fn defs() -> String {
    format!(
        r#"<defs><pattern id="not-known" width="7" height="7" patternTransform="rotate(45)" patternUnits="userSpaceOnUse"><rect width="7" height="7" fill="{grid}"/><line x1="0" y1="0" x2="0" y2="7" stroke="{stripe}" stroke-width="2.2"/></pattern><linearGradient id="process-start" x1="0" x2="1" y1="0" y2="0"><stop offset="0%" stop-color="{bg}" stop-opacity="0.85"/><stop offset="100%" stop-color="{bg}" stop-opacity="0"/></linearGradient></defs>"#,
        grid = palette::GRID,
        stripe = palette::ATTENTION,
        bg = palette::BACKGROUND
    )
}

fn legend_entries() -> [(&'static str, &'static str); 6] {
    [
        (palette::TRADING, "trading"),
        (palette::AUCTION, "auction"),
        (palette::PRE_OPEN, "pre-open"),
        (palette::BREAK, "break"),
        (palette::POST_CLOSE, "post-close"),
        (palette::CLOSED, "closed"),
    ]
}

fn phase_fill(phase: Phase) -> &'static str {
    match phase {
        Phase::ContinuousTrading => palette::TRADING,
        Phase::OpeningAuction | Phase::ClosingAuction => palette::AUCTION,
        Phase::PreOpen => palette::PRE_OPEN,
        Phase::MidDayBreak => palette::BREAK,
        Phase::PostClose => palette::POST_CLOSE,
        Phase::Closed => palette::CLOSED,
        Phase::NonTradingInterruption => palette::HALTED,
        // A phase added to the core after this renderer was built. Drawn in a colour that
        // belongs to nothing else, so it reads as unhandled rather than quietly joining a
        // neighbour.
        _ => palette::UNHANDLED,
    }
}

fn placement(
    interval: Interval,
    segment: Interval,
    track_left: f64,
    track_span: f64,
) -> Option<(f64, f64)> {
    let visible = interval.intersection(segment)?;
    let start = fraction_of(interval, visible.start);
    let end = fraction_of(interval, visible.end);
    Some((
        track_left + track_span * start,
        // A phase that lasts five minutes of a day is under a pixel wide. Give it two,
        // because an auction nobody can see is an auction the board did not report.
        (track_span * (end - start)).max(2.0),
    ))
}

fn fraction_of(interval: Interval, at: UtcInstant) -> f64 {
    let span = interval.start.saturating_nanos_until(interval.end).max(1);
    let offset = interval.start.saturating_nanos_until(at).clamp(0, span);
    nanos_as_f64(offset) / nanos_as_f64(span)
}

fn at_fraction(interval: Interval, fraction: f64) -> UtcInstant {
    let span = interval.start.saturating_nanos_until(interval.end).max(1);
    #[allow(clippy::cast_possible_truncation)]
    let offset = (fraction * nanos_as_f64(span)) as i128;
    interval.start.saturating_add_nanos(offset)
}

/// Nanosecond counts become pixel positions here, and pixels are `f64`.
///
/// The precision loss is real and irrelevant: it starts past 2^53 nanoseconds, about 104
/// days of span, and a board covering 104 days is quantised by the pixel long before it is
/// quantised by the float.
#[allow(clippy::cast_precision_loss)]
fn nanos_as_f64(value: i128) -> f64 {
    value as f64
}

#[allow(clippy::cast_precision_loss)]
fn count_as_f64(value: usize) -> f64 {
    value as f64
}

fn status_text(timeline: &Timeline, now: Option<&NowMarker>) -> String {
    let Some(now) = now else {
        return String::new();
    };
    match timeline
        .segments
        .iter()
        .find(|segment| segment.interval().contains(now.instant))
    {
        Some(TimelineSegment::Phase { answer, .. }) => answer.phase.as_str().replace('_', " "),
        Some(TimelineSegment::Unknown { .. }) => "not known".to_owned(),
        None => String::new(),
    }
}

fn source_lines(rows: &[BoardRow]) -> Vec<String> {
    let mut lines = Vec::new();
    for row in rows {
        let mut seen: Vec<&str> = Vec::new();
        for segment in &row.timeline.segments {
            let TimelineSegment::Phase { answer, .. } = segment else {
                continue;
            };
            for evidence in &answer.evidence {
                if !seen.contains(&evidence.source_url()) {
                    seen.push(evidence.source_url());
                }
            }
        }
        if !seen.is_empty() {
            lines.push(format!("{}: {}", row.label, seen.join("   ")));
        }
    }
    lines
}

fn zoned(zone: &TimeZone, at: UtcInstant) -> jiff::Zoned {
    Timestamp::from_nanosecond(at.as_nanos_since_unix_epoch())
        .unwrap_or(Timestamp::UNIX_EPOCH)
        .to_zoned(zone.clone())
}

fn format_hour(zone: &TimeZone, at: UtcInstant) -> String {
    let zoned = zoned(zone, at);
    format!("{:02}:{:02}", zoned.hour(), zoned.minute())
}

fn format_instant(zone: &TimeZone, at: UtcInstant) -> String {
    let zoned = zoned(zone, at);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        zoned.year(),
        zoned.month(),
        zoned.day(),
        zoned.hour(),
        zoned.minute(),
        zoned.second()
    )
}

/// XML-escapes text that comes from data.
///
/// Venue names, source URLs, and derivation notes are all operator-supplied, and none of
/// them may be able to close a tag.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
    out
}
