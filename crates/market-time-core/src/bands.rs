//! Regional session bands and the overlap windows between them.
//!
//! A "London session" or a "Tokyo session" is not a fact any venue publishes: it is the
//! conventional FX-desk grouping of whichever venues a person considers to be trading
//! during that stretch of the day. Task 3 of `global-market-coverage` gives that grouping
//! a derived, evidenced shape instead of a hardcoded picture — see
//! `openspec/changes/global-market-coverage/specs/session-overlap/spec.md`.
//!
//! A [`SessionBand`] is always derived from its members' [`Timeline`]s, never itself a rule
//! with evidence of its own (Principle I does not apply to it the way it applies to a
//! venue's schedule): what it carries instead is a [`DerivationNote`] naming the venue set
//! and the reasoning for grouping them, so nothing here can be mistaken for a published
//! fact ("no published band is claimed", the second scenario in the spec above).
//!
//! Two rules carry the honesty this module exists to protect:
//!
//! - **An unknown member is never dropped from the vote.** If any member's timeline is
//!   outside its declared coverage for a stretch, the whole band is [`BandState::Unknown`]
//!   there — never narrowed to what the remaining members say, because an unknown venue
//!   might have been trading and silently excluding it would understate a real gap in the
//!   evidence ("An input is out of coverage", spec above; task 3.3).
//! - **Uncertainty only ever widens.** A band's or an overlap's uncertainty is folded with
//!   [`Uncertainty::widest`] over its contributors, so a combined answer is never more
//!   precise than the least precise input.
//!
//! Like the rest of this crate, everything here is pure: instants and timelines are passed
//! in, nothing is read from a clock or a dataset (Constitution Principle IV).

use crate::evidence::{DerivationNote, EvidenceError};
use crate::ids::{BandId, VenueId};
use crate::instant::{Interval, UtcInstant};
use crate::query::{Timeline, TimelineSegment};
use crate::uncertainty::Uncertainty;
use std::fmt;

/// The venue set and reasoning a [`SessionBand`] is derived from.
///
/// A band is not itself evidenced the way a venue's rule is (Principle I): what it carries
/// instead is the [`DerivationNote`] saying why these particular venues were grouped, so an
/// inspector always sees the reasoning rather than a bare venue list ("A band is
/// inspected", session-overlap spec).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandDefinition {
    id: BandId,
    display_name: String,
    members: Vec<VenueId>,
    derivation: DerivationNote,
}

impl BandDefinition {
    /// Builds a band definition.
    ///
    /// # Errors
    ///
    /// Returns [`BandError::NoMembers`] when `members` is empty — a band with nothing to
    /// derive from is not a band — and [`BandError::DuplicateMember`] when the same venue
    /// is listed twice, which would otherwise let one member's trading state be counted
    /// twice toward the band.
    pub fn new(
        id: BandId,
        display_name: impl Into<String>,
        members: Vec<VenueId>,
        derivation: DerivationNote,
    ) -> Result<Self, BandError> {
        if members.is_empty() {
            return Err(BandError::NoMembers);
        }
        let mut seen: Vec<&VenueId> = Vec::with_capacity(members.len());
        for member in &members {
            if seen.contains(&member) {
                return Err(BandError::DuplicateMember(member.clone()));
            }
            seen.push(member);
        }

        Ok(Self {
            id,
            display_name: display_name.into(),
            members,
            derivation,
        })
    }

    /// The band's identifier.
    #[must_use]
    pub fn id(&self) -> &BandId {
        &self.id
    }

    /// The band's display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// The venues this band is derived from.
    #[must_use]
    pub fn members(&self) -> &[VenueId] {
        &self.members
    }

    /// Why these venues were grouped into one band.
    #[must_use]
    pub fn derivation(&self) -> &DerivationNote {
        &self.derivation
    }
}

