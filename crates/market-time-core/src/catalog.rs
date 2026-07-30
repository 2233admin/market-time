//! What a venue is called and what family it belongs to.
//!
//! Resolution does not need any of this: a phase answer is the same whether the venue is
//! labelled "XSHG" or "上海证券交易所". A board does need it, and the alternative to
//! carrying it here is every surface keeping its own venue table — which is how a display
//! ends up disagreeing with the data it is displaying.
//!
//! The family is a closed vocabulary for the same reason phases are: grouping that depends
//! on matching substrings of a venue's name is grouping that breaks when a venue is renamed.

use std::fmt;

/// The kind of market a venue is.
///
/// Conventional boards group rows this way, and the grouping is a display concern rather
/// than a claim about the venue's schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AssetFamily {
    /// Cash equities.
    Equities,
    /// Spot metals, energy, and foreign exchange.
    SpotAndFx,
    /// Listed futures and perpetuals.
    Futures,
}

impl AssetFamily {
    /// A stable, lowercase identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equities => "equities",
            Self::SpotAndFx => "spot_and_fx",
            Self::Futures => "futures",
        }
    }

    /// The heading a surface prints for this family.
    #[must_use]
    pub fn heading(self) -> &'static str {
        match self {
            Self::Equities => "Equities",
            Self::SpotAndFx => "Spot and FX",
            Self::Futures => "Futures",
        }
    }

    /// Parses the identifier produced by [`AssetFamily::as_str`].
    ///
    /// `None` for anything outside the vocabulary, so a dataset inventing a family fails to
    /// load rather than producing a group nothing else knows about.
    #[must_use]
    pub fn from_str_exact(value: &str) -> Option<Self> {
        match value {
            "equities" => Some(Self::Equities),
            "spot_and_fx" => Some(Self::SpotAndFx),
            "futures" => Some(Self::Futures),
            _ => None,
        }
    }

    /// Every family, in the order a board lists them.
    #[must_use]
    pub fn all() -> [Self; 3] {
        [Self::Equities, Self::SpotAndFx, Self::Futures]
    }
}

impl fmt::Display for AssetFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How a venue is presented.
///
/// Every field is optional: a dataset that says nothing but the venue's identifier is still
/// a valid dataset, and a surface falls back to the identifier rather than inventing a name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VenueProfile {
    /// The venue's name as it prefers to be called.
    pub display_name: Option<String>,
    /// The city or region it is identified by.
    pub location: Option<String>,
    /// Which family it belongs to, for grouping.
    pub family: Option<AssetFamily>,
}

impl VenueProfile {
    /// The label a surface should print, falling back to the identifier.
    #[must_use]
    pub fn label<'a>(&'a self, venue_id: &'a str) -> &'a str {
        self.display_name.as_deref().unwrap_or(venue_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_family_vocabulary_is_closed() {
        for family in AssetFamily::all() {
            assert_eq!(AssetFamily::from_str_exact(family.as_str()), Some(family));
        }
        assert_eq!(AssetFamily::from_str_exact("commodities_and_futures"), None);
    }

    #[test]
    fn a_venue_without_a_name_falls_back_to_its_identifier() {
        assert_eq!(VenueProfile::default().label("XSHG"), "XSHG");
    }
}
