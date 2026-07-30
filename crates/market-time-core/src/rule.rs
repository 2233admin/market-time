//! Rules: what a venue's schedule says, and for which dates it says it.
//!
//! Rules store phase boundaries as **civil time-of-day tied to the venue's home zone**,
//! never as pre-computed UTC. A boundary baked to UTC once cannot re-derive the right
//! offset the next time daylight saving shifts — that reintroduces the hardcoded-offset
//! failure Principle II names, just moved from code into data. The civil-to-UTC
//! conversion happens at query time, which is also what makes nonexistent and ambiguous
//! local times answerable at all (FR-014).

use crate::evidence::{DerivationNote, EvidenceRef};
use crate::ids::DatasetRevisionId;
use crate::phase::Phase;
use crate::uncertainty::Uncertainty;
use jiff::civil::{Date, Time, Weekday};
use std::fmt;

/// An inclusive range of venue-local calendar dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    /// First date in the range.
    pub start: Date,
    /// Last date in the range, inclusive.
    pub end: Date,
}

impl DateRange {
    /// Builds a date range.
    ///
    /// # Errors
    ///
    /// Returns [`RuleError::DateRangeReversed`] when `end` precedes `start`.
    pub fn new(start: Date, end: Date) -> Result<Self, RuleError> {
        if end < start {
            return Err(RuleError::DateRangeReversed { start, end });
        }
        Ok(Self { start, end })
    }

    /// Whether `date` falls in the range.
    #[must_use]
    pub fn contains(self, date: Date) -> bool {
        date >= self.start && date <= self.end
    }
}

/// A venue-local day, described as the phases it is divided into.
///
/// The schedule is stored as ordered change points rather than as spans with both ends,
/// which is what makes the tiling invariant (FR-008) structural: the first change point
/// is midnight, each phase runs until the next change point, and the last runs to the
/// next midnight. A day with a gap in it cannot be expressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilDaySchedule {
    change_points: Vec<(Time, Phase)>,
}

impl CivilDaySchedule {
    /// Builds a day schedule from its change points.
    ///
    /// # Errors
    ///
    /// Returns [`RuleError::ScheduleDoesNotStartAtMidnight`] when the first change point
    /// is not `00:00`, and [`RuleError::ScheduleNotOrdered`] when the change points are
    /// not strictly increasing. Both would break the tiling invariant.
    pub fn new(change_points: Vec<(Time, Phase)>) -> Result<Self, RuleError> {
        let Some(&(first, _)) = change_points.first() else {
            return Err(RuleError::ScheduleDoesNotStartAtMidnight);
        };
        if first != Time::midnight() {
            return Err(RuleError::ScheduleDoesNotStartAtMidnight);
        }
        if change_points.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(RuleError::ScheduleNotOrdered);
        }
        Ok(Self { change_points })
    }

    /// A day that is one phase from midnight to midnight.
    ///
    /// # Errors
    ///
    /// Cannot fail today; the signature matches [`CivilDaySchedule::new`] so callers stay
    /// uniform.
    pub fn all_day(phase: Phase) -> Result<Self, RuleError> {
        Self::new(vec![(Time::midnight(), phase)])
    }

    /// The change points, in order, starting at midnight.
    #[must_use]
    pub fn change_points(&self) -> &[(Time, Phase)] {
        &self.change_points
    }
}

/// What kind of statement a rule makes.
///
/// Precedence is part of the vocabulary rather than a caller's choice: holidays and
/// shortened sessions win over the weekly pattern (FR-015).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleKind {
    /// The ordinary weekly pattern, applying on the listed weekdays.
    WeeklyPattern {
        /// Weekdays this pattern applies on.
        weekdays: Vec<Weekday>,
    },
    /// A day the venue does not trade its normal schedule.
    Holiday {
        /// The holiday, as the venue names it.
        name: String,
    },
    /// A day the venue trades a shortened schedule.
    ShortenedSession {
        /// The session, as the venue names it.
        name: String,
    },
    /// A schedule change the venue announced, effective from a date.
    AnnouncedChange {
        /// What the venue announced.
        note: String,
    },
}

