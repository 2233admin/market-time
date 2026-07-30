//! Mark Time — the global trading-hours board.
//!
//! A shell. It decides *how* to render; the core decides *what* is true.
//!
//! # Rules this surface is bound by
//!
//! - One `now` per render, shared across every venue row, so the snapshot is coherent.
//! - Displaying "now" obliges surfacing the host's clock discipline bounds as uncertainty.
//!   When discipline data is unavailable the board MUST NOT fall back to presenting the
//!   clock as exact.
//! - An unknown stretch renders visually distinct from a closed one. They are different
//!   claims, and the primary consumer is an agent that cannot infer the difference from
//!   styling.
//! - If a display cannot express an honest answer, **the display changes, not the answer**.
//!
//! # Shape
//!
//! One row per venue, that venue's phases laid out across the queried interval on a shared
//! axis, a marker on the instant being viewed. Segment position and width come from the
//! timeline the core returned; this crate holds no schedule of its own.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

use jiff::Timestamp;
use jiff::tz::TimeZone;
use market_time_core::Phase;
use market_time_core::{EvidenceRef, Interval, UtcInstant, VenueId};
use market_time_core::{Timeline, TimelineSegment};
use std::fmt::Write as _;

/// How well the host's own clock is disciplined.
///
/// `Unmeasured` exists because it is the honest answer on most hosts: wide-area time
/// synchronisation does not reach nanoseconds, and a board that cannot measure its own
/// discipline says so rather than presenting its clock as exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockDiscipline {
    /// The host reports a bound, e.g. from an NTP or PTP daemon.
    BoundedNanos {
        /// The reported bound.
        nanos: i128,
        /// What reported it.
        source: String,
    },
    /// No discipline data is available.
    Unmeasured {
        /// Where the instant came from, so a reader knows what is being trusted.
        source: String,
    },
    /// The instant was supplied rather than read from a clock.
    ///
    /// Replaying a stated instant has no clock error to report, and saying "within 0ms"
    /// would imply a measurement nobody made.
    Given {
        /// Who supplied it.
        source: String,
    },
}

impl ClockDiscipline {
    /// A one-line rendering of the bound.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::BoundedNanos { nanos, source } => {
                #[allow(clippy::cast_precision_loss)]
                let millis = *nanos as f64 / 1_000_000.0;
                format!("host clock within {millis:.1}ms ({source})")
            }
            Self::Unmeasured { source } => {
                format!("host clock discipline unmeasured ({source})")
            }
            Self::Given { source } => format!("instant supplied, not read from a clock ({source})"),
        }
    }
}

/// The instant the board is drawing, and how well that instant is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowMarker {
    /// The instant, read by the shell and passed in.
    pub instant: UtcInstant,
    /// How well the host's clock is disciplined.
    pub discipline: ClockDiscipline,
}

/// What the board renders.
#[derive(Debug, Clone)]
pub struct BoardView {
    /// The interval the axis spans.
    pub interval: Interval,
    /// One timeline per venue, all over the same interval.
    pub rows: Vec<Timeline>,
    /// The instant being viewed, when the board is showing one.
    pub now: Option<NowMarker>,
    /// The IANA zone the axis is labelled in. Labelling only: the answers stay UTC.
    pub axis_zone: String,
    /// How many columns the timeline occupies.
    pub columns: usize,
}

