//! Shared phase vocabulary and the structurally validated timeline.

use std::{error::Error, fmt};

use crate::{CoverageRange, DatasetRevisionId, EvidenceRef, Uncertainty, UtcInstant};

/// The shared cross-venue market-phase vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Phase {
    /// No trading session is active.
    Closed,
    /// A pre-open period before an opening auction or continuous session.
    PreOpen,
    /// The venue's opening auction.
    OpeningAuction,
    /// Continuous order matching or trading.
    ContinuousTrading,
    /// A scheduled mid-day pause.
    MidDayBreak,
    /// The venue's closing auction.
    ClosingAuction,
    /// A post-close trading or reporting period.
    PostClose,
    /// An announced interruption that temporarily replaces normal trading.
    NonTradingInterruption,
}

impl Phase {
    /// Returns the stable machine-readable spelling of this shared phase.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::PreOpen => "pre_open",
            Self::OpeningAuction => "opening_auction",
            Self::ContinuousTrading => "continuous_trading",
            Self::MidDayBreak => "mid_day_break",
            Self::ClosingAuction => "closing_auction",
            Self::PostClose => "post_close",
            Self::NonTradingInterruption => "non_trading_interruption",
        }
    }
}

impl std::str::FromStr for Phase {
    type Err = PhaseParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "closed" => Ok(Self::Closed),
            "pre_open" => Ok(Self::PreOpen),
            "opening_auction" => Ok(Self::OpeningAuction),
            "continuous_trading" => Ok(Self::ContinuousTrading),
            "mid_day_break" => Ok(Self::MidDayBreak),
            "closing_auction" => Ok(Self::ClosingAuction),
            "post_close" => Ok(Self::PostClose),
            "non_trading_interruption" => Ok(Self::NonTradingInterruption),
            _ => Err(PhaseParseError),
        }
    }
}

/// Returned when text is not one of the closed shared phase names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhaseParseError;

impl fmt::Display for PhaseParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("unknown shared market phase")
    }
}

impl Error for PhaseParseError {}

/// One boundary of a resolved market phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseBoundary {
    /// The represented UTC instant.
    pub instant: UtcInstant,
    /// The evidence-supported uncertainty of this boundary.
    pub uncertainty: Uncertainty,
}

impl PhaseBoundary {
    /// Constructs a phase boundary from an explicit instant and uncertainty.
    #[must_use]
    pub const fn new(instant: UtcInstant, uncertainty: Uncertainty) -> Self {
        Self {
            instant,
            uncertainty,
        }
    }
}

/// One half-open, evidence-backed phase segment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseSegment {
    phase: Phase,
    boundary_start: PhaseBoundary,
    boundary_end: PhaseBoundary,
    evidence: Vec<EvidenceRef>,
    dataset_revisions: Vec<DatasetRevisionId>,
}

impl PhaseSegment {
    /// Validates and constructs a half-open phase segment.
    ///
    /// # Errors
    ///
    /// Returns [`PhaseSegmentError`] when the interval is not positive or when evidence
    /// or dataset revision attribution is empty.
    pub fn new(
        phase: Phase,
        boundary_start: PhaseBoundary,
        boundary_end: PhaseBoundary,
        evidence: Vec<EvidenceRef>,
        dataset_revisions: Vec<DatasetRevisionId>,
    ) -> Result<Self, PhaseSegmentError> {
        if boundary_end.instant <= boundary_start.instant {
            return Err(PhaseSegmentError::NonPositiveDuration);
        }
        if evidence.is_empty() {
            return Err(PhaseSegmentError::MissingEvidence);
        }
        if dataset_revisions.is_empty() {
            return Err(PhaseSegmentError::MissingDatasetRevision);
        }
        Ok(Self {
            phase,
            boundary_start,
            boundary_end,
            evidence,
            dataset_revisions,
        })
    }

    /// Returns the segment's shared phase.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// Returns the inclusive start boundary.
    #[must_use]
    pub const fn boundary_start(&self) -> &PhaseBoundary {
        &self.boundary_start
    }

    /// Returns the exclusive end boundary.
    #[must_use]
    pub const fn boundary_end(&self) -> &PhaseBoundary {
        &self.boundary_end
    }

