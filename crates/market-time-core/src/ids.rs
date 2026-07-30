//! Opaque, string-backed identifiers.
//!
//! Venues, dataset revisions, and zones are data, not code. They are newtypes over
//! `String` rather than enums so that adding a venue or a revision is a data change —
//! per the core API contract, "adding a venue or a dataset revision is a data change,
//! never a core code change".

use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident, $what:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Wraps a ", $what, " identifier.")]
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[doc = concat!("The ", $what, " identifier as a string slice.")]
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

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }
    };
}

string_id!(
    /// Identifies a venue whose schedule is tracked, e.g. `"XSHG"`.
    VenueId,
    "venue"
);

string_id!(
    /// Identifies an immutable dataset revision, e.g. `"xshg-2026-07-29"`.
    DatasetRevisionId,
    "dataset revision"
);

string_id!(
    /// An IANA time-zone name, e.g. `"America/New_York"`.
    IanaZoneId,
    "IANA zone"
);