/// Renders the board as text.
///
/// Every segment's position and width comes from the timeline the core returned. There is
/// no schedule here, no venue table, and no fallback: given an empty row, this draws an
/// empty row.
#[must_use]
pub fn render(view: &BoardView) -> String {
    let zone = TimeZone::get(&view.axis_zone).unwrap_or(TimeZone::UTC);
    let columns = view.columns.max(12);
    let label_width = view
        .rows
        .iter()
        .map(|row| row.venue.as_str().len())
        .max()
        .unwrap_or(6)
        .max(6);

    let mut out = String::new();

    let _ = writeln!(
        out,
        "{:label_width$}  {}",
        "",
        axis_labels(&zone, view.interval, columns),
        label_width = label_width
    );

    for row in &view.rows {
        let _ = writeln!(
            out,
            "{:label_width$}  {}  {}",
            row.venue.as_str(),
            render_row(row, view.interval, columns, view.now.as_ref()),
            status(row, view.now.as_ref()),
            label_width = label_width
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "  axis: {} ({columns} columns)", view.axis_zone);
    if let Some(now) = &view.now {
        let _ = writeln!(
            out,
            "  now:  {} — {}",
            format_instant(&zone, now.instant),
            now.discipline.describe()
        );
    }
    let _ = writeln!(out, "  key:  {}", legend());

    let sources = render_sources(&view.rows);
    if !sources.is_empty() {
        let _ = writeln!(out);
        out.push_str(&sources);
    }

    out
}

/// What one segment of a rendered row rests on.
///
/// This is the "inspect a segment" answer: a viewer pointing at a stretch of a row gets
/// the phase, the boundaries with their uncertainty, whether the governing rule was
/// published or derived, the documents behind it, and the revisions that produced it —
/// without leaving the product to look any of it up (SC-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentDetail {
    /// The venue whose row was inspected.
    pub venue: VenueId,
    /// The stretch the segment occupies.
    pub interval: Interval,
    /// The phase, or `None` when the stretch is outside declared coverage.
    pub phase: Option<Phase>,
    /// Why the stretch is not known, when it is not.
    pub not_known_because: Option<String>,
    /// How the start boundary is known.
    pub start_uncertainty: Option<String>,
    /// How the end boundary is known.
    pub end_uncertainty: Option<String>,
    /// Present when the governing rule is a reading rather than the venue's wording.
    pub derived_reasoning: Option<String>,
    /// The documents behind the segment.
    pub sources: Vec<SourceRef>,
    /// The revisions that produced it.
    pub dataset_revisions: Vec<String>,
}

/// One document behind a segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    /// The document a person can open.
    pub url: String,
    /// When it was retrieved.
    pub fetched_at: String,
    /// The date the rule takes effect.
    pub effective_from: String,
    /// When the publisher last changed it, where the publisher says.
    pub publisher_last_changed: Option<String>,
}

/// What the segment covering `at` rests on, or `None` when the row does not cover `at`.
///
/// A board is a rendering; this is the door out of it. A graphical surface calls this on
/// click, the CLI calls it for `evidence`, and both get the same answer because neither
/// assembles it itself.
#[must_use]
pub fn inspect(timeline: &Timeline, at: UtcInstant) -> Option<SegmentDetail> {
    let segment = segment_at(timeline, at)?;
    Some(match segment {
        TimelineSegment::Phase { interval, answer } => SegmentDetail {
            venue: timeline.venue.clone(),
            interval: *interval,
            phase: Some(answer.phase),
            not_known_because: None,
            start_uncertainty: Some(answer.boundary_start.uncertainty.to_string()),
            end_uncertainty: Some(answer.boundary_end.uncertainty.to_string()),
            derived_reasoning: answer.derived_reasoning.clone(),
            sources: answer.evidence.iter().map(source_ref).collect(),
            dataset_revisions: answer
                .dataset_revisions
                .iter()
                .map(ToString::to_string)
                .collect(),
        },
        TimelineSegment::Unknown { interval, gap } => SegmentDetail {
            venue: timeline.venue.clone(),
            interval: *interval,
            phase: None,
            not_known_because: Some(gap.describe()),
            start_uncertainty: None,
            end_uncertainty: None,
            derived_reasoning: None,
            sources: Vec::new(),
            dataset_revisions: gap
                .dataset_revisions
                .iter()
                .map(ToString::to_string)
                .collect(),
        },
    })
}

fn source_ref(evidence: &EvidenceRef) -> SourceRef {
    SourceRef {
        url: evidence.source_url().to_owned(),
        fetched_at: evidence.fetched_at().to_string(),
        effective_from: evidence.effective_from().to_owned(),
        publisher_last_changed: evidence.publisher_last_changed().map(ToOwned::to_owned),
    }
}

/// The sources block printed under the board.
///
/// A board that shows a phase and hides where it came from is the thing this product
/// exists not to be. Every venue drawn above lists the documents its drawn segments rest
/// on, deduplicated, with derived rules marked as derived.
fn render_sources(rows: &[Timeline]) -> String {
    let mut out = String::new();

    for row in rows {
        let mut seen: Vec<String> = Vec::new();
        let mut lines: Vec<String> = Vec::new();
        let mut derived: Vec<String> = Vec::new();

        for segment in &row.segments {
            let TimelineSegment::Phase { answer, .. } = segment else {
                continue;
            };
            for evidence in &answer.evidence {
                if seen.iter().any(|url| url == evidence.source_url()) {
                    continue;
                }
                seen.push(evidence.source_url().to_owned());
                lines.push(format!(
                    "    {} (fetched {}, effective from {})",
                    evidence.source_url(),
                    evidence.fetched_at(),
                    evidence.effective_from()
                ));
            }
            if let Some(reasoning) = &answer.derived_reasoning
                && !derived.iter().any(|note| note == reasoning)
            {
                derived.push(reasoning.clone());
            }
        }

        if lines.is_empty() && derived.is_empty() {
            continue;
        }

        let _ = writeln!(out, "  sources for {}:", row.venue);
        for line in lines {
            let _ = writeln!(out, "{line}");
        }
        for note in derived {
            let _ = writeln!(out, "    derived: {note}");
        }
    }

    out
}