    /// Returns the evidence references supporting this segment.
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRef] {
        &self.evidence
    }

    /// Returns the immutable dataset revisions that produced this segment.
    #[must_use]
    pub fn dataset_revisions(&self) -> &[DatasetRevisionId] {
        &self.dataset_revisions
    }

    /// Returns whether the half-open segment owns `instant`.
    #[must_use]
    pub fn contains(&self, instant: UtcInstant) -> bool {
        self.boundary_start.instant <= instant && instant < self.boundary_end.instant
    }
}

/// Validation failures for one phase segment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseSegmentError {
    /// The segment's end did not follow its start.
    NonPositiveDuration,
    /// The segment carried no evidence.
    MissingEvidence,
    /// The segment was not attributable to a dataset revision.
    MissingDatasetRevision,
}

impl fmt::Display for PhaseSegmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveDuration => {
                formatter.write_str("phase segment end must follow its start")
            }
            Self::MissingEvidence => formatter.write_str("phase segment requires evidence"),
            Self::MissingDatasetRevision => {
                formatter.write_str("phase segment requires a dataset revision")
            }
        }
    }
}

impl Error for PhaseSegmentError {}

/// A complete, gap-free, non-overlapping tiling of one finite coverage range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseTimeline {
    coverage: CoverageRange,
    segments: Vec<PhaseSegment>,
}

impl PhaseTimeline {
    /// Validates and constructs a timeline.
    ///
    /// # Errors
    ///
    /// Returns [`PhaseTimelineError`] when coverage is open-ended, the segment list is
    /// empty, the outer boundaries do not match coverage, or adjacent segments have a
    /// gap or overlap.
    pub fn new(
        coverage: CoverageRange,
        segments: Vec<PhaseSegment>,
    ) -> Result<Self, PhaseTimelineError> {
        let coverage_end = coverage
            .valid_until
            .ok_or(PhaseTimelineError::OpenEndedCoverageUnsupported)?;
        let first = segments.first().ok_or(PhaseTimelineError::Empty)?;
        if first.boundary_start.instant != coverage.valid_from {
            return Err(PhaseTimelineError::CoverageStartMismatch {
                expected_start: coverage.valid_from,
                actual_start: first.boundary_start.instant,
            });
        }
        for pair in segments.windows(2) {
            let expected_start = pair[0].boundary_end.instant;
            let actual_start = pair[1].boundary_start.instant;
            if expected_start != actual_start {
                return Err(PhaseTimelineError::Discontinuity {
                    expected_start,
                    actual_start,
                });
            }
        }
        let Some(last) = segments.last() else {
            return Err(PhaseTimelineError::Empty);
        };
        let last_end = last.boundary_end.instant;
        if last_end != coverage_end {
            return Err(PhaseTimelineError::CoverageEndMismatch {
                expected_end: coverage_end,
                actual_end: last_end,
            });
        }
        Ok(Self { coverage, segments })
    }

    /// Returns the timeline's declared coverage.
    #[must_use]
    pub const fn coverage(&self) -> CoverageRange {
        self.coverage
    }

    /// Returns the segment that owns `instant` under the half-open convention.
    #[must_use]
    pub fn segment_at(&self, instant: UtcInstant) -> Option<&PhaseSegment> {
        self.coverage
            .contains(instant)
            .then(|| {
                self.segments
                    .iter()
                    .find(|segment| segment.contains(instant))
            })
            .flatten()
    }

    /// Returns all segments in chronological order.
    #[must_use]
    pub fn segments(&self) -> &[PhaseSegment] {
        &self.segments
    }
}

/// Structural validation failures for a phase timeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhaseTimelineError {
    /// The declared coverage is open-ended and cannot be finitely tiled yet.
    OpenEndedCoverageUnsupported,
    /// No segment was supplied.
    Empty,
    /// The first segment did not begin at the coverage start.
    CoverageStartMismatch {
        /// The declared coverage start.
        expected_start: UtcInstant,
        /// The actual first segment start.
        actual_start: UtcInstant,
    },
    /// Two adjacent segments had a gap or overlap.
    Discontinuity {
        /// The start required by the preceding segment's end.
        expected_start: UtcInstant,
        /// The actual start of the following segment.
        actual_start: UtcInstant,
    },
    /// The last segment did not end at the coverage end.
    CoverageEndMismatch {
        /// The declared exclusive coverage end.
        expected_end: UtcInstant,
        /// The actual last segment end.
        actual_end: UtcInstant,
    },
}

impl fmt::Display for PhaseTimelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for PhaseTimelineError {}
