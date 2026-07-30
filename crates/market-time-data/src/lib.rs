//! Mark Time — dataset revisions and loading. All I/O lives here.
//!
//! # Why this crate carries the whole ingestion story
//!
//! Every launch venue forbids commercial redistribution of its published schedule, and
//! none carves out factual or calendar data (research D6a):
//!
//! | Venue   | Governing text            | Position                                       |
//! |---------|---------------------------|------------------------------------------------|
//! | SSE     | Trading Rules Art. 5.1.3  | use or publication requires Exchange permission |
//! | NYSE    | ICE Terms of Use          | personal, non-commercial only                   |
//! | Binance | ADGM Terms cl. 27         | non-commercial personal or internal use only    |
//!
//! So **no venue dataset ships in this repository**. Fetch-at-run-time is not one option
//! among several; it is the only compliant shape. Mark Time is a client, not a
//! redistributor — the operator fetches under their own relationship with each venue and
//! owns their own compliance.
//!
//! A source's terms are recorded at registration alongside its evidence, so "under what
//! terms did we obtain this record" is answerable per record, with the same discipline as
//! `source_url` and `fetched_at`.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

use std::{collections::HashMap, error::Error, fmt, fs, io, path::Path};

use market_time_core::{
    BoundaryCharacter, CoverageRange, DatasetRevision, DatasetRevisionId, EvidenceRef, IanaZoneId,
    Phase, PhaseBoundary, PhaseSegment, PhaseTimeline, Ruleset, Uncertainty, UtcInstant, VenueId,
    VenueRuleset,
};
use serde::Deserialize;

/// A validated source registration retained alongside a loaded ruleset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRegistration {
    /// Source location referenced by evidence records.
    pub source_url: String,
    /// Location of the terms reviewed before registration.
    pub terms_url: String,
    /// When the operator reviewed those terms.
    pub terms_checked_at: UtcInstant,
    /// Operator-recorded redistribution position.
    pub redistribution: String,
}

/// A materialized ruleset plus the source-term records used to admit its evidence.
#[derive(Debug)]
pub struct LoadedRuleset {
    /// Pure, validated rule data accepted by `market-time-core`.
    pub ruleset: Ruleset,
    /// Source registrations retained for audit.
    pub sources: Vec<SourceRegistration>,
}

/// Loads and validates a versioned JSON ruleset from an operator-supplied path.
///
/// This is intentionally in `market-time-data`: all filesystem I/O ends here and the
/// core only receives already materialized Rust values.
///
/// # Errors
///
/// Returns [`LoadError`] when the file cannot be read, JSON does not match the supported
/// schema, source terms are unregistered, or materialized core invariants are violated.
pub fn load_ruleset_from_path(path: impl AsRef<Path>) -> Result<LoadedRuleset, LoadError> {
    let content = fs::read_to_string(path).map_err(LoadError::Io)?;
    let document: RulesetDocument = serde_json::from_str(&content).map_err(LoadError::Json)?;
    document.materialize()
}

/// Failures while reading, decoding, or validating operator rule data.
#[derive(Debug)]
pub enum LoadError {
    /// The ruleset file could not be read.
    Io(io::Error),
    /// The ruleset was not valid JSON or did not match the schema.
    Json(serde_json::Error),
    /// The schema version is not supported by this build.
    UnsupportedSchema(u32),
    /// Decoded values violated a core or source-registration invariant.
    Invalid(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "could not read ruleset: {error}"),
            Self::Json(error) => write!(formatter, "invalid ruleset JSON: {error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported ruleset schema_version {version}")
            }
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl Error for LoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnsupportedSchema(_) | Self::Invalid(_) => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesetDocument {
    schema_version: u32,
    sources: Vec<SourceRegistrationWire>,
    revisions: Vec<DatasetRevisionWire>,
    venues: Vec<VenueRulesetWire>,
}