/// The band's trading state over a stretch of time.
///
/// Mirrors `Phase::is_trading` at the band level: a band is trading when at least one
/// member is trading, and unknown when at least one member's coverage does not reach this
/// stretch. An unknown member always wins over a trading or a non-trading one — see the
/// module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BandState {
    /// At least one member is trading, and every member is inside declared coverage.
    Trading,
    /// Every member is inside declared coverage, and none is trading.
    NotTrading,
    /// At least one member is outside its declared coverage for this stretch.
    Unknown,
}

/// One atomic stretch of a [`SessionBand`]: a state, plus who is trading and who is
/// unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandSegment {
    /// The stretch this segment occupies.
    pub interval: Interval,
    /// The band's state over that stretch.
    pub state: BandState,
    /// How precisely the state is known.
    ///
    /// `None` only when *every* contributing member is itself unknown for this stretch:
    /// with no member's schedule in evidence, precision is not a meaningful question to ask
    /// of the slice, and `None` must never be read as "exact" or as narrower than any
    /// published number — the honest reading is "wider than anything stated", the same
    /// stance [`Uncertainty::ProcessStart`] takes for an unpublished spread. Wherever at
    /// least one member is known, this is `Some`, folded with [`Uncertainty::widest`] over
    /// the known contributors.
    pub uncertainty: Option<Uncertainty>,
    /// Members trading over this stretch, sorted by identifier.
    pub venues_trading: Vec<VenueId>,
    /// Members outside their declared coverage over this stretch, sorted by identifier.
    pub venues_unknown: Vec<VenueId>,
}

/// A regional session band, derived from its constituent venues' timelines.
///
/// The segments tile `interval` exactly — the same invariant [`Timeline`] carries — so
/// every instant belongs to exactly one segment, checkable by
/// [`SessionBand::tiles_interval`] rather than merely promised in prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBand {
    definition: BandDefinition,
    interval: Interval,
    segments: Vec<BandSegment>,
}

impl SessionBand {
    /// The definition this band was derived from.
    #[must_use]
    pub fn definition(&self) -> &BandDefinition {
        &self.definition
    }

    /// The interval the band was derived over.
    #[must_use]
    pub fn interval(&self) -> Interval {
        self.interval
    }

    /// The segments tiling that interval, in order.
    #[must_use]
    pub fn segments(&self) -> &[BandSegment] {
        &self.segments
    }

    /// Whether the segments tile the derived interval exactly: no gaps, no overlaps, every
    /// instant owned by exactly one segment. Mirrors [`Timeline::tiles_interval`].
    #[must_use]
    pub fn tiles_interval(&self) -> bool {
        let mut cursor = self.interval.start;
        for segment in &self.segments {
            if segment.interval.start != cursor {
                return false;
            }
            cursor = segment.interval.end;
        }
        cursor == self.interval.end
    }
}

