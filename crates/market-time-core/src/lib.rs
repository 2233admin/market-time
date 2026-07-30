//! Mark Time — pure phase-resolution core.
//!
//! Given an instant and a venue's ruleset, answer what trading phase that venue is in,
//! with the evidence behind the answer and an honest statement of how precisely it is
//! known.
//!
//! # What this crate will not do
//!
//! Constitution Principle IV: **no I/O, no network, and no clock reads in the decision
//! path.** Instants are passed in; rule data is passed in. That is what makes golden
//! vectors replay deterministically, and it is enforced by the dependency graph rather
//! than by review — see `tests/contract/no_io_no_clock.rs`.
//!
//! Loading rule data is [`market-time-data`]'s job. Converting a non-UTC instant is
//! [`market-time-scales`]' job, and that conversion is leap-second-aware exactly once,
//! at that boundary (research D1a).
//!
//! [`market-time-data`]: https://github.com/2233admin/market-time
//! [`market-time-scales`]: https://github.com/2233admin/market-time

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

// Modules are private and the public surface is the re-export list below. Consumers
// name a type, never a path through this crate's internals — so moving a type between
// modules is not a breaking change, and nothing outside can reach a helper that was
// never meant to be part of the contract.
mod coverage;
mod event;
mod evidence;
mod ids;
mod instant;
mod phase;
mod query;
mod resolve;
mod rule;
mod ruleset;
mod uncertainty;

pub mod tzdata;

pub use coverage::{CoverageGap, CoverageRange};
pub use event::{EventKind, EventOccurrence, EventRule};
pub use evidence::{DerivationNote, EvidenceError, EvidenceRef};
pub use ids::{DatasetRevisionId, IanaZoneId, IdentifierError, VenueId};
pub use instant::{Interval, IntervalError, NANOS_PER_SECOND, UtcInstant};
pub use phase::Phase;
pub use query::{
    PhaseAnswer, PhaseBoundary, PhaseOutcome, Timeline, TimelineSegment, VenueOutcome,
};
pub use resolve::{resolve_phase, resolve_phases, resolve_timeline};
pub use rule::{CivilDaySchedule, DateRange, Rule, RuleError, RuleKind};
pub use ruleset::{DatasetRevision, Ruleset, RulesetError, VenueRuleset};
pub use uncertainty::Uncertainty;
