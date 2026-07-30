//! Reading a dataset and handing the core materialized values.
//!
//! This is the only place in the workspace that opens a file. What it produces is a
//! validated [`Ruleset`]; what it never produces is a handle, a path, or a lazily-loaded
//! anything that the core could accidentally touch.

use crate::format::{
    ChangePointRecord, DatasetFile, DateRangeRecord, EvidenceRecord, RuleKindRecord, RuleRecord,
    UncertaintyRecord, VenueRecord,
};
use jiff::civil::{Date, Time, Weekday};
use market_time_core::CoverageRange;
use market_time_core::Phase;
use market_time_core::Uncertainty;
use market_time_core::{AssetFamily, VenueProfile};
use market_time_core::{CivilDaySchedule, DateRange, Rule, RuleKind};
use market_time_core::{DatasetRevision, Ruleset, VenueRuleset};
use market_time_core::{DatasetRevisionId, IanaZoneId, IdentifierError, VenueId};
use market_time_core::{DerivationNote, EvidenceRef};
use market_time_core::{EventKind, EventRule};
use market_time_core::{Interval, UtcInstant};
use std::fmt;
use std::path::Path;

/// Reads a dataset file and builds a validated ruleset.
///
/// # Errors
///
/// Returns [`LoadError`] when the file cannot be read, is not valid JSON in the dataset
/// format, names a phase outside the shared vocabulary, or fails the core's structural
/// validation. All of that happens here, before any query is asked.
pub fn load_ruleset(path: &Path) -> Result<Ruleset, LoadError> {
    let text = std::fs::read_to_string(path).map_err(|source| LoadError::Unreadable {
        path: path.display().to_string(),
        source: source.to_string(),
    })?;
    parse_ruleset(&text)
}

/// Builds a validated ruleset from dataset JSON already in memory.
///
/// # Errors
///
/// As [`load_ruleset`], minus the file read.
pub fn parse_ruleset(text: &str) -> Result<Ruleset, LoadError> {
    let file: DatasetFile =
        serde_json::from_str(text).map_err(|source| LoadError::Malformed(source.to_string()))?;

    let revisions = file
        .revisions
        .iter()
        .map(|record| {
            Ok(DatasetRevision {
                id: identifier(DatasetRevisionId::new(&record.id))?,
                supersedes: record
                    .supersedes
                    .as_deref()
                    .map(|value| identifier(DatasetRevisionId::new(value)))
                    .transpose()?,
                iana_tzdb_version: record.iana_tzdb_version.clone(),
                assembled_at: instant(&record.assembled_at)?,
            })
        })
        .collect::<Result<Vec<_>, LoadError>>()?;

    let venues = file
        .venues
        .iter()
        .map(venue_ruleset)
        .collect::<Result<Vec<_>, LoadError>>()?;

    Ruleset::from_parts(revisions, venues).map_err(|source| LoadError::Invalid(source.to_string()))
}

fn venue_ruleset(record: &VenueRecord) -> Result<VenueRuleset, LoadError> {
    let coverage = CoverageRange::new(
        Interval::new(
            instant(&record.coverage.start)?,
            instant(&record.coverage.end)?,
        )
        .map_err(|source| LoadError::Invalid(source.to_string()))?,
    );

    Ok(VenueRuleset {
        venue: identifier(VenueId::new(&record.venue))?,
        home_zone: identifier(IanaZoneId::new(&record.home_zone))?,
        profile: VenueProfile {
            display_name: record.display_name.clone(),
            location: record.location.clone(),
            family: record
                .family
                .as_deref()
                .map(|family| {
                    AssetFamily::from_str_exact(family).ok_or_else(|| LoadError::UnknownFamily {
                        family: family.to_owned(),
                    })
                })
                .transpose()?,
        },
        coverage,
        rules: record
            .rules
            .iter()
            .map(rule)
            .collect::<Result<Vec<_>, LoadError>>()?,
        events: record
            .events
            .iter()
            .map(|event| {
                Ok(EventRule {
                    kind: EventKind::from_str_exact(&event.kind).ok_or_else(|| {
                        LoadError::UnknownEventKind {
                            kind: event.kind.clone(),
                        }
                    })?,
                    times: event
                        .times
                        .iter()
                        .map(|text| time(text))
                        .collect::<Result<Vec<_>, LoadError>>()?,
                    applies: date_range(&event.applies)?,
                    uncertainty: uncertainty(&event.uncertainty),
                    evidence: evidence(&event.evidence)?,
                    revision: identifier(DatasetRevisionId::new(&event.revision))?,
                })
            })
            .collect::<Result<Vec<_>, LoadError>>()?,
        evidence: record
            .evidence
            .iter()
            .map(evidence)
            .collect::<Result<Vec<_>, LoadError>>()?,
    })
}

