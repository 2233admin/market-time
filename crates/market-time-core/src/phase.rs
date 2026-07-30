//! The shared phase vocabulary.
//!
//! One vocabulary describes every venue. No venue introduces a phase name of its own
//! (FR-005), which is enforced structurally: the enum lives in this crate, is
//! `#[non_exhaustive]`, and shells cannot extend it.

use std::fmt;

/// A state a venue is in for a contiguous stretch of time.
///
/// Phases tile all covered time with no gaps and no overlaps (FR-008). Things that are
/// scheduled but are not states — a funding settlement, say — are [`crate::event`]s
/// overlaid on a phase, never phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Phase {
    /// Not trading and not in any pre- or post-trading state.
    Closed,
    /// Order entry is open ahead of the opening auction.
    PreOpen,
    /// The opening auction itself.
    OpeningAuction,
    /// Ordinary continuous matching.
    ContinuousTrading,
    /// A scheduled break inside the trading day; neither open nor closed.
    MidDayBreak,
    /// The closing auction itself.
    ClosingAuction,
    /// After the close, while post-close facilities remain available.
    PostClose,
    /// Trading stopped inside a stretch the schedule says should be trading.
    NonTradingInterruption,
}

impl Phase {
    /// A stable, lowercase identifier, for datasets and machine consumers.
    #[must_use]
    pub fn as_str(self) -> &'static str {
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

    /// Parses the identifier produced by [`Phase::as_str`].
    ///
    /// Returns `None` for anything outside the vocabulary — a dataset naming its own
    /// phase fails to load rather than smuggling a venue-specific name in (FR-005).
    #[must_use]
    pub fn from_str_exact(value: &str) -> Option<Self> {
        Some(match value {
            "closed" => Self::Closed,
            "pre_open" => Self::PreOpen,
            "opening_auction" => Self::OpeningAuction,
            "continuous_trading" => Self::ContinuousTrading,
            "mid_day_break" => Self::MidDayBreak,
            "closing_auction" => Self::ClosingAuction,
            "post_close" => Self::PostClose,
            "non_trading_interruption" => Self::NonTradingInterruption,
            _ => return None,
        })
    }

    /// Whether orders match against each other during this phase.
    ///
    /// A convenience for shells that need a coarse rendering. It is not a second
    /// vocabulary: shells still receive and display the phase itself.
    #[must_use]
    pub fn is_trading(self) -> bool {
        matches!(
            self,
            Self::OpeningAuction | Self::ContinuousTrading | Self::ClosingAuction
        )
    }

    /// Every phase in the vocabulary.
    #[must_use]
    pub fn all() -> [Self; 8] {
        [
            Self::Closed,
            Self::PreOpen,
            Self::OpeningAuction,
            Self::ContinuousTrading,
            Self::MidDayBreak,
            Self::ClosingAuction,
            Self::PostClose,
            Self::NonTradingInterruption,
        ]
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vocabulary_round_trips_and_is_closed() {
        for phase in Phase::all() {
            assert_eq!(Phase::from_str_exact(phase.as_str()), Some(phase));
        }
        assert_eq!(Phase::from_str_exact("after_hours_fixed_price"), None);
        assert_eq!(Phase::from_str_exact("night_session"), None);
    }
}
