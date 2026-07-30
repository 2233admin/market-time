//! Opaque identifiers carried by rule data and answers.

use std::{error::Error, fmt};

/// An error returned when an opaque identifier is empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentifierError {
    kind: &'static str,
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} must not be empty", self.kind)
    }
}

impl Error for IdentifierError {}

/// An opaque venue identifier such as `SSE` or an operator-defined identifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VenueId(String);

impl VenueId {
    /// Validates and constructs a venue identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] when the supplied value is empty or whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        non_empty(value, "venue id").map(Self)
    }

    /// Borrows the identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VenueId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The stable identifier of one immutable rule-data revision.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DatasetRevisionId(String);

impl DatasetRevisionId {
    /// Validates and constructs a dataset revision identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] when the supplied value is empty or whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        non_empty(value, "dataset revision id").map(Self)
    }

    /// Borrows the identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DatasetRevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An opaque IANA time-zone identifier such as `America/New_York`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IanaZoneId(String);

impl IanaZoneId {
    /// Validates and constructs an IANA zone identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] when the supplied value is empty or whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        non_empty(value, "IANA zone id").map(Self)
    }

    /// Borrows the identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IanaZoneId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn non_empty(value: impl Into<String>, kind: &'static str) -> Result<String, IdentifierError> {
    let value = value.into();
    if value.trim().is_empty() {
        Err(IdentifierError { kind })
    } else {
        Ok(value)
    }
}
