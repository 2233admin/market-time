//! How precisely an answer is known.
//!
//! Nanosecond representation is an arithmetic guarantee, not an accuracy claim. Every
//! boundary and every answer carries an [`Uncertainty`] that says what the evidence
//! actually supports (FR-011), and it can never be narrower than what the venue itself
//! published (FR-011a).

use crate::instant::{NANOS_PER_SECOND, UtcInstant};
use std::fmt;

/// The precision an answer or a boundary is known to.
///
/// The variants are the cases the launch set actually produces. `Exact` means exact to
/// the representation — it is only honest for a boundary that is defined rather than
/// observed, such as the boundary of a coverage declaration we authored.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Uncertainty {
    /// Exact to the nanosecond, because the value is defined rather than measured.
    Exact,
    /// The source publishes to a granularity, e.g. whole minutes.
    ///
    /// The stated uncertainty is the publication granularity: a boundary published as
    /// `09:30` is known to the minute, whatever the internal representation.
    PublishedGranularity {
        /// Width of the publication grid, in nanoseconds.
        nanos: i128,
    },
    /// The venue published its own deviation bound, and we carry it unchanged (FR-011a).
    PublishedBound {
        /// The venue-published bound, in nanoseconds.
        nanos: i128,
        /// What the venue said, so the bound is never mistaken for our estimate.
        published_as: String,
    },
    /// The boundary is the scheduled start of a process, not an instant (FR-011b).
    ///
    /// Deliberately carries no number: the spread is real and its width is not published,
    /// and inventing one would be exactly the false precision Principle II forbids.
    ProcessStart {
        /// What the process is, in the venue's own terms.
        process: String,
    },
    /// The local wall-clock time this boundary is written in occurs twice (FR-014).
    AmbiguousLocalTime {
        /// The earlier of the two instants that bear this local time.
        earlier: UtcInstant,
        /// The later of the two.
        later: UtcInstant,
    },
    /// The local wall-clock time this boundary is written in does not exist (FR-014).
    ///
    /// Both candidates are carried rather than one being silently chosen: the clocks
    /// jumped over this local time, so neither reading is the answer, and which one a
    /// consumer wants is not ours to decide for them.
    NonexistentLocalTime {
        /// The instant this local time would name under the pre-transition offset.
        as_if_before_shift: UtcInstant,
        /// The instant it would name under the post-transition offset.
        as_if_after_shift: UtcInstant,
    },
}

impl Uncertainty {
    /// A published granularity of whole seconds.
    #[must_use]
    pub const fn seconds(count: i128) -> Self {
        Self::PublishedGranularity {
            nanos: count * NANOS_PER_SECOND,
        }
    }

    /// A published granularity of whole minutes.
    #[must_use]
    pub const fn minutes(count: i128) -> Self {
        Self::seconds(count * 60)
    }

    /// The width of this uncertainty in nanoseconds, when it has one.
    ///
    /// `None` means the answer is known to be imprecise but its width is not published —
    /// a process start, or a local time that is ambiguous or nonexistent. `None` is not
    /// "no uncertainty"; treat it as wider than any number, never as zero.
    #[must_use]
    pub fn bound_nanos(&self) -> Option<i128> {
        match self {
            Self::Exact => Some(0),
            Self::PublishedGranularity { nanos } | Self::PublishedBound { nanos, .. } => {
                Some(*nanos)
            }
            Self::ProcessStart { .. }
            | Self::AmbiguousLocalTime { .. }
            | Self::NonexistentLocalTime { .. } => None,
        }
    }

    /// Whether this uncertainty claims exactness.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Exact)
    }

    /// The wider of two uncertainties.
    ///
    /// An unbounded case (process start, ambiguous or nonexistent local time) always
    /// wins over a numeric one: an unpublished spread is not narrower than a published
    /// granularity just because it carries no number. This is the combinator FR-011a and
    /// the overlap rules rely on — a combined answer is never more precise than its least
    /// precise input.
    #[must_use]
    pub fn widest(self, other: Self) -> Self {
        match (self.bound_nanos(), other.bound_nanos()) {
            (Some(a), Some(b)) => {
                if a >= b {
                    self
                } else {
                    other
                }
            }
            (Some(_), None) => other,
            (None, _) => self,
        }
    }

    /// Whether `self` is at least as wide as `floor`.
    ///
    /// Used to enforce FR-011a: a stated uncertainty may never be narrower than the bound
    /// the venue published.
    #[must_use]
    pub fn is_at_least_as_wide_as(&self, floor: &Self) -> bool {
        match (self.bound_nanos(), floor.bound_nanos()) {
            (Some(a), Some(b)) => a >= b,
            (None, _) => true,
            (Some(_), None) => false,
        }
    }
}

impl fmt::Display for Uncertainty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact => f.write_str("exact"),
            Self::PublishedGranularity { nanos } => {
                write!(f, "published to {}", humanize(*nanos))
            }
            Self::PublishedBound {
                nanos,
                published_as,
            } => write!(
                f,
                "venue-published bound {} ({published_as})",
                humanize(*nanos)
            ),
            Self::ProcessStart { process } => {
                write!(f, "scheduled start of {process}; spread not published")
            }
            Self::AmbiguousLocalTime { earlier, later } => {
                write!(f, "local time occurs twice ({earlier} and {later})")
            }
            Self::NonexistentLocalTime {
                as_if_before_shift,
                as_if_after_shift,
            } => write!(
                f,
                "local time does not exist (candidates {as_if_before_shift} and {as_if_after_shift})"
            ),
        }
    }
}

fn humanize(nanos: i128) -> String {
    if nanos == 0 {
        return "0s".to_owned();
    }
    let seconds = nanos / NANOS_PER_SECOND;
    if seconds == 0 {
        return format!("{nanos}ns");
    }
    if seconds % 60 == 0 {
        format!("{}min", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unpublished_spread_is_wider_than_any_number() {
        let process = Uncertainty::ProcessStart {
            process: "opening process".to_owned(),
        };
        let minute = Uncertainty::minutes(1);

        assert_eq!(process.clone().widest(minute.clone()), process);
        assert!(process.is_at_least_as_wide_as(&minute));
        assert!(!minute.is_at_least_as_wide_as(&process));
    }

    #[test]
    fn a_published_bound_is_a_floor() {
        let venue_bound = Uncertainty::PublishedBound {
            nanos: 15 * NANOS_PER_SECOND,
            published_as: "±15 seconds".to_owned(),
        };
        assert!(!Uncertainty::seconds(1).is_at_least_as_wide_as(&venue_bound));
        assert!(Uncertainty::seconds(30).is_at_least_as_wide_as(&venue_bound));
    }

    #[test]
    fn exact_is_zero_width_not_unknown_width() {
        assert_eq!(Uncertainty::Exact.bound_nanos(), Some(0));
    }
}