/// Derives a session band from its members' timelines.
///
/// Cuts the queried interval at every member's segment boundary, then classifies each
/// atomic slice: if any member is unknown there, the whole slice is [`BandState::Unknown`]
/// — the band's answer is never narrowed to what the remaining members say, because a
/// silently dropped unknown member could have been trading (task 3.3; "An input is out of
/// coverage" in the spec). Otherwise the slice is [`BandState::Trading`] when any member's
/// phase is trading, else [`BandState::NotTrading`]. Adjacent slices that agree on state,
/// trading venues, and unknown venues are merged, exactly as [`Timeline`] merges adjacent
/// identical phases.
///
/// Timelines in `timelines` for venues outside `definition`'s member set are ignored.
///
/// # Errors
///
/// Returns [`BandError::MissingMemberTimeline`] when a member has no timeline in
/// `timelines`, [`BandError::TimelineNotTiled`] when a member's timeline does not tile its
/// own interval, and [`BandError::MemberIntervalMismatch`] when the supplied member
/// timelines do not all cover the same interval — a band needs one shared interval to cut
/// against, not one per member.
pub fn derive_band(
    definition: &BandDefinition,
    timelines: &[Timeline],
) -> Result<SessionBand, BandError> {
    let mut members: Vec<&Timeline> = Vec::with_capacity(definition.members().len());
    for member in definition.members() {
        let timeline = timelines
            .iter()
            .find(|timeline| &timeline.venue == member)
            .ok_or_else(|| BandError::MissingMemberTimeline(member.clone()))?;
        members.push(timeline);
    }

    for timeline in &members {
        if !timeline.tiles_interval() {
            return Err(BandError::TimelineNotTiled(timeline.venue.clone()));
        }
    }

    let Some(first) = members.first() else {
        // `BandDefinition::new` rejects an empty member list, so this is unreachable in
        // practice; kept as a returned error rather than an unwrap so it cannot panic.
        return Err(BandError::NoMembers);
    };
    let interval = first.interval;

    for timeline in &members {
        if timeline.interval != interval {
            return Err(BandError::MemberIntervalMismatch {
                first_venue: first.venue.clone(),
                first_interval: interval,
                other_venue: timeline.venue.clone(),
                other_interval: timeline.interval,
            });
        }
    }

    let mut cut_points: Vec<UtcInstant> = vec![interval.start, interval.end];
    for timeline in &members {
        for segment in &timeline.segments {
            cut_points.push(segment.interval().start);
            cut_points.push(segment.interval().end);
        }
    }
    cut_points.sort();
    cut_points.dedup();

    let mut segments = Vec::new();
    for (&start, &end) in cut_points.iter().zip(cut_points.iter().skip(1)) {
        if start >= end {
            continue;
        }
        let Ok(slice) = Interval::new(start, end) else {
            continue;
        };

        let mut trading = Vec::new();
        let mut unknown = Vec::new();
        let mut known_uncertainty: Option<Uncertainty> = None;

        for timeline in &members {
            match segment_at(timeline, start) {
                TimelineSegment::Unknown { .. } => unknown.push(timeline.venue.clone()),
                TimelineSegment::Phase { answer, .. } => {
                    known_uncertainty =
                        widen_optional(known_uncertainty, Some(answer.uncertainty.clone()));
                    if answer.phase.is_trading() {
                        trading.push(timeline.venue.clone());
                    }
                }
            }
        }

        trading.sort();
        unknown.sort();

        let state = if !unknown.is_empty() {
            BandState::Unknown
        } else if !trading.is_empty() {
            BandState::Trading
        } else {
            BandState::NotTrading
        };

        segments.push(BandSegment {
            interval: slice,
            state,
            uncertainty: known_uncertainty,
            venues_trading: trading,
            venues_unknown: unknown,
        });
    }

    Ok(SessionBand {
        definition: definition.clone(),
        interval,
        segments: merge_band_segments(segments),
    })
}

/// Finds the timeline segment that owns `at`.
///
/// Only called after the timeline's `tiles_interval` invariant has been checked and `at` has
/// been taken from a cut point strictly inside `[timeline.interval.start,
/// timeline.interval.end)`, so a segment owning it is guaranteed to exist. Kept as a private
/// helper — rather than inlined into the public `derive_band` — precisely so that guarantee
/// stays out of the public panic surface.
fn segment_at(timeline: &Timeline, at: UtcInstant) -> &TimelineSegment {
    timeline
        .segments
        .iter()
        .find(|segment| segment.interval().contains(at))
        .expect("tiles_interval was checked before this lookup")
}

/// The wider of two optional uncertainties, treating `None` as "nothing known" rather than
/// as an unbounded width.
///
/// This is the counterpart to [`Uncertainty::widest`] for slices that may have nothing to
/// report at all: an all-unknown slice has no numeric or venue-published uncertainty to
/// fold in, and `None` here means exactly that — never "wider than everything", the way an
/// unbounded [`Uncertainty`] variant reads.
fn widen_optional(a: Option<Uncertainty>, b: Option<Uncertainty>) -> Option<Uncertainty> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.widest(b)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

