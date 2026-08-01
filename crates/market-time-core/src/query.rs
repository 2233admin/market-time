//! The shapes an answer comes back in.

use crate::coverage::CoverageGap;
use crate::event::EventOccurrence;
use crate::evidence::EvidenceRef;
use crate::ids::{DatasetRevisionId, VenueId};
use crate::instant::{Interval, UtcInstant};
use crate::phase::Phase;
use crate::uncertainty::Uncertainty;

/// A phase boundary and how precisely it is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseBoundary {
    /// The instant the boundary falls on.
    pub instant: UtcInstant,
    /// How precisely that instant is known — including the process-start case, where the
    /// boundary has a spread the venue does not quantify.
    pub uncertainty: Uncertainty,
}

/// A phase answer for one venue at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseAnswer {
    /// The venue.
    pub venue: VenueId,
    /// The phase, from the shared vocabulary.
    pub phase: Phase,
    /// The category of the calendar rule that governs this venue-local date.
    pub calendar_rule_kind: String,
    /// The venue's own label for the governing calendar rule.
    pub calendar_label: String,
    /// When the phase started.
    pub boundary_start: PhaseBoundary,
    /// When it ends.
    pub boundary_end: PhaseBoundary,
    /// Events scheduled inside this phase. May be empty; events do not tile.
    pub events: Vec<EventOccurrence>,
    /// The evidence for the rules that produced this answer.
    pub evidence: Vec<EvidenceRef>,
    /// Whether the governing rule is our reading rather than the venue's wording.
    pub derived_reasoning: Option<String>,
    /// The answer-level uncertainty: never narrower than either boundary's.
    pub uncertainty: Uncertainty,
    /// The revisions that produced the answer.
    pub dataset_revisions: Vec<DatasetRevisionId>,
}

/// Either a phase or an explicit unknown.
///
/// `Unknown` is not an error and not `Phase::Closed`. A consumer that ignores the
/// distinction cannot accidentally read a gap as a closed market, because the two are
/// different variants rather than the same value rendered differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseOutcome {
    /// The venue's phase at the queried instant.
    Known(Box<PhaseAnswer>),
    /// The instant falls outside declared coverage.
    Unknown(Box<CoverageGap>),
}

impl PhaseOutcome {
    /// The phase, when the outcome is known.
    #[must_use]
    pub fn phase(&self) -> Option<Phase> {
        match self {
            Self::Known(answer) => Some(answer.phase),
            Self::Unknown(_) => None,
        }
    }

    /// Whether this outcome is an explicit unknown.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_))
    }
}

/// One venue's outcome inside a multi-venue answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueOutcome {
    /// The venue.
    pub venue: VenueId,
    /// Its outcome at the shared instant.
    pub outcome: PhaseOutcome,
}

/// One stretch of a timeline: a phase, or a stretch outside coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineSegment {
    /// A phase, occupying `interval`.
    Phase {
        /// The stretch this phase occupies, clipped to the queried interval.
        interval: Interval,
        /// The answer for that stretch.
        answer: Box<PhaseAnswer>,
    },
    /// A stretch outside declared coverage.
    Unknown {
        /// The stretch that is not known, clipped to the queried interval.
        interval: Interval,
        /// Which coverage declaration this stretch falls outside.
        gap: Box<CoverageGap>,
    },
}

impl TimelineSegment {
    /// The stretch this segment occupies.
    #[must_use]
    pub fn interval(&self) -> Interval {
        match self {
            Self::Phase { interval, .. } | Self::Unknown { interval, .. } => *interval,
        }
    }

    /// The phase, when this segment is a phase.
    #[must_use]
    pub fn phase(&self) -> Option<Phase> {
        match self {
            Self::Phase { answer, .. } => Some(answer.phase),
            Self::Unknown { .. } => None,
        }
    }

    /// Whether this stretch is outside coverage.
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

/// One venue's phases across a queried interval.
///
/// The segments tile `interval` with no gaps and no overlaps: every instant in the
/// interval belongs to exactly one segment, and a stretch outside coverage is an
/// `Unknown` segment rather than a hole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    /// The venue.
    pub venue: VenueId,
    /// The interval that was asked about.
    pub interval: Interval,
    /// The segments covering it, in order.
    pub segments: Vec<TimelineSegment>,
}

impl Timeline {
    /// Whether the segments tile the queried interval exactly.
    ///
    /// This is the invariant the board's rendering depends on, checkable by consumers
    /// rather than merely promised in prose.
    #[must_use]
    pub fn tiles_interval(&self) -> bool {
        let mut cursor = self.interval.start;
        for segment in &self.segments {
            if segment.interval().start != cursor {
                return false;
            }
            cursor = segment.interval().end;
        }
        cursor == self.interval.end
    }
}
