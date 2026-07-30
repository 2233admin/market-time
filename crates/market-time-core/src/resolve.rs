//! Total phase resolution over an already validated in-memory ruleset.

use crate::{
    CoverageRange, DatasetRevisionId, EvidenceRef, Phase, PhaseBoundary, Ruleset, Uncertainty,
    UtcInstant, VenueId,
};

/// Resolves one venue's phase at one explicit UTC instant.
///
/// Data gaps are ordinary [`PhaseOutcome::Unknown`] values. Malformed rule data cannot
/// reach this function because [`Ruleset::from_parts`](Ruleset::from_parts) validates it
/// once at construction.
#[must_use]
pub fn resolve_phase(at: UtcInstant, venue: VenueId, ruleset: &Ruleset) -> PhaseOutcome {
    let Some(venue_ruleset) = ruleset.venue(&venue) else {
        return PhaseOutcome::Unknown(CoverageGap {
            venue,
            queried_at: at,
            coverage: None,
            dataset_revisions: ruleset.revision_ids(),
        });
    };
    let coverage = venue_ruleset.timeline.coverage();
    if !coverage.contains(at) {
        return PhaseOutcome::Unknown(CoverageGap {
            venue,
            queried_at: at,
            coverage: Some(coverage),
            dataset_revisions: revisions_for_timeline(venue_ruleset.timeline.segments()),
        });
    }

    let Some(segment) = venue_ruleset.timeline.segment_at(at) else {
        return PhaseOutcome::Unknown(CoverageGap {
            venue,
            queried_at: at,
            coverage: Some(coverage),
            dataset_revisions: revisions_for_timeline(venue_ruleset.timeline.segments()),
        });
    };
    let boundary_start = segment.boundary_start().clone();
    let boundary_end = segment.boundary_end().clone();
    PhaseOutcome::Known(PhaseAnswer {
        venue,
        phase: segment.phase(),
        uncertainty: Uncertainty::combine(&boundary_start.uncertainty, &boundary_end.uncertainty),
        boundary_start,
        boundary_end,
        evidence: segment.evidence().to_vec(),
        dataset_revisions: segment.dataset_revisions().to_vec(),
    })
}

/// A phase query's total, three-way semantic outcome: known phase or explicit unknown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhaseOutcome {
    /// Rule data covers the instant and produced an attributable phase.
    Known(PhaseAnswer),
    /// Rule data does not cover the instant or venue.
    Unknown(CoverageGap),
}

/// A complete known answer returned by the decision core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseAnswer {
    /// Venue whose phase was resolved.
    pub venue: VenueId,
    /// Shared-vocabulary phase.
    pub phase: Phase,
    /// Inclusive phase start.
    pub boundary_start: PhaseBoundary,
    /// Exclusive phase end.
    pub boundary_end: PhaseBoundary,
    /// Evidence references that justify the segment and its boundaries.
    pub evidence: Vec<EvidenceRef>,
    /// Answer-level uncertainty, conservatively combined from both boundaries.
    pub uncertainty: Uncertainty,
    /// Immutable revisions that produced the answer.
    pub dataset_revisions: Vec<DatasetRevisionId>,
}

/// Explicit information about why a query could not be answered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageGap {
    /// Venue that was queried.
    pub venue: VenueId,
    /// Exact instant that was queried.
    pub queried_at: UtcInstant,
    /// Declared range crossed by the query, or `None` when the venue itself is unknown.
    pub coverage: Option<CoverageRange>,
    /// Loaded immutable revisions available when the gap was reported.
    pub dataset_revisions: Vec<DatasetRevisionId>,
}

fn revisions_for_timeline(segments: &[crate::PhaseSegment]) -> Vec<DatasetRevisionId> {
    let mut revisions = Vec::new();
    for segment in segments {
        for revision in segment.dataset_revisions() {
            if !revisions.contains(revision) {
                revisions.push(revision.clone());
            }
        }
    }
    revisions
}
