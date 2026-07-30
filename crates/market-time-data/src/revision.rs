//! Turning fetched documents into an immutable dataset revision.
//!
//! What the operator has after fetching is bytes and provenance. What the engine needs is
//! a validated ruleset. This module is the step between: it fills evidence in from the
//! retrieval record rather than from typing, refuses to write a revision the loader would
//! reject, and writes to a path the operator chooses — never into this repository, which
//! ships no venue data (`DATA-LICENSING.md`, and CI fails the build if a `data/` directory
//! appears).

use crate::fetch::FetchedDocument;
use crate::format::{
    CoverageRecord, DatasetFile, EventRecord, EvidenceRecord, RevisionRecord, RuleRecord,
    VenueRecord,
};
use crate::load::{LoadError, parse_ruleset};
use market_time_core::UtcInstant;
use std::fmt;
use std::path::Path;

/// A dataset revision under construction.
///
/// Immutability is a property of a revision once written: correcting a rule produces a new
/// revision that supersedes the old one, never an edit in place (Principle III). This type
/// is the mutable stage before that, and it has no way to reopen a revision already on
/// disk.
#[derive(Debug, Clone)]
pub struct RevisionAssembly {
    revision: RevisionRecord,
    venues: Vec<VenueRecord>,
}

impl RevisionAssembly {
    /// Starts a revision.
    ///
    /// `supersedes` is the revision this one replaces, where it replaces one. The tzdb
    /// release is recorded from the build that assembled the data, because zone rules are
    /// as much a part of an answer as the venue's own schedule.
    #[must_use]
    pub fn new(id: impl Into<String>, assembled_at: UtcInstant) -> Self {
        Self {
            revision: RevisionRecord {
                id: id.into(),
                supersedes: None,
                iana_tzdb_version: market_time_core::tzdata::iana_tzdb_version()
                    .map(ToOwned::to_owned),
                assembled_at: rfc3339(assembled_at),
            },
            venues: Vec::new(),
        }
    }

    /// Records which revision this one replaces.
    #[must_use]
    pub fn superseding(mut self, previous: impl Into<String>) -> Self {
        self.revision.supersedes = Some(previous.into());
        self
    }

    /// The revision identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.revision.id
    }

    /// Adds a venue's rules to the revision.
    #[must_use]
    pub fn with_venue(mut self, venue: VenueRecord) -> Self {
        self.venues.push(venue);
        self
    }

    /// Builds a venue record whose rules all cite this revision.
    ///
    /// Rule records carry a `revision` field; setting it here rather than asking each
    /// caller to repeat the identifier is the difference between one source of truth and
    /// several chances to typo.
    #[must_use]
    pub fn venue(
        &self,
        venue: impl Into<String>,
        home_zone: impl Into<String>,
        coverage: (UtcInstant, UtcInstant),
        evidence: Vec<EvidenceRecord>,
        mut rules: Vec<RuleRecord>,
        mut events: Vec<EventRecord>,
    ) -> VenueRecord {
        for rule in &mut rules {
            rule.revision.clone_from(&self.revision.id);
        }
        for event in &mut events {
            event.revision.clone_from(&self.revision.id);
        }

        VenueRecord {
            venue: venue.into(),
            home_zone: home_zone.into(),
            display_name: None,
            location: None,
            family: None,
            coverage: CoverageRecord {
                start: rfc3339(coverage.0),
                end: rfc3339(coverage.1),
            },
            evidence,
            rules,
            events,
        }
    }

    /// The dataset as it would be written.
    #[must_use]
    pub fn to_json(&self) -> String {
        let file = DatasetFile {
            revisions: vec![self.revision.clone()],
            venues: self.venues.clone(),
        };
        serde_json::to_string_pretty(&file).unwrap_or_default()
    }

    /// Checks that the loader would accept this revision.
    ///
    /// # Errors
    ///
    /// Returns the [`LoadError`] the loader would have returned. Validation is the loader
    /// itself rather than a second set of rules that could drift from it: coverage the
    /// rules cannot answer, a phase outside the shared vocabulary, and a rule without a
    /// source all fail here for exactly the reason they would fail on load.
    pub fn validate(&self) -> Result<(), LoadError> {
        parse_ruleset(&self.to_json()).map(|_| ())
    }

    /// Validates, then writes the revision to `path`.
    ///
    /// # Errors
    ///
    /// Returns [`AssemblyError::Invalid`] when the revision would not load, and
    /// [`AssemblyError::Unwritable`] when the file cannot be written. Nothing is written
    /// when validation fails: a dataset on disk that the engine refuses to read is worse
    /// than no dataset, because it looks like data.
    pub fn write(&self, path: &Path) -> Result<(), AssemblyError> {
        self.validate()
            .map_err(|error| AssemblyError::Invalid(error.to_string()))?;

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| AssemblyError::Unwritable {
                path: path.display().to_string(),
                detail: error.to_string(),
            })?;
        }

        std::fs::write(path, self.to_json()).map_err(|error| AssemblyError::Unwritable {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }
}

/// Builds an evidence record from a retrieval, so provenance is transcribed by the
/// machine that did the fetching rather than by a person at a keyboard.
///
/// The digest travels with it: the answer a rule produced is attributable not merely to a
/// URL, but to the exact bytes that URL returned when it was read.
#[must_use]
pub fn evidence_from(
    document: &FetchedDocument,
    effective_from: impl Into<String>,
) -> EvidenceRecord {
    EvidenceRecord {
        source_url: document.source().url().to_owned(),
        fetched_at: rfc3339(document.fetched_at()),
        effective_from: effective_from.into(),
        publisher_last_changed: None,
        terms: Some(document.source().terms().to_owned()),
        digest: Some(document.digest().to_owned()),
    }
}

fn rfc3339(at: UtcInstant) -> String {
    jiff::Timestamp::from_nanosecond(at.as_nanos_since_unix_epoch())
        .map_or_else(|_| at.to_string(), |timestamp| timestamp.to_string())
}

/// Why a revision could not be assembled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblyError {
    /// The revision would not load.
    Invalid(String),
    /// The revision could not be written.
    Unwritable {
        /// The path that was tried.
        path: String,
        /// What the operating system said.
        detail: String,
    },
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(detail) => write!(
                f,
                "refusing to write a revision the loader would reject: {detail}"
            ),
            Self::Unwritable { path, detail } => write!(f, "cannot write {path}: {detail}"),
        }
    }
}

impl std::error::Error for AssemblyError {}
