//! Scheduled occurrences that are not states.
//!
//! A funding settlement happens at a scheduled instant, but the venue is not "in" it the
//! way it is in continuous trading. Events sit on top of the phase in force and never
//! replace, split, or extend it (FR-007, FR-008).
//!
//! [`EventKind`] is a separate closed vocabulary from [`crate::phase::Phase`] precisely so
//! the two can never be substituted for one another.

use crate::evidence::EvidenceRef;
use crate::instant::UtcInstant;
use crate::uncertainty::Uncertainty;
use std::fmt;

/// The kind of a scheduled occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum EventKind {
    /// A perpetual-futures funding settlement.
    FundingSettlement,
}

impl EventKind {
    /// A stable, lowercase identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FundingSettlement => "funding_settlement",
        }
    }

    /// Parses the identifier produced by [`EventKind::as_str`].
    #[must_use]
    pub fn from_str_exact(value: &str) -> Option<Self> {
        match value {
            "funding_settlement" => Some(Self::FundingSettlement),
            _ => None,
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A rule that produces occurrences, stated as venue-local times of day.
///
/// Like phase rules, event rules are stored in civil time and converted at query time, so
/// a daylight-saving shift moves them correctly instead of silently keeping a stale
/// offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRule {
    /// What kind of occurrence this rule produces.
    pub kind: EventKind,
    /// The venue-local times of day it occurs at.
    pub times: Vec<jiff::civil::Time>,
    /// The dates the rule is in force.
    pub applies: crate::rule::DateRange,
    /// How precisely the occurrence instant is known.
    ///
    /// Where the venue publishes its own deviation, this carries that bound unchanged —
    /// it is handed to us, not estimated (FR-011a).
    pub uncertainty: Uncertainty,
    /// Where the rule came from.
    pub evidence: EvidenceRef,
    /// The dataset revision this rule belongs to.
    pub revision: crate::ids::DatasetRevisionId,
}

/// One occurrence of a scheduled event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventOccurrence {
    /// What kind of occurrence this is.
    pub kind: EventKind,
    /// When it is scheduled.
    pub instant: UtcInstant,
    /// How precisely that instant is known. A venue-published deviation lives here.
    pub uncertainty: Uncertainty,
    /// The evidence for the rule that produced it.
    pub evidence: Vec<EvidenceRef>,
}