impl RuleKind {
    /// Higher wins when more than one rule applies to a date.
    #[must_use]
    pub fn precedence(&self) -> u8 {
        match self {
            Self::Holiday { .. } | Self::ShortenedSession { .. } => 30,
            Self::AnnouncedChange { .. } => 20,
            Self::WeeklyPattern { .. } => 10,
        }
    }

    /// Whether this rule can apply on `weekday`.
    #[must_use]
    pub fn applies_on_weekday(&self, weekday: Weekday) -> bool {
        match self {
            Self::WeeklyPattern { weekdays } => weekdays.contains(&weekday),
            _ => true,
        }
    }

    /// A short label for rendering.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::WeeklyPattern { .. } => "weekly pattern",
            Self::Holiday { name } | Self::ShortenedSession { name } => name,
            Self::AnnouncedChange { note } => note,
        }
    }
}

/// One statement about a venue's schedule, with the evidence for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// What kind of statement this is.
    pub kind: RuleKind,
    /// The venue-local day this rule describes.
    pub schedule: CivilDaySchedule,
    /// The dates the rule is in force.
    pub applies: DateRange,
    /// How precisely the source publishes these boundaries (FR-011).
    pub boundary_uncertainty: Uncertainty,
    /// Where the rule came from (FR-009).
    pub evidence: EvidenceRef,
    /// Present when the rule is our reading rather than the venue's wording (FR-010).
    pub derived: Option<DerivationNote>,
    /// The dataset revision this rule belongs to (FR-016).
    pub revision: DatasetRevisionId,
}

impl Rule {
    /// Whether this rule applies on `date`.
    #[must_use]
    pub fn applies_on(&self, date: Date) -> bool {
        self.applies.contains(date) && self.kind.applies_on_weekday(date.weekday())
    }

    /// Whether the rule is derived rather than published as such.
    #[must_use]
    pub fn is_derived(&self) -> bool {
        self.derived.is_some()
    }
}

/// Why a rule or a schedule could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleError {
    /// A day schedule did not start at midnight.
    ScheduleDoesNotStartAtMidnight,
    /// A day schedule's change points were not strictly increasing.
    ScheduleNotOrdered,
    /// A date range ended before it started.
    DateRangeReversed {
        /// The proposed start.
        start: Date,
        /// The proposed end.
        end: Date,
    },
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScheduleDoesNotStartAtMidnight => {
                f.write_str("a day schedule must start at 00:00, or it leaves a gap")
            }
            Self::ScheduleNotOrdered => {
                f.write_str("day schedule change points must strictly increase")
            }
            Self::DateRangeReversed { start, end } => {
                write!(f, "date range ends at {end}, before it starts at {start}")
            }
        }
    }
}

impl std::error::Error for RuleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_with_a_gap_cannot_be_expressed() {
        let not_midnight = vec![(Time::constant(9, 30, 0, 0), Phase::ContinuousTrading)];
        assert_eq!(
            CivilDaySchedule::new(not_midnight),
            Err(RuleError::ScheduleDoesNotStartAtMidnight)
        );
    }

    #[test]
    fn change_points_must_increase() {
        let backwards = vec![
            (Time::midnight(), Phase::Closed),
            (Time::constant(15, 0, 0, 0), Phase::PostClose),
            (Time::constant(9, 30, 0, 0), Phase::ContinuousTrading),
        ];
        assert_eq!(
            CivilDaySchedule::new(backwards),
            Err(RuleError::ScheduleNotOrdered)
        );
    }

    #[test]
    fn holidays_outrank_the_weekly_pattern() {
        let holiday = RuleKind::Holiday {
            name: "New Year".to_owned(),
        };
        let weekly = RuleKind::WeeklyPattern {
            weekdays: vec![Weekday::Monday],
        };
        assert!(holiday.precedence() > weekly.precedence());
    }
}
