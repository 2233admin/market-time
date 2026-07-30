//! Retrieving a source document, and recording what was retrieved.
//!
//! Mark Time is a client, not a redistributor. Every launch venue forbids commercial
//! redistribution of its published schedule, so the compliant shape is: the operator
//! fetches, under the operator's own relationship with the venue, and the tooling helps
//! them turn what came back into an evidenced dataset revision.
//!
//! # Why the transport is a trait and not an HTTP client
//!
//! This crate deliberately vendors no network stack. Whose credentials, whose proxy, whose
//! rate limit, and whose agreement with the venue are all operator questions, and an HTTP
//! client compiled in here would answer them badly by default. [`SourceFetcher`] is the
//! seam; [`FileFetcher`] covers the case where the operator already has the document, and
//! anything else — a few lines around whatever HTTP client the operator already trusts —
//! plugs into the same trait.
//!
//! # What is recorded
//!
//! A source's terms are recorded **at registration**, before any programmatic retrieval,
//! alongside its evidence — the same discipline as `source_url` and `fetched_at`, and for
//! the same reason: "under what terms did we obtain this record" has to be answerable per
//! record.

use market_time_core::UtcInstant;
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};

/// A source, registered before anything is fetched from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRegistration {
    url: String,
    terms: String,
    note: Option<String>,
}

impl SourceRegistration {
    /// Registers a source together with the terms it may be used under.
    ///
    /// # Errors
    ///
    /// Returns [`FetchError::MissingUrl`] for a blank URL and [`FetchError::MissingTerms`]
    /// for blank terms. Terms are not optional metadata: a document whose terms nobody
    /// wrote down is a document nobody can say we were allowed to use.
    pub fn new(url: impl Into<String>, terms: impl Into<String>) -> Result<Self, FetchError> {
        let url = url.into();
        let terms = terms.into();
        if url.trim().is_empty() {
            return Err(FetchError::MissingUrl);
        }
        if terms.trim().is_empty() {
            return Err(FetchError::MissingTerms { url });
        }
        Ok(Self {
            url,
            terms,
            note: None,
        })
    }

    /// Adds a free-text note, e.g. which page of the terms the quotation came from.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// The document's location.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The terms recorded at registration.
    #[must_use]
    pub fn terms(&self) -> &str {
        &self.terms
    }

    /// The registration note, where one was given.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// A document that was retrieved, with the provenance of the retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedDocument {
    source: SourceRegistration,
    fetched_at: UtcInstant,
    bytes: Vec<u8>,
    digest: String,
}

impl FetchedDocument {
    /// Records a retrieval. The digest is computed here rather than supplied, so it cannot
    /// disagree with the bytes it describes.
    #[must_use]
    pub fn new(source: SourceRegistration, fetched_at: UtcInstant, bytes: Vec<u8>) -> Self {
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        Self {
            source,
            fetched_at,
            bytes,
            digest,
        }
    }

    /// Where it came from.
    #[must_use]
    pub fn source(&self) -> &SourceRegistration {
        &self.source
    }

    /// When it was retrieved.
    #[must_use]
    pub fn fetched_at(&self) -> UtcInstant {
        self.fetched_at
    }

    /// The bytes as retrieved.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The content digest, `sha256:<hex>`.
    ///
    /// This is what makes "the venue changed its page" a detectable event rather than a
    /// surprise: two retrievals of the same document either agree or they do not.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// The document as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns [`FetchError::NotText`] when the bytes are not valid UTF-8.
    pub fn text(&self) -> Result<&str, FetchError> {
        std::str::from_utf8(&self.bytes).map_err(|_| FetchError::NotText {
            url: self.source.url.clone(),
        })
    }
}

/// How a document is retrieved.
///
/// Implementations decide the transport. They do not decide the provenance: `fetched_at`
/// is supplied by the caller, because the shell owns the clock, and the digest is computed
/// from the bytes.
pub trait SourceFetcher {
    /// Retrieves `source`, recording `fetched_at` as the retrieval time.
    ///
    /// # Errors
    ///
    /// Returns [`FetchError`] when the document cannot be retrieved.
    fn fetch(
        &self,
        source: &SourceRegistration,
        fetched_at: UtcInstant,
    ) -> Result<FetchedDocument, FetchError>;
}

