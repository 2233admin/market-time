//! Provenance carried on every rule, and therefore on every answer.
//!
//! Principle I is non-negotiable: a rule without a source document, a retrieval date, and
//! an effective date does not ship, however obviously correct it looks. A rule we derived
//! rather than read carries its reasoning, and derived is never presented as observed
//! (FR-010).

use crate::instant::UtcInstant;
use std::fmt;

/// Where a rule came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    /// The source document a person can open and verify (SC-002, SC-008).
    source_url: String,
    /// When we retrieved that document.
    fetched_at: UtcInstant,
    /// The date the rule takes effect, as the venue states it (ISO-8601 calendar date).
    effective_from: String,
    /// When the publisher last changed the document, where the publisher provides it.
    publisher_last_changed: Option<String>,
}

impl EvidenceRef {
    /// Builds an evidence reference.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError`] when the source URL or the effective date is blank. A
    /// blank field would satisfy the type while defeating the principle, so it is
    /// rejected here rather than discovered by a reader later.
    pub fn new(
        source_url: impl Into<String>,
        fetched_at: UtcInstant,
        effective_from: impl Into<String>,
    ) -> Result<Self, EvidenceError> {
        let source_url = source_url.into();
        let effective_from = effective_from.into();
        if source_url.trim().is_empty() {
            return Err(EvidenceError::MissingSourceUrl);
        }
        if effective_from.trim().is_empty() {
            return Err(EvidenceError::MissingEffectiveFrom);
        }
        Ok(Self {
            source_url,
            fetched_at,
            effective_from,
            publisher_last_changed: None,
        })
    }

    /// Records when the publisher last changed the document.
    #[must_use]
    pub fn with_publisher_last_changed(mut self, when: impl Into<String>) -> Self {
        self.publisher_last_changed = Some(when.into());
        self
    }

    /// The source document.
    #[must_use]
    pub fn source_url(&self) -> &str {
        &self.source_url
    }

    /// When the source was retrieved.
    #[must_use]
    pub fn fetched_at(&self) -> UtcInstant {
        self.fetched_at
    }

    /// The date the rule takes effect.
    #[must_use]
    pub fn effective_from(&self) -> &str {
        &self.effective_from
    }

    /// When the publisher last changed the document, where known.
    #[must_use]
    pub fn publisher_last_changed(&self) -> Option<&str> {
        self.publisher_last_changed.as_deref()
    }
}

/// Why an evidence reference could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    /// No source document was given.
    MissingSourceUrl,
    /// No effective date was given.
    MissingEffectiveFrom,
    /// A rule was marked derived without reasoning.
    DerivedWithoutReasoning,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSourceUrl => f.write_str("evidence has no source document"),
            Self::MissingEffectiveFrom => f.write_str("evidence has no effective date"),
            Self::DerivedWithoutReasoning => f.write_str("a derived rule must carry its reasoning"),
        }
    }
}

impl std::error::Error for EvidenceError {}

/// Why a rule is a reading of a source rather than a quotation of one.
///
/// A derived rule cannot exist without this: the constructor rejects empty reasoning, so
/// "derived, unexplained" is not a state the type can hold (FR-010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationNote {
    reasoning: String,
}

impl DerivationNote {
    /// Builds a derivation note.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError::DerivedWithoutReasoning`] when the reasoning is blank.
    pub fn new(reasoning: impl Into<String>) -> Result<Self, EvidenceError> {
        let reasoning = reasoning.into();
        if reasoning.trim().is_empty() {
            return Err(EvidenceError::DerivedWithoutReasoning);
        }
        Ok(Self { reasoning })
    }

    /// The reasoning behind the derivation.
    #[must_use]
    pub fn reasoning(&self) -> &str {
        &self.reasoning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant() -> UtcInstant {
        UtcInstant::from_seconds_since_unix_epoch(1_750_000_000)
    }

    #[test]
    fn evidence_without_a_source_is_rejected() {
        assert_eq!(
            EvidenceRef::new("   ", instant(), "2026-01-01"),
            Err(EvidenceError::MissingSourceUrl)
        );
    }

    #[test]
    fn evidence_without_an_effective_date_is_rejected() {
        assert_eq!(
            EvidenceRef::new("https://example.test/rules", instant(), ""),
            Err(EvidenceError::MissingEffectiveFrom)
        );
    }

    #[test]
    fn a_derived_rule_cannot_ship_unexplained() {
        assert_eq!(
            DerivationNote::new(" "),
            Err(EvidenceError::DerivedWithoutReasoning)
        );
        assert!(
            DerivationNote::new("two intervals unassigned; read from the trading rules").is_ok()
        );
    }
}