/// The glyph a phase renders as.
///
/// Unknown renders as `?`, which is not a lighter shade of any phase glyph: an
/// out-of-coverage stretch must not read as a quiet closed.
#[must_use]
pub fn glyph(phase: Phase) -> char {
    match phase {
        Phase::Closed => '.',
        Phase::PreOpen => '-',
        Phase::OpeningAuction | Phase::ClosingAuction => '=',
        Phase::ContinuousTrading => '#',
        Phase::MidDayBreak => ':',
        Phase::PostClose => '_',
        Phase::NonTradingInterruption => '!',
        // The vocabulary is closed and owned by the core, and this shell cannot extend it.
        // A phase added to the core after this board was built renders as "a phase this
        // build does not know how to draw" — which is deliberately not `?`, because that
        // means "outside coverage" and the two must never blur.
        _ => '*',
    }
}

/// The key printed under the board.
#[must_use]
pub fn legend() -> String {
    "# trading  = auction  - pre-open  : break  _ post-close  . closed  ! halt  ? not known"
        .to_owned()
}

fn render_row(
    timeline: &Timeline,
    interval: Interval,
    columns: usize,
    now: Option<&NowMarker>,
) -> String {
    let span = interval.start.saturating_nanos_until(interval.end).max(1);
    let mut cells = vec![' '; columns];

    for (column, cell) in cells.iter_mut().enumerate() {
        let offset = span * as_i128(column) / as_i128(columns);
        let at = interval.start.saturating_add_nanos(offset);
        *cell = match segment_at(timeline, at) {
            Some(TimelineSegment::Phase { answer, .. }) => glyph(answer.phase),
            Some(TimelineSegment::Unknown { .. }) => '?',
            None => ' ',
        };
    }

    if let Some(now) = now
        && interval.contains(now.instant)
    {
        let offset = interval.start.saturating_nanos_until(now.instant);
        let column = usize::try_from(offset * as_i128(columns) / span).unwrap_or(0);
        if let Some(cell) = cells.get_mut(column) {
            *cell = '|';
        }
    }

    let rendered: String = cells.into_iter().collect();
    format!("[{rendered}]")
}

fn as_i128(value: usize) -> i128 {
    i128::try_from(value).unwrap_or(i128::MAX)
}

fn segment_at(timeline: &Timeline, at: UtcInstant) -> Option<&TimelineSegment> {
    timeline
        .segments
        .iter()
        .find(|segment| segment.interval().contains(at))
}

fn status(timeline: &Timeline, now: Option<&NowMarker>) -> String {
    let Some(now) = now else {
        return String::new();
    };
    match segment_at(timeline, now.instant) {
        Some(TimelineSegment::Phase { answer, .. }) => answer.phase.as_str().to_owned(),
        Some(TimelineSegment::Unknown { .. }) => "not known".to_owned(),
        None => "not shown".to_owned(),
    }
}

fn axis_labels(zone: &TimeZone, interval: Interval, columns: usize) -> String {
    let span = interval.start.saturating_nanos_until(interval.end).max(1);
    let ticks = 6_usize;
    let mut line = vec![' '; columns + 2];

    for tick in 0..ticks {
        let offset = span * as_i128(tick) / as_i128(ticks);
        let at = interval.start.saturating_add_nanos(offset);
        let label = format_hour(zone, at);
        let column = 1 + (columns * tick / ticks);
        for (index, character) in label.chars().enumerate() {
            if let Some(cell) = line.get_mut(column + index) {
                *cell = character;
            }
        }
    }

    line.into_iter().collect()
}

fn zoned(zone: &TimeZone, at: UtcInstant) -> jiff::Zoned {
    let timestamp =
        Timestamp::from_nanosecond(at.as_nanos_since_unix_epoch()).unwrap_or(Timestamp::UNIX_EPOCH);
    timestamp.to_zoned(zone.clone())
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
