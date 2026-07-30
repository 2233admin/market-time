//! Declared half-open coverage ranges.

use std::{error::Error, fmt};

use crate::UtcInstant;

/// A declared validity range `[valid_from, valid_until)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverageRange {
    /// The inclusive start of coverage.
    pub valid_from: UtcInstant,
    /// The exclusive end of coverage, or `None` for an open-ended declaration.
    pub valid_until: Option<UtcInstant>,
}

impl CoverageRange {
    /// Constructs a finite half-open range.
    ///
    /// # Errors
    ///
    /// Returns [`CoverageError::EndBeforeStart`] when `valid_until` precedes
    /// `valid_from`. Equal endpoints are allowed to represent zero coverage.
    pub fn closed_open(
        valid_from: UtcInstant,
        valid_until: UtcInstant,
    ) -> Result<Self, CoverageError> {
        if valid_until < valid_from {
            return Err(CoverageError::EndBeforeStart);
        }
        Ok(Self {
            valid_from,
            valid_until: Some(valid_until),
        })
    }

    /// Constructs an open-ended range beginning at `valid_from`.
    #[must_use]
    pub const fn open_ended(valid_from: UtcInstant) -> Self {
        Self {
            valid_from,
            valid_until: None,
        }
    }

    /// Returns whether `instant` is within this half-open range.
    #[must_use]
    pub fn contains(self, instant: UtcInstant) -> bool {
        instant >= self.valid_from
            && self
                .valid_until
                .is_none_or(|valid_until| instant < valid_until)
    }
}

/// Validation failures for a coverage range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageError {
    /// The exclusive end precedes the inclusive start.
    EndBeforeStart,
}

impl fmt::Display for CoverageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("coverage valid_until must not precede valid_from")
    }
}

impl Error for CoverageError {}
