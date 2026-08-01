//! Wall-clock time bound to a zone, and its conversion to an absolute instant.
//!
//! A venue publishes "09:30", not an instant. Turning that into a point on the timeline is
//! a question the zone's rules answer — and twice a year, in zones that observe daylight
//! saving, the answer is either "two instants" or "no instant at all". Neither is an error
//! and neither is a coin flip: [`CivilResolution`] carries the cases as values, so a caller
//! must decide what to do about them rather than receive a silent pick (FR-014).

use crate::ids::IanaZoneId;
use crate::instant::UtcInstant;
use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::{AmbiguousOffset, Offset, TimeZone};
use std::fmt;

/// A wall-clock reading together with the zone it is a reading in.
///
/// The zone is part of the value rather than context a caller is trusted to remember: a
/// civil datetime without its zone names no instant, and this type refuses to exist in
/// that state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilInstant {
    zone_id: IanaZoneId,
    zone: TimeZone,
    datetime: DateTime,
}

impl CivilInstant {
    /// Binds a civil datetime to a zone.
    ///
    /// # Errors
    ///
    /// Returns [`CivilError::UnknownZone`] when the zone is not in the bundled IANA
    /// database. The database is compiled in, so this is a question about the name, not
    /// about the host.
    pub fn new(zone_id: IanaZoneId, datetime: DateTime) -> Result<Self, CivilError> {
        let zone = TimeZone::get(zone_id.as_str()).map_err(|_| CivilError::UnknownZone {
            zone: zone_id.clone(),
        })?;
        Ok(Self {
            zone_id,
            zone,
            datetime,
        })
    }

    /// Parses `YYYY-MM-DDTHH:MM[:SS]` in the named zone.
    ///
    /// # Errors
    ///
    /// Returns [`CivilError::UnknownZone`] for an unknown zone and
    /// [`CivilError::Unparseable`] when the text is not a civil datetime. Note that a
    /// nonexistent wall-clock time parses successfully — it is a perfectly well-formed
    /// reading that no instant corresponds to, and saying so is [`CivilInstant::to_utc`]'s
    /// job, not the parser's.
    pub fn parse(zone_id: IanaZoneId, text: &str) -> Result<Self, CivilError> {
        let datetime: DateTime = text.parse().map_err(|_| CivilError::Unparseable {
            value: text.to_owned(),
        })?;
        Self::new(zone_id, datetime)
    }

    /// The zone this reading is in.
    #[must_use]
    pub fn zone(&self) -> &IanaZoneId {
        &self.zone_id
    }

    /// The wall-clock reading itself.
    #[must_use]
    pub fn datetime(&self) -> DateTime {
        self.datetime
    }

    /// Which instant this reading names, if any.
    ///
    /// Three outcomes, all of them ordinary: the usual one, the hour that happens twice
    /// when clocks go back, and the hour that never happens when they go forward.
    #[must_use]
    pub fn to_utc(&self) -> CivilResolution {
        match self.zone.to_ambiguous_timestamp(self.datetime).offset() {
            AmbiguousOffset::Unambiguous { offset } => {
                CivilResolution::Unambiguous(instant_of(offset, self.datetime))
            }
            AmbiguousOffset::Fold { before, after } => {
                let first = instant_of(before, self.datetime);
                let second = instant_of(after, self.datetime);
                let (earlier, later) = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                CivilResolution::Ambiguous { earlier, later }
            }
            AmbiguousOffset::Gap { before, after } => {
                let as_if_before_shift = instant_of(before, self.datetime);
                let as_if_after_shift = instant_of(after, self.datetime);
                CivilResolution::Nonexistent {
                    as_if_before_shift,
                    as_if_after_shift,
                }
            }
        }
    }
}

impl fmt::Display for CivilInstant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} in {}", self.datetime, self.zone_id)
    }
}

/// What a wall-clock reading resolves to.
///
/// There is deliberately no `unwrap_or_pick_one`. A caller that wants the earlier
/// occurrence of an ambiguous time must say so at its own call site, in its own words,
/// where the choice is visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilResolution {
    /// Exactly one instant bears this reading.
    Unambiguous(UtcInstant),
    /// Two instants bear it, because the clocks went back.
    Ambiguous {
        /// The first occurrence.
        earlier: UtcInstant,
        /// The second.
        later: UtcInstant,
    },
    /// No instant bears it, because the clocks jumped over it.
    Nonexistent {
        /// What it would name under the pre-transition offset.
        as_if_before_shift: UtcInstant,
        /// What it would name under the post-transition offset.
        as_if_after_shift: UtcInstant,
    },
}