fn rule(record: &RuleRecord) -> Result<Rule, LoadError> {
    let change_points = record
        .schedule
        .iter()
        .map(change_point)
        .collect::<Result<Vec<_>, LoadError>>()?;

    Ok(Rule {
        kind: rule_kind(&record.kind)?,
        schedule: CivilDaySchedule::new(change_points)
            .map_err(|source| LoadError::Invalid(source.to_string()))?,
        applies: date_range(&record.applies)?,
        boundary_uncertainty: uncertainty(&record.boundary_uncertainty),
        evidence: evidence(&record.evidence)?,
        derived: record
            .derived_reasoning
            .as_deref()
            .map(DerivationNote::new)
            .transpose()
            .map_err(|source| LoadError::Invalid(source.to_string()))?,
        revision: identifier(DatasetRevisionId::new(&record.revision))?,
    })
}

fn rule_kind(record: &RuleKindRecord) -> Result<RuleKind, LoadError> {
    Ok(match record {
        RuleKindRecord::WeeklyPattern { weekdays } => RuleKind::WeeklyPattern {
            weekdays: weekdays
                .iter()
                .map(|day| weekday(day))
                .collect::<Result<Vec<_>, LoadError>>()?,
        },
        RuleKindRecord::Holiday { name } => RuleKind::Holiday { name: name.clone() },
        RuleKindRecord::ShortenedSession { name } => {
            RuleKind::ShortenedSession { name: name.clone() }
        }
        RuleKindRecord::AnnouncedChange { note } => {
            RuleKind::AnnouncedChange { note: note.clone() }
        }
    })
}

fn change_point(record: &ChangePointRecord) -> Result<(Time, Phase), LoadError> {
    let phase = Phase::from_str_exact(&record.phase).ok_or_else(|| LoadError::UnknownPhase {
        phase: record.phase.clone(),
    })?;
    Ok((time(&record.at)?, phase))
}

fn uncertainty(record: &UncertaintyRecord) -> Uncertainty {
    match record {
        UncertaintyRecord::Exact => Uncertainty::Exact,
        UncertaintyRecord::PublishedGranularity { seconds } => {
            Uncertainty::seconds(i128::from(*seconds))
        }
        UncertaintyRecord::PublishedBound {
            seconds,
            published_as,
        } => Uncertainty::PublishedBound {
            nanos: i128::from(*seconds) * market_time_core::NANOS_PER_SECOND,
            published_as: published_as.clone(),
        },
        UncertaintyRecord::ProcessStart { process } => Uncertainty::ProcessStart {
            process: process.clone(),
        },
    }
}

fn evidence(record: &EvidenceRecord) -> Result<EvidenceRef, LoadError> {
    let reference = EvidenceRef::new(
        &record.source_url,
        instant(&record.fetched_at)?,
        &record.effective_from,
    )
    .map_err(|source| LoadError::Invalid(source.to_string()))?;

    Ok(match &record.publisher_last_changed {
        Some(when) => reference.with_publisher_last_changed(when),
        None => reference,
    })
}

fn date_range(record: &DateRangeRecord) -> Result<DateRange, LoadError> {
    DateRange::new(date(&record.start)?, date(&record.end)?)
        .map_err(|source| LoadError::Invalid(source.to_string()))
}

