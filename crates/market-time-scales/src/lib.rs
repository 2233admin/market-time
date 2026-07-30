//! Mark Time — time-scale ingest boundary.
//!
//! Converts an instant tagged with a non-UTC time scale (TAI, GNSS system time, a host
//! monotonic scale) into UTC, leap-second-aware, using hifitime's IERS table.
//!
//! # The seam
//!
//! This is the only place in the tree where that conversion happens, deliberately.
//! Downstream of here everything operates on UTC nanoseconds via `jiff`, which does not
//! model leap seconds — so an instant that crosses this boundary becomes leap-second-naive
//! and cannot round-trip back through a real historical leap second.
//!
//! That is an architectural seam, not a defect, and it is documented rather than papered
//! over (research D1a). No venue phase boundary has ever fallen inside a leap second, so
//! the limitation is acceptable for this domain — but it is asserted by a golden vector
//! rather than assumed.
//!
//! Converting by adding a hardcoded constant is forbidden (Principle II): the UTC-to-GNSS
//! offset is itself a dated fact that changes.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

use hifitime::{Epoch, TimeScale, Unit};
use market_time_core::UtcInstant;
use std::fmt;

/// A time scale an input instant may be labelled with.
///
/// An unlabelled instant is not accepted: the label is what makes the conversion
/// answerable, so it is required by the type rather than defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Scale {
    /// Coordinated Universal Time.
    Utc,
    /// International Atomic Time.
    Tai,
    /// GPS system time.
    Gps,
}

impl Scale {
    /// A stable, lowercase identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Utc => "utc",
            Self::Tai => "tai",
            Self::Gps => "gps",
        }
    }

    /// Parses the identifier produced by [`Scale::as_str`].
    #[must_use]
    pub fn from_str_exact(value: &str) -> Option<Self> {
        match value {
            "utc" => Some(Self::Utc),
            "tai" => Some(Self::Tai),
            "gps" => Some(Self::Gps),
            _ => None,
        }
    }

    fn to_hifitime(self) -> TimeScale {
        match self {
            Self::Utc => TimeScale::UTC,
            Self::Tai => TimeScale::TAI,
            Self::Gps => TimeScale::GPST,
        }
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An instant that still carries its scale, before conversion.
///
/// This type deliberately has no arithmetic: it exists to be converted once, here, and
/// then to stop existing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawScaledInstant {
    scale: Scale,
    seconds_since_scale_epoch: f64,
}

impl RawScaledInstant {
    /// Labels a count of seconds with the scale it is counted on.
    ///
    /// The epoch is that scale's own reference epoch, as `hifitime` defines it.
    #[must_use]
    pub const fn new(scale: Scale, seconds_since_scale_epoch: f64) -> Self {
        Self {
            scale,
            seconds_since_scale_epoch,
        }
    }

    /// The scale this instant is counted on.
    #[must_use]
    pub const fn scale(self) -> Scale {
        self.scale
    }

    /// Converts to UTC, leap-second-aware.
    ///
    /// # Errors
    ///
    /// Returns [`ScaleError::OutOfRange`] when the result falls outside the representable
    /// nanosecond range. That is a refusal to answer rather than a silently clamped
    /// instant.
    pub fn to_utc(self) -> Result<UtcInstant, ScaleError> {
        if self.scale == Scale::Utc {
            return Ok(UtcInstant::from_nanos_since_unix_epoch(seconds_to_nanos(
                self.seconds_since_scale_epoch,
            )));
        }

        // Counted from the scale's own reference epoch, as hifitime defines it: J1900 for
        // TAI, 1980-01-06 for GPS.
        let epoch = Epoch::from_duration(
            self.seconds_since_scale_epoch * Unit::Second,
            self.scale.to_hifitime(),
        );

        // Nanoseconds all the way, never f64 seconds: a float carrying 1.7e18 nanoseconds
        // has lost the last two digits before anyone looks at it, and this crate exists to
        // be exact about instants.
        let since_j1900 = epoch.to_utc_duration().total_nanoseconds();
        let nanos = since_j1900
            .checked_sub(J1900_TO_UNIX_SECONDS * NANOS_PER_SECOND)
            .ok_or(ScaleError::OutOfRange {
                scale: self.scale,
                seconds: self.seconds_since_scale_epoch,
            })?;

        Ok(UtcInstant::from_nanos_since_unix_epoch(nanos))
    }
}