/// Merges adjacent slices that agree on state, trading venues, and unknown venues.
///
/// The band-level counterpart of `Timeline`'s "an always-on venue is one segment, not
/// seven": a stretch that does not change state should not be split into many segments
/// purely because two different members' boundaries happened to land inside it.
fn merge_band_segments(segments: Vec<BandSegment>) -> Vec<BandSegment> {
    let mut merged: Vec<BandSegment> = Vec::with_capacity(segments.len());
    for segment in segments {
        if let Some(previous) = merged.last_mut()
            && previous.interval.end == segment.interval.start
            && previous.state == segment.state
            && previous.venues_trading == segment.venues_trading
            && previous.venues_unknown == segment.venues_unknown
        {
            previous.interval.end = segment.interval.end;
            previous.uncertainty =
                widen_optional(previous.uncertainty.clone(), segment.uncertainty);
            continue;
        }
        merged.push(segment);
    }
    merged
}

/// Whether two bands are simultaneously trading over a stretch of time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlapState {
    /// Both bands are trading.
    Overlapping,
    /// Both bands are known for this stretch, and at least one is not trading.
    NotOverlapping,
    /// At least one band is unknown for this stretch.
    ///
    /// Never [`OverlapState::NotOverlapping`]: an unknown band cannot prove absence of
    /// overlap ("An input is out of coverage", session-overlap spec), so an unknown input
    /// always wins.
    Unknown,
}

/// One atomic stretch of a [`BandOverlap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlapSegment {
    /// The stretch this segment occupies.
    pub interval: Interval,
    /// Whether the two bands overlap over that stretch.
    pub state: OverlapState,
    /// How precisely the state is known, under the same `None` convention as
    /// [`BandSegment::uncertainty`].
    pub uncertainty: Option<Uncertainty>,
}

/// The computed overlap between two [`SessionBand`]s.
///
/// Never a published fact: the session-overlap spec requires overlap windows to be
/// computed from the bands they overlap, and [`BandOverlap::derivation`] says so, so an
/// overlap can never be mistaken for something a venue announced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandOverlap {
    bands: [BandId; 2],
    interval: Interval,
    segments: Vec<OverlapSegment>,
    derivation: DerivationNote,
}

impl BandOverlap {
    /// The two bands this overlap was computed from.
    #[must_use]
    pub fn bands(&self) -> &[BandId; 2] {
        &self.bands
    }

    /// The interval the overlap was computed over.
    #[must_use]
    pub fn interval(&self) -> Interval {
        self.interval
    }

    /// The segments tiling that interval, in order.
    #[must_use]
    pub fn segments(&self) -> &[OverlapSegment] {
        &self.segments
    }

    /// Why this overlap is a computed value, never a published fact.
    #[must_use]
    pub fn derivation(&self) -> &DerivationNote {
        &self.derivation
    }

    /// The `Overlapping` windows only, without making every consumer re-filter
    /// [`BandOverlap::segments`] itself.
    pub fn windows(&self) -> impl Iterator<Item = &OverlapSegment> {
        self.segments
            .iter()
            .filter(|segment| segment.state == OverlapState::Overlapping)
    }

    /// Whether the segments tile the derived interval exactly.
    #[must_use]
    pub fn tiles_interval(&self) -> bool {
        let mut cursor = self.interval.start;
        for segment in &self.segments {
            if segment.interval.start != cursor {
                return false;
            }
            cursor = segment.interval.end;
        }
        cursor == self.interval.end
    }
}