/// Turns an identifier rejection into a load error, so a blank id in a dataset fails the
/// load rather than travelling on as provenance that names nothing.
fn identifier<T>(result: Result<T, IdentifierError>) -> Result<T, LoadError> {
    result.map_err(|error| LoadError::Invalid(error.to_string()))
}

fn instant(text: &str) -> Result<UtcInstant, LoadError> {
    let timestamp: jiff::Timestamp = text.parse().map_err(|_| LoadError::BadInstant {
        value: text.to_owned(),
    })?;
    Ok(UtcInstant::from_nanos_since_unix_epoch(
        timestamp.as_nanosecond(),
    ))
}

fn date(text: &str) -> Result<Date, LoadError> {
    text.parse().map_err(|_| LoadError::BadDate {
        value: text.to_owned(),
    })
}

fn time(text: &str) -> Result<Time, LoadError> {
    let padded = if text.len() == 5 {
        format!("{text}:00")
    } else {
        text.to_owned()
    };
    padded.parse().map_err(|_| LoadError::BadTime {
        value: text.to_owned(),
    })
}

fn weekday(text: &str) -> Result<Weekday, LoadError> {
    Ok(match text.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Weekday::Monday,
        "tue" | "tuesday" => Weekday::Tuesday,
        "wed" | "wednesday" => Weekday::Wednesday,
        "thu" | "thursday" => Weekday::Thursday,
        "fri" | "friday" => Weekday::Friday,
        "sat" | "saturday" => Weekday::Saturday,
        "sun" | "sunday" => Weekday::Sunday,
        _ => {
            return Err(LoadError::BadWeekday {
                value: text.to_owned(),
            });
        }
    })
}

/// Why a dataset could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The file could not be read.
    Unreadable {
        /// The path that was tried.
        path: String,
        /// What the operating system said.
        source: String,
    },
    /// The file is not valid JSON in the dataset format.
    Malformed(String),
    /// The data is well-formed but does not describe a valid ruleset.
    Invalid(String),
    /// A phase name outside the shared vocabulary.
    UnknownPhase {
        /// The name the dataset used.
        phase: String,
    },
    /// An asset family outside the closed set.
    UnknownFamily {
        /// The name the dataset used.
        family: String,
    },
    /// An event kind outside the shared vocabulary.
    UnknownEventKind {
        /// The name the dataset used.
        kind: String,
    },
    /// An instant that is not RFC 3339.
    BadInstant {
        /// The value that could not be parsed.
        value: String,
    },
    /// A date that is not `YYYY-MM-DD`.
    BadDate {
        /// The value that could not be parsed.
        value: String,
    },
    /// A time of day that is not `HH:MM` or `HH:MM:SS`.
    BadTime {
        /// The value that could not be parsed.
        value: String,
    },
    /// A weekday name that is not recognised.
    BadWeekday {
        /// The value that could not be parsed.
        value: String,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, source } => write!(f, "cannot read {path}: {source}"),
            Self::Malformed(detail) => write!(f, "dataset is not in the expected format: {detail}"),
            Self::Invalid(detail) => {
                write!(f, "dataset does not describe a valid ruleset: {detail}")
            }
            Self::UnknownPhase { phase } => write!(
                f,
                "dataset uses phase {phase:?}, which is not in the shared vocabulary; \
                 a venue may not introduce a phase name of its own"
            ),
            Self::UnknownFamily { family } => write!(
                f,
                "dataset uses asset family {family:?}, which is not one of equities,                  spot_and_fx, or futures"
            ),
            Self::UnknownEventKind { kind } => {
                write!(
                    f,
                    "dataset uses event kind {kind:?}, which is not in the vocabulary"
                )
            }
            Self::BadInstant { value } => write!(f, "{value:?} is not an RFC 3339 instant"),
            Self::BadDate { value } => write!(f, "{value:?} is not a YYYY-MM-DD date"),
            Self::BadTime { value } => write!(f, "{value:?} is not an HH:MM time of day"),
            Self::BadWeekday { value } => write!(f, "{value:?} is not a weekday name"),
        }
    }
}

impl std::error::Error for LoadError {}
