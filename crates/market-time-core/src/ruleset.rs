//! Rule data, materialized in memory, validated once.
//!
//! The core performs no I/O: a [`Ruleset`] is built from owned values a shell already
//! read (Principle IV). Structural validation happens here, at construction — so the
//! query path stays total, and "unknown versus error" stays a single line: unknown is
//! about the query, an error is about the ruleset being invalid before any query ran.

use crate::coverage::CoverageRange;
use crate::event::EventRule;
use crate::evidence::EvidenceRef;
use crate::ids::{DatasetRevisionId, IanaZoneId, VenueId};
use crate::instant::UtcInstant;
use crate::rule::Rule;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use std::collections::BTreeMap;
use std::fmt;

/// An immutable, identified version of a body of rule data.
///
/// Corrections produce a new revision that supersedes the old one; a revision is never
/// edited in place (Principle III).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetRevision {
    /// The revision identifier.
    pub id: DatasetRevisionId,
    /// The revision this one replaces, where it replaces one.
    pub supersedes: Option<DatasetRevisionId>,
    /// The IANA tzdb release the data was prepared against, where recorded.
    pub iana_tzdb_version: Option<String>,
    /// When the revision was assembled.
    pub assembled_at: UtcInstant,
}

/// One venue's rules, zone, and coverage declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueRuleset {
    /// The venue these rules describe.
    pub venue: VenueId,
    /// The venue's home zone, e.g. `"Asia/Shanghai"`.
    pub home_zone: IanaZoneId,
    /// What this data is willing to answer for.
    pub coverage: CoverageRange,
    /// The rules themselves.
    pub rules: Vec<Rule>,
    /// Rules for scheduled occurrences that are not phases (FR-007). May be empty.
    pub events: Vec<EventRule>,
    /// Venue-level evidence, e.g. the page the venue publishes its calendar on.
    pub evidence: Vec<EvidenceRef>,
}

/// A validated body of rule data.
#[derive(Debug, Clone)]
pub struct Ruleset {
    revisions: Vec<DatasetRevision>,
    venues: BTreeMap<VenueId, VenueRuleset>,
    zones: BTreeMap<VenueId, TimeZone>,
}

impl Ruleset {
    /// Validates and takes ownership of rule data.
    ///
    /// # Errors
    ///
    /// Returns [`RulesetError`] when a venue is declared twice, a zone is unknown to the
    /// bundled tzdb, a rule cites a revision that is not present, a rule applies outside
    /// its venue's coverage, or a date inside coverage has no rule at all. That last one
    /// is the tiling invariant (FR-008) enforced end to end: coverage that promises an
    /// answer the rules cannot give is rejected before any query is asked.
    pub fn from_parts(
        revisions: Vec<DatasetRevision>,
        venues: Vec<VenueRuleset>,
    ) -> Result<Self, RulesetError> {
        let revision_ids: Vec<&DatasetRevisionId> = revisions.iter().map(|r| &r.id).collect();

        let mut by_venue = BTreeMap::new();
        let mut zones = BTreeMap::new();

        for venue_ruleset in venues {
            let venue = venue_ruleset.venue.clone();
            if by_venue.contains_key(&venue) {
                return Err(RulesetError::DuplicateVenue { venue });
            }

            let zone = TimeZone::get(venue_ruleset.home_zone.as_str()).map_err(|_| {
                RulesetError::UnknownZone {
                    venue: venue.clone(),
                    zone: venue_ruleset.home_zone.clone(),
                }
            })?;

            for revision in venue_ruleset
                .rules
                .iter()
                .map(|r| &r.revision)
                .chain(venue_ruleset.events.iter().map(|e| &e.revision))
            {
                if !revision_ids.contains(&revision) {
                    return Err(RulesetError::UnknownRevision {
                        venue: venue.clone(),
                        revision: revision.clone(),
                    });
                }
            }

            let first_covered = local_date(&zone, venue_ruleset.coverage.start());
            let last_covered =
                local_date(&zone, venue_ruleset.coverage.end().saturating_add_nanos(-1));

            for rule in &venue_ruleset.rules {
                if rule.applies.start < first_covered || rule.applies.end > last_covered {
                    return Err(RulesetError::RuleOutsideCoverage {
                        venue: venue.clone(),
                        rule: rule.kind.label().to_owned(),
                    });
                }
            }

            let mut date = first_covered;
            loop {
                if !venue_ruleset.rules.iter().any(|r| r.applies_on(date)) {
                    return Err(RulesetError::UncoveredDate {
                        venue: venue.clone(),
                        date,
                    });
                }
                if date >= last_covered {
                    break;
                }
                date = date
                    .tomorrow()
                    .map_err(|_| RulesetError::CoverageOutOfCalendarRange {
                        venue: venue.clone(),
                    })?;
            }

            zones.insert(venue.clone(), zone);
            by_venue.insert(venue, venue_ruleset);
        }

        Ok(Self {
            revisions,
            venues: by_venue,
            zones,
        })
    }

