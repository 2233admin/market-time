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

pub mod svg;

pub use svg::{SvgOptions, render_svg, render_svg_with};

use jiff::Timestamp;
use jiff::tz::TimeZone;
use market_time_core::Phase;
use market_time_core::{AssetFamily, EvidenceRef, Interval, UtcInstant, VenueId, VenueProfile};
use market_time_core::{
    BandOverlap, BandSegment, BandState, OverlapSegment, OverlapState, SessionBand,
};
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

/// One venue's row: what it is called, what family it belongs to, and its timeline.
///
/// The label and family are display data. Resolution never sees them, and a row whose
/// dataset says nothing about presentation still renders — under the venue identifier,
/// which is a fallback rather than an invention.
#[derive(Debug, Clone)]
pub struct BoardRow {
    /// The phases this venue passes through across the queried interval.
    pub timeline: Timeline,
    /// The name to print.
    pub label: String,
    /// The city or region, printed under the name where a surface has room.
    pub location: Option<String>,
    /// The family this row is grouped under.
    pub family: Option<AssetFamily>,
}

impl BoardRow {
    /// Builds a row from a timeline and whatever the dataset says about presentation.
    #[must_use]
    pub fn new(timeline: Timeline, profile: Option<&VenueProfile>) -> Self {
        let label = profile.map_or_else(
            || timeline.venue.as_str().to_owned(),
            |profile| profile.label(timeline.venue.as_str()).to_owned(),
        );
        Self {
            label,
            location: profile.and_then(|profile| profile.location.clone()),
            family: profile.and_then(|profile| profile.family),
            timeline,
        }
    }

    /// The venue this row is about.
    #[must_use]
    pub fn venue(&self) -> &VenueId {
        &self.timeline.venue
    }
}

/// The derived session bands and their pairwise overlaps to draw beneath the venue rows.
///
/// Bundled as one struct, not two loose fields on [`BoardView`], so a caller can never end
/// up passing overlaps computed from a different band set than the one drawn above them.
/// Defaults to empty, which draws no band section at all — not an empty section with
/// nothing in it. A band and an overlap are never a venue's published schedule (see
/// `market_time_core`'s `bands` module docs); this crate draws no schedule of its own
/// either way, only what was derived and handed in.
#[derive(Debug, Clone, Default)]
pub struct BandSection {
    /// The derived bands to draw, each already produced by
    /// `market_time_core::derive_band`.
    pub bands: Vec<SessionBand>,
    /// The pairwise overlaps between those bands, each already produced by
    /// `market_time_core::derive_overlap`.
    pub overlaps: Vec<BandOverlap>,
}

impl BandSection {
    /// Whether there is nothing to draw.
    ///
    /// Overlaps never arrive without bands (there is nothing to overlap otherwise), so
    /// checking `bands` alone is enough to decide whether the whole section is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bands.is_empty()
    }
}

/// What the board renders.
#[derive(Debug, Clone)]
pub struct BoardView {
    /// The interval the axis spans.
    pub interval: Interval,
    /// One row per venue, all over the same interval.
    pub rows: Vec<BoardRow>,
    /// The instant being viewed, when the board is showing one.
    pub now: Option<NowMarker>,
    /// The IANA zone the axis is labelled in. Labelling only: the answers stay UTC.
    pub axis_zone: String,
    /// How many columns the timeline occupies.
    pub columns: usize,
    /// Derived bands and their overlaps, drawn beneath the venue rows. Empty by default:
    /// a caller that never mentions bands gets exactly the board this crate drew before
    /// bands existed.
    pub bands: BandSection,
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
        .map(|row| row.label.len())
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