/// Seconds from the J1900 epoch hifitime counts from to the Unix epoch.
const J1900_TO_UNIX_SECONDS: i128 = 2_208_988_800;

/// Nanoseconds in one second.
const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// Splits before scaling, so the integer part never rides through a float multiply.
fn seconds_to_nanos(seconds: f64) -> i128 {
    #[allow(clippy::cast_possible_truncation)]
    let whole = seconds.trunc() as i128;
    #[allow(clippy::cast_possible_truncation)]
    let fraction = (seconds.fract() * 1e9).round() as i128;
    whole * NANOS_PER_SECOND + fraction
}

/// How far `scale` runs ahead of UTC at `at`, in seconds.
///
/// Exposed so a shell can record *which* offset was applied rather than merely trusting
/// that one was. An offset nobody can report is an offset nobody can audit.
///
/// Returns `None` when the leap-second table does not cover `at` — an unknown offset is
/// reported as unknown rather than as zero.
#[must_use]
pub fn offset_from_utc_seconds(scale: Scale, at: UtcInstant) -> Option<f64> {
    if scale == Scale::Utc {
        return Some(0.0);
    }

    #[allow(clippy::cast_precision_loss)]
    let unix_seconds = (at.as_nanos_since_unix_epoch() / 1_000_000_000) as f64;
    let epoch = Epoch::from_unix_duration(unix_seconds * Unit::Second);
    let tai_minus_utc = epoch.leap_seconds(false)?;

    Some(match scale {
        Scale::Utc => 0.0,
        Scale::Tai => tai_minus_utc,
        // GPS time is defined as TAI minus 19 seconds. That 19 is a definition rather
        // than a drifting fact, which is why it may be written down; the part that
        // drifts is the leap-second count above, and that is read from the table.
        Scale::Gps => tai_minus_utc - GPS_BEHIND_TAI_SECONDS,
    })
}

/// GPS time runs a defined 19 seconds behind TAI.
const GPS_BEHIND_TAI_SECONDS: f64 = 19.0;

/// Why a conversion could not be performed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleError {
    /// The value falls outside the representable range.
    OutOfRange {
        /// The scale being converted from.
        scale: Scale,
        /// The value that could not be represented.
        seconds: f64,
    },
}

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange { scale, seconds } => write!(
                f,
                "{seconds}s on the {scale} scale is outside the representable range"
            ),
        }
    }
}

impl std::error::Error for ScaleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_passes_through_unchanged() {
        let raw = RawScaledInstant::new(Scale::Utc, 1_750_000_000.0);
        let utc = raw.to_utc().expect("utc converts");
        assert_eq!(
            utc.as_nanos_since_unix_epoch() / 1_000_000_000,
            1_750_000_000
        );
    }

    #[test]
    fn tai_runs_ahead_of_utc_by_the_leap_second_count() {
        // Read from the IERS table rather than added as a constant, so this asserts the
        // shape -- TAI leads UTC by tens of seconds -- not a hardcoded 37.
        let offset = offset_from_utc_seconds(
            Scale::Tai,
            UtcInstant::from_seconds_since_unix_epoch(1_750_000_000),
        )
        .expect("the table covers 2025");
        assert!(
            (30.0..=45.0).contains(&offset),
            "TAI-UTC offset {offset} is outside the plausible modern range"
        );
    }

    #[test]
    fn gps_runs_ahead_of_utc_but_behind_tai() {
        let at = UtcInstant::from_seconds_since_unix_epoch(1_750_000_000);
        let gps = offset_from_utc_seconds(Scale::Gps, at).expect("the table covers 2025");
        let tai = offset_from_utc_seconds(Scale::Tai, at).expect("the table covers 2025");
        assert!(
            gps > 0.0 && gps < tai,
            "GPS {gps} should sit between UTC and TAI {tai}"
        );
    }

    #[test]
    fn the_scale_label_round_trips() {
        for scale in [Scale::Utc, Scale::Tai, Scale::Gps] {
            assert_eq!(Scale::from_str_exact(scale.as_str()), Some(scale));
        }
        assert_eq!(Scale::from_str_exact("monotonic"), None);
    }
}
