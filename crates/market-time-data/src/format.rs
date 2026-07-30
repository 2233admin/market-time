//! The on-disk dataset format.
//!
//! These types mirror the JSON a dataset revision is written in. They are deliberately
//! separate from the core's types: the core takes materialized values and knows nothing
//! about a file format, and a format change must not be able to reach into the decision
//! path.

use serde::{Deserialize, Serialize};

/// A whole dataset file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetFile {
    /// The revisions this file carries.
    pub revisions: Vec<RevisionRecord>,
    /// The venues it describes.
    pub venues: Vec<VenueRecord>,
}

/// One immutable revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionRecord {
    /// The revision identifier.
    pub id: String,
    /// The revision this one replaces.
    #[serde(default)]
    pub supersedes: Option<String>,
    /// The IANA tzdb release the data was prepared against.
    #[serde(default)]
    pub iana_tzdb_version: Option<String>,
    /// When the revision was assembled, RFC 3339.
    pub assembled_at: String,
}

/// One venue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueRecord {
    /// The venue identifier.
    pub venue: String,
    /// The venue's IANA home zone.
    pub home_zone: String,
    /// The venue's name as it prefers to be called. Display only.
    #[serde(default)]
    pub display_name: Option<String>,
    /// The city or region the venue is identified by. Display only.
    #[serde(default)]
    pub location: Option<String>,
    /// Which family the venue belongs to: `equities`, `spot_and_fx`, or `futures`.
    #[serde(default)]
    pub family: Option<String>,
    /// What the data is willing to answer for.
    pub coverage: CoverageRecord,
    /// Venue-level evidence.
    #[serde(default)]
    pub evidence: Vec<EvidenceRecord>,
    /// The schedule rules.
    pub rules: Vec<RuleRecord>,
    /// Scheduled occurrences that are not phases.
    #[serde(default)]
    pub events: Vec<EventRecord>,
}

/// A coverage declaration, RFC 3339 at both ends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageRecord {
    /// First instant inside coverage.
    pub start: String,
    /// First instant after coverage.
    pub end: String,
}

/// Provenance for a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    /// The source document.
    pub source_url: String,
    /// When it was retrieved, RFC 3339.
    pub fetched_at: String,
    /// The date the rule takes effect.
    pub effective_from: String,
    /// When the publisher last changed the document.
    #[serde(default)]
    pub publisher_last_changed: Option<String>,
    /// The terms the source was obtained under, recorded at registration.
    #[serde(default)]
    pub terms: Option<String>,
    /// Digest of the retrieved bytes, `sha256:<hex>`, where the document was fetched
    /// rather than transcribed. Two retrievals of the same document either agree or they
    /// do not; this is what makes that checkable after the fact.
    #[serde(default)]
    pub digest: Option<String>,
}

/// One schedule rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleRecord {
    /// What kind of statement the rule makes.
    pub kind: RuleKindRecord,
    /// The venue-local day, as ordered change points starting at `00:00`.
    pub schedule: Vec<ChangePointRecord>,
    /// The dates the rule is in force.
    pub applies: DateRangeRecord,
    /// How precisely the source publishes these boundaries.
    pub boundary_uncertainty: UncertaintyRecord,
    /// Where the rule came from.
    pub evidence: EvidenceRecord,
    /// Why the rule is our reading rather than the venue's wording.
    #[serde(default)]
    pub derived_reasoning: Option<String>,
    /// The revision this rule belongs to.
    pub revision: String,
}

/// A rule kind, tagged by `type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleKindRecord {
    /// The ordinary weekly pattern.
    WeeklyPattern {
        /// Three-letter weekday names, e.g. `["mon", "tue"]`.
        weekdays: Vec<String>,
    },
    /// A holiday.
    Holiday {
        /// The holiday, as the venue names it.
        name: String,
    },
    /// A shortened session.
    ShortenedSession {
        /// The session, as the venue names it.
        name: String,
    },
    /// An announced schedule change.
    AnnouncedChange {
        /// What the venue announced.
        note: String,
    },
}

/// One change point in a day schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePointRecord {
    /// Venue-local time of day, `HH:MM` or `HH:MM:SS`.
    pub at: String,
    /// The phase that begins here, from the shared vocabulary.
    pub phase: String,
}

/// An inclusive range of venue-local dates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRangeRecord {
    /// First date, `YYYY-MM-DD`.
    pub start: String,
    /// Last date, inclusive.
    pub end: String,
}

/// How precisely something is known, tagged by `type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UncertaintyRecord {
    /// Exact to the representation.
    Exact,
    /// The source publishes to a granularity.
    PublishedGranularity {
        /// Width of the publication grid, in seconds.
        seconds: i64,
    },
    /// The venue published its own bound.
    PublishedBound {
        /// The bound, in seconds.
        seconds: i64,
        /// What the venue said.
        published_as: String,
    },
    /// The boundary is the scheduled start of a process.
    ProcessStart {
        /// The process, in the venue's own terms.
        process: String,
    },
}

/// One event rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// The event kind, from the shared vocabulary.
    pub kind: String,
    /// Venue-local times of day it occurs at.
    pub times: Vec<String>,
    /// The dates the rule is in force.
    pub applies: DateRangeRecord,
    /// How precisely the occurrence instant is known.
    pub uncertainty: UncertaintyRecord,
    /// Where the rule came from.
    pub evidence: EvidenceRecord,
    /// The revision this rule belongs to.
    pub revision: String,
}
