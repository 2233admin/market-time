//! The only instant type the core accepts.
//!
//! [`UtcInstant`] is UTC by construction, so FR-013's "explicitly declared time scale" is
//! satisfied by the type rather than by a runtime tag the core would have to check.
//! Input on another scale is tagged and converted, leap-second-aware, in
//! `market-time-scales` before it ever reaches here.
//!
//! There is deliberately **no `now()`**. Every constructor takes the value from the
//! caller, which is what makes golden vectors replay deterministically (Principle IV).

use std::fmt;

/// Nanoseconds in one second.
pub const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// An instant on the UTC timeline, at nanosecond resolution.
///
/// Ordering is the timeline order. Equality is exact: two `UtcInstant`s are equal only if
/// they name the same nanosecond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcInstant {
    nanos_since_unix_epoch: i128,
}

impl UtcInstant {
    /// Builds an instant from nanoseconds since the Unix epoch.
    #[must_use]
    pub const fn from_nanos_since_unix_epoch(nanos: i128) -> Self {
        Self {
            nanos_since_unix_epoch: nanos,
        }
    }

    /// Builds an instant from whole seconds since the Unix epoch.
    #[must_use]
    pub const fn from_seconds_since_unix_epoch(seconds: i64) -> Self {
        Self::from_nanos_since_unix_epoch(seconds as i128 * NANOS_PER_SECOND)
    }

    /// Nanoseconds since the Unix epoch.
    #[must_use]
    pub const fn as_nanos_since_unix_epoch(self) -> i128 {
        self.nanos_since_unix_epoch
    }

    /// This instant shifted by `nanos`, saturating rather than wrapping.
    #[must_use]
    pub const fn saturating_add_nanos(self, nanos: i128) -> Self {
        Self::from_nanos_since_unix_epoch(self.nanos_since_unix_epoch.saturating_add(nanos))
    }

    /// Nanoseconds from `self` to `other`, saturating rather than wrapping.
    #[must_use]
    pub const fn saturating_nanos_until(self, other: Self) -> i128 {
        other
            .nanos_since_unix_epoch
            .saturating_sub(self.nanos_since_unix_epoch)
    }
}

impl fmt::Display for UtcInstant {
    /// RFC 3339 in UTC, falling back to raw nanoseconds outside the representable
    /// calendar.
    ///
    /// Formatting is presentation, and the core normally leaves presentation to shells —
    /// but an instant that appears inside an error or a coverage description is read by a
    /// person, and `1798693200000000000ns` is not a fact anyone can check at a glance.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match jiff::Timestamp::from_nanosecond(self.nanos_since_unix_epoch) {
            Ok(timestamp) => write!(f, "{timestamp}"),
            Err(_) => write!(f, "{}ns", self.nanos_since_unix_epoch),
        }
    }
}

/// A half-open interval `[start, end)` on the UTC timeline.
///
/// Half-open is the repository-wide boundary ownership convention: the instant a phase
/// ends is owned by the next phase, so every instant belongs to exactly one phase
/// (FR-004). No source document settles this; it is ours to decide, and it is decided
/// here rather than per call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    /// First instant inside the interval.
    pub start: UtcInstant,
    /// First instant *after* the interval.
    pub end: UtcInstant,
}

impl Interval {
    /// Builds an interval.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::EndBeforeStart`] when `end` precedes `start`, and
    /// [`IntervalError::Empty`] when the two are equal — an empty interval has no
    /// meaningful phase answer, so it is rejected at construction rather than producing
    /// an empty timeline later.
    pub fn new(start: UtcInstant, end: UtcInstant) -> Result<Self, IntervalError> {
        match start.cmp(&end) {
            std::cmp::Ordering::Less => Ok(Self { start, end }),
            std::cmp::Ordering::Equal => Err(IntervalError::Empty { at: start }),
            std::cmp::Ordering::Greater => Err(IntervalError::EndBeforeStart { start, end }),
        }
    }

    /// Whether `at` falls inside the interval, under `[start, end)` ownership.
    #[must_use]
    pub fn contains(self, at: UtcInstant) -> bool {
        at >= self.start && at < self.end
    }

    /// The overlap between two intervals, or `None` when they do not overlap.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        (start < end).then_some(Self { start, end })
    }
}

/// Why an interval could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntervalError {
    /// `end` precedes `start`.
    EndBeforeStart {
        /// The proposed start.
        start: UtcInstant,
        /// The proposed end.
        end: UtcInstant,
    },
    /// `start` and `end` are the same instant.
    Empty {
        /// The instant both bounds named.
        at: UtcInstant,
    },
}

impl fmt::Display for IntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndBeforeStart { start, end } => {
                write!(f, "interval end {end} precedes start {start}")
            }
            Self::Empty { at } => write!(f, "interval is empty at {at}"),
        }
    }
}

impl std::error::Error for IntervalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_instant_belongs_to_the_later_interval() {
        let a = Interval::new(
            UtcInstant::from_seconds_since_unix_epoch(0),
            UtcInstant::from_seconds_since_unix_epoch(10),
        )
        .expect("valid interval");
        let b = Interval::new(
            UtcInstant::from_seconds_since_unix_epoch(10),
            UtcInstant::from_seconds_since_unix_epoch(20),
        )
        .expect("valid interval");

        let boundary = UtcInstant::from_seconds_since_unix_epoch(10);
        assert!(
            !a.contains(boundary),
            "end instant is not owned by the earlier interval"
        );
        assert!(
            b.contains(boundary),
            "end instant is owned by the later interval"
        );
    }

    #[test]
    fn empty_and_reversed_intervals_are_rejected() {
        let t0 = UtcInstant::from_seconds_since_unix_epoch(5);
        let t1 = UtcInstant::from_seconds_since_unix_epoch(6);
        assert!(matches!(
            Interval::new(t0, t0),
            Err(IntervalError::Empty { .. })
        ));
        assert!(matches!(
            Interval::new(t1, t0),
            Err(IntervalError::EndBeforeStart { .. })
        ));
    }
}
