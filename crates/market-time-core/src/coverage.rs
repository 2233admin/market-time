//! What a dataset is willing to answer for.
//!
//! Coverage is declared per venue and enforced against that declaration (FR-018). Outside
//! it, the answer is an explicit unknown naming the boundary — never an extrapolated
//! schedule (FR-002).

use crate::ids::{DatasetRevisionId, VenueId};
use crate::instant::{Interval, UtcInstant};

/// The time range a venue's data is valid for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageRange {
    interval: Interval,
}

impl CoverageRange {
    /// Declares coverage over `interval`.
    #[must_use]
    pub fn new(interval: Interval) -> Self {
        Self { interval }
    }

    /// The declared interval, under `[start, end)` ownership.
    #[must_use]
    pub fn interval(&self) -> Interval {
        self.interval
    }

    /// First instant inside coverage.
    #[must_use]
    pub fn start(&self) -> UtcInstant {
        self.interval.start
    }

    /// First instant after coverage.
    #[must_use]
    pub fn end(&self) -> UtcInstant {
        self.interval.end
    }

    /// Whether `at` is inside declared coverage.
    #[must_use]
    pub fn contains(&self, at: UtcInstant) -> bool {
        self.interval.contains(at)
    }
}

/// The answer when a query falls outside declared coverage.
///
/// This is a variant of the ordinary answer, not an error and not `Phase::Closed`: an
/// unknown and a closed market are different values, not one value rendered two ways.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    /// The venue that was asked about.
    pub venue: VenueId,
    /// The instant that was asked about.
    pub queried_at: UtcInstant,
    /// The declaration that was violated.
    pub coverage: Option<CoverageRange>,
    /// The revisions consulted while establishing that this is a gap.
    pub dataset_revisions: Vec<DatasetRevisionId>,
}

impl CoverageGap {
    /// Which side of coverage the query fell on, in words a shell can render.
    #[must_use]
    pub fn describe(&self) -> String {
        match &self.coverage {
            None => format!("no coverage is declared for {}", self.venue),
            Some(range) if self.queried_at < range.start() => {
                format!("before {} coverage begins at {}", self.venue, range.start())
            }
            Some(range) => format!("after {} coverage ends at {}", self.venue, range.end()),
        }
    }
}