/// Computes the overlap between two session bands.
///
/// Cuts the shared interval at both bands' segment boundaries. A slice is
/// [`OverlapState::Unknown`] when either band is unknown there — never `NotOverlapping`,
/// because an unknown band cannot prove absence of overlap. Otherwise it is
/// [`OverlapState::Overlapping`] when both bands are trading, else `NotOverlapping`.
/// Uncertainty per slice is the widest of the two contributing band segments', under the
/// same `None` convention as [`BandSegment::uncertainty`]. Adjacent slices that agree on
/// state are merged.
///
/// # Errors
///
/// Returns [`BandError::SameBand`] when `a` and `b` carry the same [`BandId`] — an overlap
/// between a band and itself is not a meaningful question — and
/// [`BandError::BandIntervalMismatch`] when the two bands were derived over different
/// intervals.
pub fn derive_overlap(a: &SessionBand, b: &SessionBand) -> Result<BandOverlap, BandError> {
    if a.definition.id == b.definition.id {
        return Err(BandError::SameBand(a.definition.id.clone()));
    }
    if a.interval != b.interval {
        return Err(BandError::BandIntervalMismatch {
            first: a.definition.id.clone(),
            second: b.definition.id.clone(),
        });
    }

    let mut cut_points: Vec<UtcInstant> = vec![a.interval.start, a.interval.end];
    for segment in a.segments.iter().chain(b.segments.iter()) {
        cut_points.push(segment.interval.start);
        cut_points.push(segment.interval.end);
    }
    cut_points.sort();
    cut_points.dedup();

    let mut segments = Vec::new();
    for (&start, &end) in cut_points.iter().zip(cut_points.iter().skip(1)) {
        if start >= end {
            continue;
        }
        let Ok(slice) = Interval::new(start, end) else {
            continue;
        };

        let left = overlap_segment_at(a, start);
        let right = overlap_segment_at(b, start);

        let state = if left.state == BandState::Unknown || right.state == BandState::Unknown {
            OverlapState::Unknown
        } else if left.state == BandState::Trading && right.state == BandState::Trading {
            OverlapState::Overlapping
        } else {
            OverlapState::NotOverlapping
        };

        let uncertainty = widen_optional(left.uncertainty.clone(), right.uncertainty.clone());

        segments.push(OverlapSegment {
            interval: slice,
            state,
            uncertainty,
        });
    }

    let derivation = derivation_note(&format!(
        "computed as the overlap of session bands \"{}\" and \"{}\"; no venue publishes a \
         combined window",
        a.definition.display_name, b.definition.display_name
    ))?;

    Ok(BandOverlap {
        bands: [a.definition.id.clone(), b.definition.id.clone()],
        interval: a.interval,
        segments: merge_overlap_segments(segments),
        derivation,
    })
}

/// Finds the band segment that owns `at`. Same guarantee and same reason for being private
/// as `segment_at`.
fn overlap_segment_at(band: &SessionBand, at: UtcInstant) -> &BandSegment {
    band.segments
        .iter()
        .find(|segment| segment.interval.contains(at))
        .expect("tiles_interval was checked before this lookup")
}

/// Builds the overlap's derivation note from a reasoning string this module generated
/// itself, never a blank one — so the only way `DerivationNote::new` fails here is a bug in
/// the format string above, which is reported rather than unwrapped past.
fn derivation_note(reasoning: &str) -> Result<DerivationNote, BandError> {
    DerivationNote::new(reasoning).map_err(BandError::Evidence)
}

/// Merges adjacent overlap slices that agree on state.
fn merge_overlap_segments(segments: Vec<OverlapSegment>) -> Vec<OverlapSegment> {
    let mut merged: Vec<OverlapSegment> = Vec::with_capacity(segments.len());
    for segment in segments {
        if let Some(previous) = merged.last_mut()
            && previous.interval.end == segment.interval.start
            && previous.state == segment.state
        {
            previous.interval.end = segment.interval.end;
            previous.uncertainty =
                widen_optional(previous.uncertainty.clone(), segment.uncertainty);
            continue;
        }
        merged.push(segment);
    }
    merged
}

/// Why a band or an overlap could not be derived.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BandError {
    /// A band definition was given no member venues.
    NoMembers,
    /// The same venue was listed as a member more than once.
    DuplicateMember(VenueId),
    /// A member venue has no timeline among those supplied to [`derive_band`].
    MissingMemberTimeline(VenueId),
    /// A member's timeline does not tile its own declared interval.
    TimelineNotTiled(VenueId),
    /// Member timelines were derived over different intervals.
    MemberIntervalMismatch {
        /// The first member checked, and the interval its timeline covers.
        first_venue: VenueId,
        /// The interval `first_venue`'s timeline covers.
        first_interval: Interval,
        /// The member whose timeline disagreed.
        other_venue: VenueId,
        /// The interval `other_venue`'s timeline covers.
        other_interval: Interval,
    },
    /// [`derive_overlap`] was given the same band twice.
    SameBand(BandId),
    /// The two bands passed to [`derive_overlap`] were derived over different intervals.
    BandIntervalMismatch {
        /// The first band.
        first: BandId,
        /// The second band.
        second: BandId,
    },
    /// The overlap's own derivation note could not be built.
    ///
    /// Reachable only if a future change makes the generated reasoning text blank; kept as
    /// a variant rather than a panic so that bug would surface as a `Result`, not a crash.
    Evidence(EvidenceError),
}