    /// The venues this ruleset knows about, in identifier order.
    #[must_use]
    pub fn venues(&self) -> Vec<VenueId> {
        self.venues.keys().cloned().collect()
    }

    /// One venue's rules, or `None` when the ruleset has no entry for it.
    #[must_use]
    pub fn venue(&self, venue: &VenueId) -> Option<&VenueRuleset> {
        self.venues.get(venue)
    }

    /// One venue's home zone, resolved once at construction.
    #[must_use]
    pub fn zone(&self, venue: &VenueId) -> Option<&TimeZone> {
        self.zones.get(venue)
    }

    /// Every revision in this ruleset.
    #[must_use]
    pub fn revisions(&self) -> &[DatasetRevision] {
        &self.revisions
    }

    /// The revision identifiers, in order.
    #[must_use]
    pub fn revision_ids(&self) -> Vec<DatasetRevisionId> {
        self.revisions.iter().map(|r| r.id.clone()).collect()
    }
}

fn local_date(zone: &TimeZone, at: UtcInstant) -> Date {
    crate::resolve::civil_datetime(zone, at).date()
}

/// Why a ruleset could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesetError {
    /// The same venue was supplied twice.
    DuplicateVenue {
        /// The repeated venue.
        venue: VenueId,
    },
    /// The venue's home zone is not in the bundled tzdb.
    UnknownZone {
        /// The venue.
        venue: VenueId,
        /// The zone that could not be resolved.
        zone: IanaZoneId,
    },
    /// A rule cites a revision that was not supplied.
    UnknownRevision {
        /// The venue.
        venue: VenueId,
        /// The revision the rule cited.
        revision: DatasetRevisionId,
    },
    /// A rule applies to dates outside its venue's declared coverage.
    RuleOutsideCoverage {
        /// The venue.
        venue: VenueId,
        /// The rule, by label.
        rule: String,
    },
    /// A date inside coverage has no rule, so coverage promises more than the rules give.
    UncoveredDate {
        /// The venue.
        venue: VenueId,
        /// The date with no applicable rule.
        date: Date,
    },
    /// Coverage extends past the representable calendar.
    CoverageOutOfCalendarRange {
        /// The venue.
        venue: VenueId,
    },
}

impl fmt::Display for RulesetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateVenue { venue } => write!(f, "venue {venue} was supplied twice"),
            Self::UnknownZone { venue, zone } => {
                write!(f, "venue {venue} declares unknown zone {zone}")
            }
            Self::UnknownRevision { venue, revision } => {
                write!(f, "venue {venue} cites unknown revision {revision}")
            }
            Self::RuleOutsideCoverage { venue, rule } => write!(
                f,
                "venue {venue} has a rule ({rule}) applying outside its declared coverage"
            ),
            Self::UncoveredDate { venue, date } => write!(
                f,
                "venue {venue} declares coverage over {date} but has no rule for it"
            ),
            Self::CoverageOutOfCalendarRange { venue } => {
                write!(
                    f,
                    "venue {venue} declares coverage past the representable calendar"
                )
            }
        }
    }
}

impl std::error::Error for RulesetError {}