    for (family, members) in grouped(&view.rows) {
        let heading = family.map_or("Other", AssetFamily::heading);
        let _ = writeln!(out, "{heading}");
        for row in members {
            let _ = writeln!(
                out,
                "{:label_width$}  {}  {}",
                row.label,
                render_row(&row.timeline, view.interval, columns, view.now.as_ref()),
                status(&row.timeline, view.now.as_ref()),
                label_width = label_width
            );
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "  axis: {} ({columns} columns)", view.axis_zone);
    if let Some(now) = &view.now {
        let _ = writeln!(
            out,
            "  count: {}",
            trading_count(&view.rows, now.instant).describe()
        );
    }
    if let Some(now) = &view.now {
        let _ = writeln!(
            out,
            "  now:  {} — {}",
            format_instant(&zone, now.instant),
            now.discipline.describe()
        );
    }
    let _ = writeln!(out, "  key:  {}", legend());

    if !view.bands.is_empty() {
        let _ = writeln!(out);
        out.push_str(&render_band_section(view, columns));
    }

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
fn render_sources(rows: &[BoardRow]) -> String {
    let mut out = String::new();

    for row in rows.iter().map(|row| &row.timeline) {
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

/// The rows, grouped for display: families in their listed order, then anything the
/// dataset left ungrouped.
///
/// Grouping is presentation. It reorders rows and never changes one.
#[must_use]
pub fn grouped(rows: &[BoardRow]) -> Vec<(Option<AssetFamily>, Vec<&BoardRow>)> {
    let mut groups: Vec<(Option<AssetFamily>, Vec<&BoardRow>)> = Vec::new();

    for family in AssetFamily::all() {
        let members: Vec<&BoardRow> = rows
            .iter()
            .filter(|row| row.family == Some(family))
            .collect();
        if !members.is_empty() {
            groups.push((Some(family), members));
        }
    }

    let ungrouped: Vec<&BoardRow> = rows.iter().filter(|row| row.family.is_none()).collect();
    if !ungrouped.is_empty() {
        groups.push((None, ungrouped));
    }

    groups
}

/// How many of the drawn venues are trading at `at`, and how many were asked about.
///
/// The second number is not decoration. "12 trading" alone hides whether the other
/// venues are closed or simply not known, and those are different claims.
#[must_use]
pub fn trading_count(rows: &[BoardRow], at: UtcInstant) -> TradingCount {
    let mut count = TradingCount::default();
    for row in rows {
        match segment_at(&row.timeline, at) {
            Some(TimelineSegment::Phase { answer, .. }) => {
                count.venues += 1;
                if answer.phase.is_trading() {
                    count.trading += 1;
                } else {
                    count.not_trading += 1;
                }
            }
            Some(TimelineSegment::Unknown { .. }) => {
                count.venues += 1;
                count.not_known += 1;
            }
            None => count.venues += 1,
        }
    }
    count
}

/// A count of what the board is showing at one instant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TradingCount {
    /// Venues drawn.
    pub venues: usize,
    /// Venues whose phase matches an order-matching state.
    pub trading: usize,
    /// Venues in a phase that is not matching — closed, on a break, in a halt.
    pub not_trading: usize,
    /// Venues outside their declared coverage, which is neither.
    pub not_known: usize,
}

impl TradingCount {
    /// A one-line summary that never reports an unknown as a closure.
    #[must_use]
    pub fn describe(self) -> String {
        let mut text = format!("{} of {} venues trading", self.trading, self.venues);
        if self.not_known > 0 {
            let _ = write!(text, ", {} not known", self.not_known);
        }
        text
    }
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

    mark_now(&mut cells, interval, now, columns, span);

    let rendered: String = cells.into_iter().collect();
    format!("[{rendered}]")
}

/// Marks the instant being viewed on a rendered row of cells.
///
/// Shared by the venue, band, and overlap row renderers so the marker's column arithmetic
/// cannot drift between them — a viewer reading straight down a column across all three
/// sections must see the same instant, not three approximations of it.
fn mark_now(
    cells: &mut [char],
    interval: Interval,
    now: Option<&NowMarker>,
    columns: usize,
    span: i128,
) {
    if let Some(now) = now
        && interval.contains(now.instant)
    {
        let offset = interval.start.saturating_nanos_until(now.instant);
        let column = usize::try_from(offset * as_i128(columns) / span).unwrap_or(0);
        if let Some(cell) = cells.get_mut(column) {
            *cell = '|';
        }
    }
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

/// The glyph a derived band's state renders as.
///
/// Deliberately not [`glyph`]'s vocabulary: a [`BandState`] is a computed grouping
/// (`market_time_core`'s `bands` module docs), never a venue's published [`Phase`], and
/// reusing `#`/`.`/`?` here would let a band row be misread as a venue's phase at a
/// glance. `?` in particular stays reserved for a venue's own out-of-coverage stretch —
/// [`BandState::Unknown`] renders as `x` instead, so the two kinds of "not known" are never
/// the same character.
#[must_use]
pub fn band_glyph(state: BandState) -> char {
    match state {
        BandState::Trading => '+',
        BandState::NotTrading => '~',
        BandState::Unknown => 'x',
        // The vocabulary is closed and owned by the core; kept distinct from `glyph`'s own
        // fallback `*` for the same reason `?` is kept distinct above.
        _ => '%',
    }
}

/// The glyph a computed overlap's state renders as.
///
/// Shares [`band_glyph`]'s three characters — `+`/`~`/`x` read as "trading-like / not /
/// unknown" whether the row is a band or an overlap of two bands — rather than inventing a
/// fourth vocabulary a reader would have to learn on top of the other two.
#[must_use]
pub fn overlap_glyph(state: OverlapState) -> char {
    match state {
        OverlapState::Overlapping => '+',
        OverlapState::NotOverlapping => '~',
        OverlapState::Unknown => 'x',
        _ => '%',
    }
}

/// The key for the derived band and overlap vocabulary above.
///
/// Kept separate from [`legend()`], never merged into it: the phase legend is printed
/// whether or not a board has any bands to draw, and folding band vocabulary into it would
/// put a key in front of a reader for rows that are not on screen. This one is only
/// printed alongside an actual band section.
#[must_use]
pub fn band_legend() -> String {
    "+ trading/overlapping  ~ not trading/not overlapping  x unknown  (derived — not a phase)"
        .to_owned()
}

/// The label a band's row is printed under: its id and display name together, so an
/// abbreviated id never stands alone without the human-readable name next to it.
pub(crate) fn band_label(band: &SessionBand) -> String {
    format!(
        "{}: {}",
        band.definition().id(),
        band.definition().display_name()
    )
}

/// The label an overlap's row is printed under: the two band ids it was computed from.
pub(crate) fn overlap_label(overlap: &BandOverlap) -> String {
    let [left, right] = overlap.bands();
    format!("{left} x {right}")
}

/// The word a [`BandState`] renders as in a status column or a hover title.
pub(crate) fn band_state_word(state: BandState) -> &'static str {
    match state {
        BandState::Trading => "trading",
        BandState::NotTrading => "not trading",
        BandState::Unknown => "unknown",
        _ => "unrecognized",
    }
}

/// The word an [`OverlapState`] renders as in a status column or a hover title.
pub(crate) fn overlap_state_word(state: OverlapState) -> &'static str {
    match state {
        OverlapState::Overlapping => "overlapping",
        OverlapState::NotOverlapping => "not overlapping",
        OverlapState::Unknown => "unknown",
        _ => "unrecognized",
    }
}

fn band_segment_at(band: &SessionBand, at: UtcInstant) -> Option<&BandSegment> {
    band.segments()
        .iter()
        .find(|segment| segment.interval.contains(at))
}

fn overlap_segment_at(overlap: &BandOverlap, at: UtcInstant) -> Option<&OverlapSegment> {
    overlap
        .segments()
        .iter()
        .find(|segment| segment.interval.contains(at))
}

fn render_band_row(
    band: &SessionBand,
    interval: Interval,
    columns: usize,
    now: Option<&NowMarker>,
) -> String {
    let span = interval.start.saturating_nanos_until(interval.end).max(1);
    let mut cells = vec![' '; columns];

    for (column, cell) in cells.iter_mut().enumerate() {
        let offset = span * as_i128(column) / as_i128(columns);
        let at = interval.start.saturating_add_nanos(offset);
        *cell = band_segment_at(band, at).map_or(' ', |segment| band_glyph(segment.state));
    }

    mark_now(&mut cells, interval, now, columns, span);

    let rendered: String = cells.into_iter().collect();
    format!("[{rendered}]")
}

fn render_overlap_row(
    overlap: &BandOverlap,
    interval: Interval,
    columns: usize,
    now: Option<&NowMarker>,
) -> String {
    let span = interval.start.saturating_nanos_until(interval.end).max(1);
    let mut cells = vec![' '; columns];

    for (column, cell) in cells.iter_mut().enumerate() {
        let offset = span * as_i128(column) / as_i128(columns);
        let at = interval.start.saturating_add_nanos(offset);
        *cell = overlap_segment_at(overlap, at).map_or(' ', |segment| overlap_glyph(segment.state));
    }

    mark_now(&mut cells, interval, now, columns, span);

    let rendered: String = cells.into_iter().collect();
    format!("[{rendered}]")
}

fn band_status(band: &SessionBand, now: Option<&NowMarker>) -> String {
    let Some(now) = now else {
        return String::new();
    };
    match band_segment_at(band, now.instant) {
        Some(segment) => band_state_word(segment.state).to_owned(),
        None => "not shown".to_owned(),
    }
}

fn overlap_status(overlap: &BandOverlap, now: Option<&NowMarker>) -> String {
    let Some(now) = now else {
        return String::new();
    };
    match overlap_segment_at(overlap, now.instant) {
        Some(segment) => overlap_state_word(segment.state).to_owned(),
        None => "not shown".to_owned(),
    }
}

/// Renders the derived band rows, then the derived overlap rows, beneath the venue
/// section.
///
/// Every row's label ends "(derived)" and both headings say DERIVED, because a band or an
/// overlap is never a venue's published schedule (`market_time_core`'s `bands` module
/// docs) — a reader scanning quickly must not be able to mistake one of these rows for a
/// venue row above it. The glyphs come from [`band_glyph`]/[`overlap_glyph`], never
/// [`glyph`], for the same reason. Only called when [`BandSection::is_empty`] is false, so
/// a zero-band view never reaches this function at all.
fn render_band_section(view: &BoardView, columns: usize) -> String {
    let mut out = String::new();
    let bands = &view.bands.bands;
    let overlaps = &view.bands.overlaps;

    let label_width = bands
        .iter()
        .map(|band| band_label(band).len())
        .chain(overlaps.iter().map(|overlap| overlap_label(overlap).len()))
        .max()
        .unwrap_or(6)
        .max(6);

    let _ = writeln!(out, "DERIVED BANDS (never a venue's published schedule)");
    for band in bands {
        let _ = writeln!(
            out,
            "{:label_width$}  {}  {} (derived)",
            band_label(band),
            render_band_row(band, view.interval, columns, view.now.as_ref()),
            band_status(band, view.now.as_ref()),
            label_width = label_width,
        );
        let _ = writeln!(
            out,
            "  derived  {}",
            band.definition().derivation().reasoning()
        );
    }

    if !overlaps.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "DERIVED OVERLAPS");
        for overlap in overlaps {
            let _ = writeln!(
                out,
                "{:label_width$}  {}  {} (derived)",
                overlap_label(overlap),
                render_overlap_row(overlap, view.interval, columns, view.now.as_ref()),
                overlap_status(overlap, view.now.as_ref()),
                label_width = label_width,
            );
            let _ = writeln!(out, "  derived  {}", overlap.derivation().reasoning());
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "  bands key:  {}", band_legend());

    out
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