impl fmt::Display for BandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMembers => f.write_str("a session band needs at least one member venue"),
            Self::DuplicateMember(venue) => {
                write!(f, "venue {venue} is listed as a member more than once")
            }
            Self::MissingMemberTimeline(venue) => {
                write!(
                    f,
                    "member venue {venue} has no timeline among those supplied"
                )
            }
            Self::TimelineNotTiled(venue) => {
                write!(
                    f,
                    "member venue {venue}'s timeline does not tile its own interval"
                )
            }
            Self::MemberIntervalMismatch {
                first_venue,
                first_interval,
                other_venue,
                other_interval,
            } => write!(
                f,
                "member {first_venue} covers {}..{} but member {other_venue} covers {}..{}",
                first_interval.start, first_interval.end, other_interval.start, other_interval.end,
            ),
            Self::SameBand(id) => write!(f, "band {id} cannot be overlapped with itself"),
            Self::BandIntervalMismatch { first, second } => {
                write!(
                    f,
                    "band {first} and band {second} were derived over different intervals"
                )
            }
            Self::Evidence(err) => write!(f, "overlap derivation note: {err}"),
        }
    }
}

impl std::error::Error for BandError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant(seconds: i64) -> UtcInstant {
        UtcInstant::from_seconds_since_unix_epoch(seconds)
    }

    #[test]
    fn widen_optional_treats_none_as_nothing_known() {
        assert_eq!(widen_optional(None, None), None);
        assert_eq!(
            widen_optional(None, Some(Uncertainty::minutes(1))),
            Some(Uncertainty::minutes(1))
        );
        assert_eq!(
            widen_optional(Some(Uncertainty::minutes(5)), None),
            Some(Uncertainty::minutes(5))
        );
    }

    #[test]
    fn widen_optional_picks_the_wider_of_two_known_values() {
        let wide = widen_optional(
            Some(Uncertainty::minutes(1)),
            Some(Uncertainty::minutes(10)),
        )
        .expect("both sides known");
        assert!(wide.is_at_least_as_wide_as(&Uncertainty::minutes(10)));
    }

    fn segment(
        start: i64,
        end: i64,
        state: BandState,
        uncertainty: Option<Uncertainty>,
    ) -> BandSegment {
        BandSegment {
            interval: Interval::new(instant(start), instant(end)).expect("valid interval"),
            state,
            uncertainty,
            venues_trading: Vec::new(),
            venues_unknown: Vec::new(),
        }
    }

    #[test]
    fn adjacent_matching_band_segments_merge_and_widen_uncertainty() {
        let segments = vec![
            segment(0, 10, BandState::Trading, Some(Uncertainty::minutes(1))),
            segment(10, 20, BandState::Trading, Some(Uncertainty::minutes(5))),
        ];
        let merged = merge_band_segments(segments);
        assert_eq!(merged.len(), 1, "same state and same venue sets merge");
        assert_eq!(merged[0].interval.start, instant(0));
        assert_eq!(merged[0].interval.end, instant(20));
        assert!(
            merged[0]
                .uncertainty
                .as_ref()
                .expect("both sides known")
                .is_at_least_as_wide_as(&Uncertainty::minutes(5))
        );
    }

    #[test]
    fn band_segments_with_different_state_do_not_merge() {
        let segments = vec![
            segment(0, 10, BandState::Trading, Some(Uncertainty::minutes(1))),
            segment(10, 20, BandState::NotTrading, Some(Uncertainty::minutes(1))),
        ];
        assert_eq!(merge_band_segments(segments).len(), 2);
    }
}
