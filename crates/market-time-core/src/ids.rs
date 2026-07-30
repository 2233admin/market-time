//! Opaque, string-backed identifiers.
//!
//! Venues, dataset revisions, and zones are data, not code. They are newtypes over
//! `String` rather than enums so that adding a venue or a revision is a data change —
//! per the core API contract, "adding a venue or a dataset revision is a data change,
//! never a core code change".
//!
//! Construction is validated. An empty or whitespace identifier would satisfy the type
//! while naming nothing, and would then ride through a whole answer as provenance that
//! points at nothing. It is rejected where it enters instead.

use std::fmt;

/// Why an identifier could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierError {
    kind: &'static str,
}

impl IdentifierError {
    /// What kind of identifier was empty, e.g. `"venue id"`.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} must not be empty", self.kind)
    }
}

impl std::error::Error for IdentifierError {}

fn non_empty(value: impl Into<String>, kind: &'static str) -> Result<String, IdentifierError> {
    let value = value.into();
    if value.trim().is_empty() {
        Err(IdentifierError { kind })
    } else {
        Ok(value)
    }
}

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident, $what:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Validates and wraps a ", $what, ".")]
            ///
            /// # Errors
            ///
            /// Returns [`IdentifierError`] when the value is empty or only whitespace.
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                non_empty(value, $what).map(Self)
            }

            #[doc = concat!("The ", $what, " as a string slice.")]
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = IdentifierError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }
    };
}

string_id!(
    /// Identifies a venue whose schedule is tracked, e.g. `"XSHG"`.
    VenueId,
    "venue id"
);

string_id!(
    /// Identifies an immutable dataset revision, e.g. `"xshg-2026-07-29"`.
    DatasetRevisionId,
    "dataset revision id"
);

string_id!(
    /// An IANA time-zone name, e.g. `"America/New_York"`.
    IanaZoneId,
    "IANA zone id"
);

string_id!(
    /// Identifies a derived session band, e.g. `"session-band-london"`.
    ///
    /// A band is never a venue: it names the *derived* grouping in
    /// [`crate::BandDefinition`], not a rule with evidence of its own.
    BandId,
    "band id"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_identifier_is_rejected_where_it_enters() {
        assert_eq!(
            VenueId::new("   ").expect_err("blank is rejected").kind(),
            "venue id"
        );
        assert!(DatasetRevisionId::new("").is_err());
        assert!(IanaZoneId::new("\t").is_err());
    }

    #[test]
    fn a_real_identifier_round_trips() {
        let venue = VenueId::new("XSHG").expect("valid identifier");
        assert_eq!(venue.as_str(), "XSHG");
        assert_eq!(venue.to_string(), "XSHG");
        assert_eq!("XSHG".parse::<VenueId>().expect("parses"), venue);
    }
}