/// Reads a document the operator already has on disk.
///
/// The common real case: the venue's schedule was downloaded by hand, or by whatever the
/// operator's data pipeline already uses, and what is wanted here is the provenance record
/// around it rather than another downloader.
#[derive(Debug, Clone)]
pub struct FileFetcher {
    root: PathBuf,
}

impl FileFetcher {
    /// Reads documents relative to `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Where a source's document is expected on disk.
    ///
    /// The URL's last path segment is the file name, so a registration can name the real
    /// document and the local copy at once.
    #[must_use]
    pub fn path_for(&self, source: &SourceRegistration) -> PathBuf {
        let name = source
            .url
            .rsplit('/')
            .find(|segment| !segment.is_empty())
            .unwrap_or("document");
        self.root.join(name)
    }
}

impl SourceFetcher for FileFetcher {
    fn fetch(
        &self,
        source: &SourceRegistration,
        fetched_at: UtcInstant,
    ) -> Result<FetchedDocument, FetchError> {
        let path = self.path_for(source);
        read_document(source, fetched_at, &path)
    }
}

fn read_document(
    source: &SourceRegistration,
    fetched_at: UtcInstant,
    path: &Path,
) -> Result<FetchedDocument, FetchError> {
    let bytes = std::fs::read(path).map_err(|error| FetchError::Unreadable {
        url: source.url.clone(),
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    Ok(FetchedDocument::new(source.clone(), fetched_at, bytes))
}

/// Why a source could not be registered or retrieved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The registration had no URL.
    MissingUrl,
    /// The registration had no terms.
    MissingTerms {
        /// The URL that was being registered.
        url: String,
    },
    /// The document could not be read.
    Unreadable {
        /// The registered URL.
        url: String,
        /// Where the fetcher looked.
        path: String,
        /// What the operating system said.
        detail: String,
    },
    /// The document is not UTF-8 text.
    NotText {
        /// The registered URL.
        url: String,
    },
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUrl => f.write_str("a source registration needs a URL"),
            Self::MissingTerms { url } => write!(
                f,
                "no terms recorded for {url}; terms are recorded at registration, before \
                 anything is fetched"
            ),
            Self::Unreadable { url, path, detail } => {
                write!(f, "cannot read {url} from {path}: {detail}")
            }
            Self::NotText { url } => write!(f, "{url} is not UTF-8 text"),
        }
    }
}

impl std::error::Error for FetchError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant() -> UtcInstant {
        UtcInstant::from_seconds_since_unix_epoch(1_785_283_200)
    }

    #[test]
    fn a_source_without_terms_cannot_be_registered() {
        assert_eq!(
            SourceRegistration::new("https://venue.test/hours", "  "),
            Err(FetchError::MissingTerms {
                url: "https://venue.test/hours".to_owned()
            })
        );
    }

    #[test]
    fn a_retrieval_carries_a_digest_of_what_was_retrieved() {
        let source = SourceRegistration::new("https://venue.test/hours", "personal use only")
            .expect("valid registration");
        let first = FetchedDocument::new(source.clone(), instant(), b"09:30-16:00".to_vec());
        let same = FetchedDocument::new(source.clone(), instant(), b"09:30-16:00".to_vec());
        let changed = FetchedDocument::new(source, instant(), b"09:30-13:00".to_vec());

        assert_eq!(first.digest(), same.digest());
        assert_ne!(
            first.digest(),
            changed.digest(),
            "a changed document is a detectable event, not a surprise"
        );
        assert!(first.digest().starts_with("sha256:"));
    }

    #[test]
    fn the_terms_travel_with_the_document() {
        let source = SourceRegistration::new("https://venue.test/hours", "no redistribution")
            .expect("valid registration")
            .with_note("terms page, clause 7");
        let document = FetchedDocument::new(source, instant(), b"x".to_vec());

        assert_eq!(document.source().terms(), "no redistribution");
        assert_eq!(document.source().note(), Some("terms page, clause 7"));
    }
}
