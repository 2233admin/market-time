//! Immutable, fully materialized rule-data revisions and venue timelines.

use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
};

use crate::{DatasetRevisionId, IanaZoneId, PhaseTimeline, UtcInstant, VenueId};

/// One immutable full snapshot of a rule dataset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatasetRevision {
    /// Stable identifier that is never reassigned.
    pub id: DatasetRevisionId,
    /// Operator-defined dataset class.
    pub dataset_kind: String,
    /// When this immutable revision was created.
    pub created_at: UtcInstant,
    /// Human-readable description of the underlying source set.
    pub source_description: String,
    /// The prior immutable revision corrected by this one, if any.
    pub supersedes: Option<DatasetRevisionId>,
}

impl DatasetRevision {
    /// Validates and constructs immutable revision metadata.
    ///
    /// # Errors
    ///
    /// Returns [`DatasetRevisionError`] when required descriptive fields are empty or
    /// the revision claims to supersede itself.
    pub fn new(
        id: DatasetRevisionId,
        dataset_kind: impl Into<String>,
        created_at: UtcInstant,
        source_description: impl Into<String>,
        supersedes: Option<DatasetRevisionId>,
    ) -> Result<Self, DatasetRevisionError> {
        let dataset_kind = dataset_kind.into();
        if dataset_kind.trim().is_empty() {
            return Err(DatasetRevisionError::EmptyDatasetKind);
        }
        let source_description = source_description.into();
        if source_description.trim().is_empty() {
            return Err(DatasetRevisionError::EmptySourceDescription);
        }
        if supersedes.as_ref() == Some(&id) {
            return Err(DatasetRevisionError::SupersedesItself);
        }
        Ok(Self {
            id,
            dataset_kind,
            created_at,
            source_description,
            supersedes,
        })
    }
}

/// Validation failures for immutable revision metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DatasetRevisionError {
    /// The dataset kind was empty.
    EmptyDatasetKind,
    /// The source description was empty.
    EmptySourceDescription,
    /// The revision claimed to supersede itself.
    SupersedesItself,
}

impl fmt::Display for DatasetRevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for DatasetRevisionError {}

/// The fully materialized, structurally validated rules for one venue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VenueRuleset {
    /// Venue identifier.
    pub venue: VenueId,
    /// Venue home zone used by the ingestion layer when materializing civil rules.
    pub home_zone: IanaZoneId,
    /// Complete UTC phase timeline for the declared coverage revision.
    pub timeline: PhaseTimeline,
}

impl VenueRuleset {
    /// Constructs a venue ruleset from an already validated timeline.
    #[must_use]
    pub const fn new(venue: VenueId, home_zone: IanaZoneId, timeline: PhaseTimeline) -> Self {
        Self {
            venue,
            home_zone,
            timeline,
        }
    }
}

/// A validated, in-memory-only ruleset accepted by the pure query core.
#[derive(Clone, Debug)]
pub struct Ruleset {
    revisions: Vec<DatasetRevision>,
    venues: Vec<VenueRuleset>,
    venue_index: HashMap<VenueId, usize>,
}

impl Ruleset {
    /// Validates fully materialized values without reading any file, URL, socket, or clock.
    ///
    /// # Errors
    ///
    /// Returns [`RulesetError`] for duplicate identifiers, invalid supersedes links, or
    /// phase segments that cite absent revisions.
    pub fn from_parts(
        revisions: Vec<DatasetRevision>,
        venues: Vec<VenueRuleset>,
    ) -> Result<Self, RulesetError> {
        let mut revision_ids = HashSet::with_capacity(revisions.len());
        for revision in &revisions {
            if !revision_ids.insert(revision.id.clone()) {
                return Err(RulesetError::DuplicateRevision(revision.id.clone()));
            }
            if let Some(supersedes) = &revision.supersedes
                && !revisions
                    .iter()
                    .any(|candidate| &candidate.id == supersedes)
            {
                return Err(RulesetError::UnknownSupersededRevision(supersedes.clone()));
            }
        }

        let mut venue_index = HashMap::with_capacity(venues.len());
        for (index, venue) in venues.iter().enumerate() {
            if venue_index.insert(venue.venue.clone(), index).is_some() {
                return Err(RulesetError::DuplicateVenue(venue.venue.clone()));
            }
            for segment in venue.timeline.segments() {
                for revision_id in segment.dataset_revisions() {
                    if !revision_ids.contains(revision_id) {
                        return Err(RulesetError::UnknownSegmentRevision(revision_id.clone()));
                    }
                }
            }
        }

        Ok(Self {
            revisions,
            venues,
            venue_index,
        })
    }

    /// Returns immutable revision metadata in stable input order.
    #[must_use]
    pub fn revisions(&self) -> &[DatasetRevision] {
        &self.revisions
    }

    /// Returns revision identifiers in stable input order.
    #[must_use]
    pub fn revision_ids(&self) -> Vec<DatasetRevisionId> {
        self.revisions
            .iter()
            .map(|revision| revision.id.clone())
            .collect()
    }

    /// Returns venue metadata and timelines in stable input order.
    #[must_use]
    pub fn venues(&self) -> &[VenueRuleset] {
        &self.venues
    }

    pub(crate) fn venue(&self, venue: &VenueId) -> Option<&VenueRuleset> {
        self.venue_index
            .get(venue)
            .and_then(|index| self.venues.get(*index))
    }
}

/// Validation failures caught once, before a ruleset reaches the query path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RulesetError {
    /// Two revision snapshots reused the same stable identifier.
    DuplicateRevision(DatasetRevisionId),
    /// A supersedes link named a revision absent from the supplied snapshot set.
    UnknownSupersededRevision(DatasetRevisionId),
    /// Two venue entries reused the same venue identifier.
    DuplicateVenue(VenueId),
    /// A phase segment cited a revision absent from the supplied snapshot set.
    UnknownSegmentRevision(DatasetRevisionId),
}

impl fmt::Display for RulesetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for RulesetError {}