impl CivilResolution {
    /// The instant, when there is exactly one.
    ///
    /// `None` for both daylight-saving cases: a caller that ignores them gets nothing
    /// rather than a guess.
    #[must_use]
    pub fn unambiguous(self) -> Option<UtcInstant> {
        match self {
            Self::Unambiguous(instant) => Some(instant),
            Self::Ambiguous { .. } | Self::Nonexistent { .. } => None,
        }
    }

    /// Whether the reading names exactly one instant.
    #[must_use]
    pub fn is_unambiguous(self) -> bool {
        matches!(self, Self::Unambiguous(_))
    }
}

impl fmt::Display for CivilResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unambiguous(instant) => write!(f, "{instant}"),
            Self::Ambiguous { earlier, later } => write!(
                f,
                "occurs twice: {earlier} and {later}; which one is meant is the caller's to say"
            ),
            Self::Nonexistent {
                as_if_before_shift,
                as_if_after_shift,
            } => write!(
                f,
                "does not exist: the clocks jumped from {as_if_after_shift} to {as_if_before_shift}"
            ),
        }
    }
}

/// Why a civil reading could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CivilError {
    /// The zone is not in the bundled IANA database.
    UnknownZone {
        /// The zone that was named.
        zone: IanaZoneId,
    },
    /// The text is not a civil datetime.
    Unparseable {
        /// The text that was given.
        value: String,
    },
}

impl fmt::Display for CivilError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownZone { zone } => write!(f, "unknown IANA zone {zone}"),
            Self::Unparseable { value } => {
                write!(
                    f,
                    "{value:?} is not a civil datetime (YYYY-MM-DDTHH:MM[:SS])"
                )
            }
        }
    }
}

impl std::error::Error for CivilError {}

pub(crate) fn instant_of(offset: Offset, datetime: DateTime) -> UtcInstant {
    offset.to_timestamp(datetime).map_or_else(
        |_| UtcInstant::from_nanos_since_unix_epoch(0),
        |timestamp| UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond()),
    )
}

/// Returns the civil datetime for an absolute instant, saturating at Jiff's bounds.
///
/// An out-of-range instant is outside every usable coverage declaration, but callers
/// still need a stable civil date to construct that explicit unknown answer.
pub(crate) fn datetime_at(zone: &TimeZone, at: UtcInstant) -> DateTime {
    let timestamp =
        Timestamp::from_nanosecond(at.as_nanos_since_unix_epoch()).unwrap_or_else(|_| {
            if at.as_nanos_since_unix_epoch() < 0 {
                Timestamp::MIN
            } else {
                Timestamp::MAX
            }
        });
    zone.to_datetime(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn new_york(text: &str) -> CivilInstant {
        CivilInstant::parse(
            IanaZoneId::new("America/New_York").expect("valid zone"),
            text,
        )
        .expect("valid civil reading")
    }

    #[test]
    fn an_ordinary_reading_names_one_instant() {
        let resolution = new_york("2026-07-15T09:30:00").to_utc();
        assert!(resolution.is_unambiguous());
        assert!(resolution.unambiguous().is_some());
    }

    #[test]
    fn the_hour_that_happens_twice_is_reported_as_two() {
        // 2026-11-01: New York clocks go back at 02:00 local, so 01:30 occurs twice.
        let resolution = new_york("2026-11-01T01:30:00").to_utc();
        match resolution {
            CivilResolution::Ambiguous { earlier, later } => {
                assert!(earlier < later);
                assert_eq!(
                    earlier.saturating_nanos_until(later),
                    3_600 * crate::instant::NANOS_PER_SECOND,
                    "the two occurrences are an hour apart"
                );
            }
            other => panic!("expected an ambiguous reading, got {other}"),
        }
        assert_eq!(resolution.unambiguous(), None, "no silent pick");
    }

    #[test]
    fn the_hour_that_never_happens_is_reported_as_none_of_them() {
        // 2026-03-08: New York clocks jump 02:00 -> 03:00, so 02:30 does not exist.
        let resolution = new_york("2026-03-08T02:30:00").to_utc();
        assert!(
            matches!(resolution, CivilResolution::Nonexistent { .. }),
            "got {resolution}"
        );
        assert_eq!(resolution.unambiguous(), None, "no invented instant");
    }

    #[test]
    fn a_zone_free_reading_cannot_be_built() {
        let zone = IanaZoneId::new("Mars/Olympus_Mons").expect("non-empty");
        assert_eq!(
            CivilInstant::new(zone.clone(), date(2026, 1, 1).at(0, 0, 0, 0)),
            Err(CivilError::UnknownZone { zone })
        );
    }

    #[test]
    fn a_nonexistent_reading_still_parses() {
        // Parsing is about the text; existence is about the zone. Conflating them would
        // make the error message wrong.
        assert!(
            CivilInstant::parse(
                IanaZoneId::new("America/New_York").expect("valid zone"),
                "2026-03-08T02:30:00"
            )
            .is_ok()
        );
    }
}
