//! Evidence references attached to every rule-derived answer.

use std::{error::Error, fmt};

use crate::UtcInstant;

/// A compact reference to the source that justifies a phase segment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRef {
    /// Stable, openable source location.
    pub source_url: String,
    /// When the operator retrieved the source.
    pub fetched_at: UtcInstant,
    /// When the source says the rule took effect.
    pub effective_from: UtcInstant,
    /// When the publisher says the source was updated, if provided.
    pub source_updated_at: Option<UtcInstant>,
    /// Whether the rule was reasoned about rather than directly published.
    pub is_derived: bool,
    /// The reasoning for a derived rule; absent for an observed rule.
    pub derivation_reasoning: Option<String>,
}

impl EvidenceRef {
    /// Constructs evidence for a directly observed rule.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError::InvalidSourceUrl`] when `source_url` is not a non-empty
    /// absolute URI.
    pub fn observed(
        source_url: impl Into<String>,
        fetched_at: UtcInstant,
        effective_from: UtcInstant,
        source_updated_at: Option<UtcInstant>,
    ) -> Result<Self, EvidenceError> {
        let source_url = validate_source_url(source_url.into())?;
        Ok(Self {
            source_url,
            fetched_at,
            effective_from,
            source_updated_at,
            is_derived: false,
            derivation_reasoning: None,
        })
    }

    /// Constructs evidence for a derived rule and requires non-empty reasoning.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError::InvalidSourceUrl`] for an invalid source URI or
    /// [`EvidenceError::MissingDerivationReasoning`] for empty reasoning.
    pub fn derived(
        source_url: impl Into<String>,
        fetched_at: UtcInstant,
        effective_from: UtcInstant,
        source_updated_at: Option<UtcInstant>,
        reasoning: impl Into<String>,
    ) -> Result<Self, EvidenceError> {
        let source_url = validate_source_url(source_url.into())?;
        let reasoning = reasoning.into();
        if reasoning.trim().is_empty() {
            return Err(EvidenceError::MissingDerivationReasoning);
        }
        Ok(Self {
            source_url,
            fetched_at,
            effective_from,
            source_updated_at,
            is_derived: true,
            derivation_reasoning: Some(reasoning),
        })
    }
}

/// Validation failures for an evidence reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceError {
    /// The source URL was empty or did not include a URI scheme.
    InvalidSourceUrl,
    /// A derived rule omitted the reasoning that produced it.
    MissingDerivationReasoning,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourceUrl => {
                formatter.write_str("source_url must be a non-empty absolute URI")
            }
            Self::MissingDerivationReasoning => {
                formatter.write_str("derived evidence requires non-empty reasoning")
            }
        }
    }
}

impl Error for EvidenceError {}

fn validate_source_url(source_url: String) -> Result<String, EvidenceError> {
    if source_url.trim().is_empty() || !source_url.contains("://") {
        Err(EvidenceError::InvalidSourceUrl)
    } else {
        Ok(source_url)
    }
}