impl RulesetDocument {
    fn materialize(self) -> Result<LoadedRuleset, LoadError> {
        if self.schema_version != 1 {
            return Err(LoadError::UnsupportedSchema(self.schema_version));
        }
        let sources = self
            .sources
            .into_iter()
            .map(SourceRegistrationWire::materialize)
            .collect::<Result<Vec<_>, _>>()?;
        let source_index: HashMap<&str, &SourceRegistration> = sources
            .iter()
            .map(|source| (source.source_url.as_str(), source))
            .collect();
        if source_index.len() != sources.len() {
            return Err(LoadError::Invalid(
                "source registrations must have unique source_url values".to_owned(),
            ));
        }
        let revisions = self
            .revisions
            .into_iter()
            .map(DatasetRevisionWire::materialize)
            .collect::<Result<Vec<_>, _>>()?;
        let venues = self
            .venues
            .into_iter()
            .map(|venue| venue.materialize(&source_index))
            .collect::<Result<Vec<_>, _>>()?;
        let ruleset = Ruleset::from_parts(revisions, venues)
            .map_err(|error| LoadError::Invalid(format!("invalid ruleset: {error}")))?;
        Ok(LoadedRuleset { ruleset, sources })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRegistrationWire {
    source_url: String,
    terms_url: String,
    terms_checked_at_ns: i128,
    redistribution: String,
}

impl SourceRegistrationWire {
    fn materialize(self) -> Result<SourceRegistration, LoadError> {
        if !is_absolute_uri(&self.source_url) {
            return Err(LoadError::Invalid(
                "source registration source_url must be an absolute URI".to_owned(),
            ));
        }
        if !is_absolute_uri(&self.terms_url) {
            return Err(LoadError::Invalid(
                "source registration terms_url must be an absolute URI".to_owned(),
            ));
        }
        if self.redistribution.trim().is_empty() {
            return Err(LoadError::Invalid(
                "source registration redistribution must not be empty".to_owned(),
            ));
        }
        Ok(SourceRegistration {
            source_url: self.source_url,
            terms_url: self.terms_url,
            terms_checked_at: instant(self.terms_checked_at_ns),
            redistribution: self.redistribution,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetRevisionWire {
    id: String,
    dataset_kind: String,
    created_at_ns: i128,
    source_description: String,
    supersedes: Option<String>,
}

impl DatasetRevisionWire {
    fn materialize(self) -> Result<DatasetRevision, LoadError> {
        let id = revision_id(self.id)?;
        let supersedes = self.supersedes.map(revision_id).transpose()?;
        DatasetRevision::new(
            id,
            self.dataset_kind,
            instant(self.created_at_ns),
            self.source_description,
            supersedes,
        )
        .map_err(|error| LoadError::Invalid(format!("invalid dataset revision: {error}")))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VenueRulesetWire {
    id: String,
    home_zone: String,
    coverage: CoverageWire,
    segments: Vec<PhaseSegmentWire>,
}

impl VenueRulesetWire {
    fn materialize(
        self,
        source_index: &HashMap<&str, &SourceRegistration>,
    ) -> Result<VenueRuleset, LoadError> {
        let venue = VenueId::new(self.id)
            .map_err(|error| LoadError::Invalid(format!("invalid venue: {error}")))?;
        let home_zone = IanaZoneId::new(self.home_zone)
            .map_err(|error| LoadError::Invalid(format!("invalid home zone: {error}")))?;
        let coverage = self.coverage.materialize()?;
        let segments = self
            .segments
            .into_iter()
            .map(|segment| segment.materialize(source_index))
            .collect::<Result<Vec<_>, _>>()?;
        let timeline = PhaseTimeline::new(coverage, segments)
            .map_err(|error| LoadError::Invalid(format!("invalid phase timeline: {error}")))?;
        Ok(VenueRuleset::new(venue, home_zone, timeline))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageWire {
    valid_from_ns: i128,
    valid_until_ns: i128,
}

impl CoverageWire {
    fn materialize(self) -> Result<CoverageRange, LoadError> {
        CoverageRange::closed_open(instant(self.valid_from_ns), instant(self.valid_until_ns))
            .map_err(|error| LoadError::Invalid(format!("invalid coverage: {error}")))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PhaseSegmentWire {
    phase: String,
    start: PhaseBoundaryWire,
    end: PhaseBoundaryWire,
    evidence: Vec<EvidenceWire>,
    dataset_revisions: Vec<String>,
}

impl PhaseSegmentWire {
    fn materialize(
        self,
        source_index: &HashMap<&str, &SourceRegistration>,
    ) -> Result<PhaseSegment, LoadError> {
        let phase = self
            .phase
            .parse::<Phase>()
            .map_err(|error| LoadError::Invalid(format!("invalid phase: {error}")))?;
        let evidence = self
            .evidence
            .into_iter()
            .map(|evidence| evidence.materialize(source_index))
            .collect::<Result<Vec<_>, _>>()?;
        let dataset_revisions = self
            .dataset_revisions
            .into_iter()
            .map(revision_id)
            .collect::<Result<Vec<_>, _>>()?;
        PhaseSegment::new(
            phase,
            self.start.materialize()?,
            self.end.materialize()?,
            evidence,
            dataset_revisions,
        )
        .map_err(|error| LoadError::Invalid(format!("invalid phase segment: {error}")))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PhaseBoundaryWire {
    instant_ns: i128,
    uncertainty: UncertaintyWire,
}

impl PhaseBoundaryWire {
    fn materialize(self) -> Result<PhaseBoundary, LoadError> {
        Ok(PhaseBoundary::new(
            instant(self.instant_ns),
            self.uncertainty.materialize()?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncertaintyWire {
    granularity_ns: Option<u64>,
    published_bound_ns: Option<u64>,
    boundary_character: String,
    process_spread_ns: Option<u64>,
    is_derived: bool,
}

impl UncertaintyWire {
    fn materialize(self) -> Result<Uncertainty, LoadError> {
        let boundary_character = match self.boundary_character.as_str() {
            "instantaneous" => BoundaryCharacter::Instantaneous,
            "process_start" => BoundaryCharacter::ProcessStart,
            value => {
                return Err(LoadError::Invalid(format!(
                    "invalid boundary_character {value:?}"
                )));
            }
        };
        if self.process_spread_ns.is_some() && boundary_character != BoundaryCharacter::ProcessStart
        {
            return Err(LoadError::Invalid(
                "process_spread_ns requires boundary_character process_start".to_owned(),
            ));
        }
        Ok(Uncertainty {
            granularity_ns: self.granularity_ns,
            published_bound_ns: self.published_bound_ns,
            boundary_character,
            process_spread_ns: self.process_spread_ns,
            is_derived: self.is_derived,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceWire {
    source_url: String,
    fetched_at_ns: i128,
    effective_from_ns: i128,
    source_updated_at_ns: Option<i128>,
    is_derived: bool,
    derivation_reasoning: Option<String>,
}

impl EvidenceWire {
    fn materialize(
        self,
        source_index: &HashMap<&str, &SourceRegistration>,
    ) -> Result<EvidenceRef, LoadError> {
        if !source_index.contains_key(self.source_url.as_str()) {
            return Err(LoadError::Invalid(format!(
                "evidence source_url {:?} has no source registration",
                self.source_url
            )));
        }
        let fetched_at = instant(self.fetched_at_ns);
        let effective_from = instant(self.effective_from_ns);
        let source_updated_at = self.source_updated_at_ns.map(instant);
        match (self.is_derived, self.derivation_reasoning) {
            (false, None) => EvidenceRef::observed(
                self.source_url,
                fetched_at,
                effective_from,
                source_updated_at,
            ),
            (true, Some(reasoning)) => EvidenceRef::derived(
                self.source_url,
                fetched_at,
                effective_from,
                source_updated_at,
                reasoning,
            ),
            _ => {
                return Err(LoadError::Invalid(
                    "is_derived and derivation_reasoning must be paired".to_owned(),
                ));
            }
        }
        .map_err(|error| LoadError::Invalid(format!("invalid evidence: {error}")))
    }
}

const fn instant(nanos: i128) -> UtcInstant {
    UtcInstant::from_nanos_since_unix_epoch(nanos)
}

fn revision_id(value: String) -> Result<DatasetRevisionId, LoadError> {
    DatasetRevisionId::new(value)
        .map_err(|error| LoadError::Invalid(format!("invalid revision id: {error}")))
}

fn is_absolute_uri(value: &str) -> bool {
    !value.trim().is_empty() && value.contains("://")
}
