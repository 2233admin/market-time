//! The decision path: instant plus rule data in, phase out.
//!
//! Nothing here reads a clock, a file, or a socket. `at` is always a parameter, the
//! ruleset is always a parameter, and this crate's dependency graph contains nothing
//! capable of doing otherwise — so a Principle IV violation fails to compile rather than
//! failing review.
//!
//! Civil-to-UTC conversion happens here, at query time, through jiff's
//! ambiguity-preserving path. A local time that occurs twice, or that does not exist at
//! all, becomes a value on the answer rather than a silent pick (FR-014).

use crate::coverage::CoverageGap;
use crate::event::{EventOccurrence, EventRule};
use crate::evidence::EvidenceRef;
use crate::ids::{DatasetRevisionId, VenueId};
use crate::instant::{Interval, UtcInstant};
use crate::phase::Phase;
use crate::query::{
    PhaseAnswer, PhaseBoundary, PhaseOutcome, Timeline, TimelineSegment, VenueOutcome,
};
use crate::rule::Rule;
use crate::ruleset::{Ruleset, VenueRuleset};
use crate::uncertainty::Uncertainty;
use jiff::Timestamp;
use jiff::civil::{Date, DateTime, Time};
use jiff::tz::{AmbiguousOffset, TimeZone};

/// The venue-local civil datetime for `at`.
///
/// Saturates at the representable calendar rather than panicking: an instant outside
/// jiff's range is far outside any venue's declared coverage, so the query it belongs to
/// is answered as unknown regardless.
#[must_use]
pub fn civil_datetime(zone: &TimeZone, at: UtcInstant) -> DateTime {
    let timestamp =
        Timestamp::from_nanosecond(at.as_nanos_since_unix_epoch()).unwrap_or_else(|_| {
            if at.as_nanos_since_unix_epoch() < 0 {
                Timestamp::MIN
            } else {
                Timestamp::MAX
            }
        });
    zone.to_datetime(timestamp)
}

/// What phase is `venue` in at `at`?
///
/// Returns [`PhaseOutcome::Unknown`] for an instant outside declared coverage, and for a
/// venue the ruleset has no entry for at all — a venue with no data has zero coverage,
/// which is an unknown about the query rather than an error about the call.
#[must_use]
pub fn resolve_phase(at: UtcInstant, venue: &VenueId, ruleset: &Ruleset) -> PhaseOutcome {
    let (Some(venue_rules), Some(zone)) = (ruleset.venue(venue), ruleset.zone(venue)) else {
        return unknown(venue, at, None, ruleset.revision_ids());
    };

    if !venue_rules.coverage.contains(at) {
        return unknown(
            venue,
            at,
            Some(venue_rules.coverage.clone()),
            ruleset.revision_ids(),
        );
    }

    let local = civil_datetime(zone, at);
    // A phase can start on the previous local day and run past midnight, so the day
    // before is a candidate too.
    let mut candidates = Vec::new();
    if let Ok(yesterday) = local.date().yesterday() {
        candidates.extend(day_segments(venue_rules, zone, yesterday));
    }
    candidates.extend(day_segments(venue_rules, zone, local.date()));
    if let Ok(tomorrow) = local.date().tomorrow() {
        candidates.extend(day_segments(venue_rules, zone, tomorrow));
    }

    candidates
        .into_iter()
        .find(|segment| segment.interval.contains(at))
        .map_or_else(
            || {
                unknown(
                    venue,
                    at,
                    Some(venue_rules.coverage.clone()),
                    ruleset.revision_ids(),
                )
            },
            |segment| {
                PhaseOutcome::Known(Box::new(segment.into_answer(
                    venue.clone(),
                    venue_rules,
                    zone,
                    ruleset.revision_ids(),
                )))
            },
        )
}

/// What phase is each of `venues` in at the same `at`?
///
/// One outcome per requested venue, in order. There is no batch-level error variant, so
/// one venue being outside coverage cannot suppress another's answer (FR-020). `at` is a
/// single value applied to every venue, so the response describes one instant rather than
/// a composite of moments read microseconds apart.
#[must_use]
pub fn resolve_phases(at: UtcInstant, venues: &[VenueId], ruleset: &Ruleset) -> Vec<VenueOutcome> {
    venues
        .iter()
        .map(|venue| VenueOutcome {
            venue: venue.clone(),
            outcome: resolve_phase(at, venue, ruleset),
        })
        .collect()
}

/// What phases does `venue` pass through across `interval`?
///
/// The returned segments tile `interval` exactly: no gaps, no overlaps, and a stretch
/// outside declared coverage is an explicit unknown segment rather than a hole. This is
/// what the board renders — position and width come from here, never from a schedule the
/// board holds itself.
#[must_use]
pub fn resolve_timeline(interval: Interval, venue: &VenueId, ruleset: &Ruleset) -> Timeline {
    let revisions = ruleset.revision_ids();
    let (Some(venue_rules), Some(zone)) = (ruleset.venue(venue), ruleset.zone(venue)) else {
        return Timeline {
            venue: venue.clone(),
            interval,
            segments: vec![TimelineSegment::Unknown {
                interval,
                gap: Box::new(gap(venue, interval.start, None, revisions)),
            }],
        };
    };

    let coverage = venue_rules.coverage.interval();
    let mut segments = Vec::new();

    if interval.start < coverage.start
        && let Ok(before) = Interval::new(interval.start, interval.end.min(coverage.start))
    {
        segments.push(TimelineSegment::Unknown {
            interval: before,
            gap: Box::new(gap(
                venue,
                before.start,
                Some(venue_rules.coverage.clone()),
                revisions.clone(),
            )),
        });
    }

    if let Some(covered) = interval.intersection(coverage) {
        let mut cursor = covered.start;
        let mut guard = 0_u32;
        while cursor < covered.end {
            guard += 1;
            if guard > MAX_TIMELINE_DAYS {
                break;
            }
            let date = civil_datetime(zone, cursor).date();
            let day = day_segments(venue_rules, zone, date);
            let mut advanced = false;
            for segment in day {
                let Some(clipped) = segment.interval.intersection(covered) else {
                    continue;
                };
                if clipped.end <= cursor {
                    continue;
                }
                let Ok(visible) = Interval::new(cursor.max(clipped.start), clipped.end) else {
                    continue;
                };
                segments.push(TimelineSegment::Phase {
                    interval: visible,
                    answer: Box::new(segment.clone().into_answer(
                        venue.clone(),
                        venue_rules,
                        zone,
                        revisions.clone(),
                    )),
                });
                cursor = visible.end;
                advanced = true;
            }
            if !advanced {
                // No segment covered the cursor: step to the next local day rather than
                // spinning. Only reachable at a daylight-saving edge where a whole local
                // day collapses, which no venue in the launch set has.
                let Ok(next) = date.tomorrow() else { break };
                let next_midnight = to_utc(zone, next.at(0, 0, 0, 0)).0;
                if next_midnight <= cursor {
                    break;
                }
                cursor = next_midnight.min(covered.end);
            }
        }
    }

    if interval.end > coverage.end
        && let Ok(after) = Interval::new(interval.start.max(coverage.end), interval.end)
    {
        segments.push(TimelineSegment::Unknown {
            interval: after,
            gap: Box::new(gap(
                venue,
                after.start,
                Some(venue_rules.coverage.clone()),
                revisions,
            )),
        });
    }

    Timeline {
        venue: venue.clone(),
        interval,
        segments: merge_adjacent(segments),
    }
}

/// Upper bound on the days one timeline call will walk, so a malformed zone cannot spin.
const MAX_TIMELINE_DAYS: u32 = 3_660;

#[derive(Debug, Clone)]
struct DaySegment {
    interval: Interval,
    phase: Phase,
    start_uncertainty: Uncertainty,
    end_uncertainty: Uncertainty,
    rule: Rule,
}

impl DaySegment {
    fn into_answer(
        self,
        venue: VenueId,
        venue_rules: &VenueRuleset,
        zone: &TimeZone,
        dataset_revisions: Vec<DatasetRevisionId>,
    ) -> PhaseAnswer {
        let uncertainty = self
            .start_uncertainty
            .clone()
            .widest(self.end_uncertainty.clone());
        let mut evidence: Vec<EvidenceRef> = vec![self.rule.evidence.clone()];
        evidence.extend(venue_rules.evidence.iter().cloned());

        PhaseAnswer {
            venue,
            phase: self.phase,
            boundary_start: PhaseBoundary {
                instant: self.interval.start,
                uncertainty: self.start_uncertainty,
            },
            boundary_end: PhaseBoundary {
                instant: self.interval.end,
                uncertainty: self.end_uncertainty,
            },
            events: events_in(&venue_rules.events, zone, self.interval),
            evidence,
            derived_reasoning: self
                .rule
                .derived
                .as_ref()
                .map(|note| note.reasoning().to_owned()),
            uncertainty,
            dataset_revisions,
        }
    }
}

fn day_segments(venue_rules: &VenueRuleset, zone: &TimeZone, date: Date) -> Vec<DaySegment> {
    let Some(rule) = governing_rule(&venue_rules.rules, date) else {
        return Vec::new();
    };

    let points = rule.schedule.change_points();
    let mut boundaries: Vec<(UtcInstant, Uncertainty)> = points
        .iter()
        .map(|(time, _)| to_utc_at(zone, date, *time, &rule.boundary_uncertainty))
        .collect();

    let next_midnight = match date.tomorrow() {
        Ok(next) => to_utc_at(zone, next, Time::midnight(), &rule.boundary_uncertainty),
        Err(_) => return Vec::new(),
    };
    boundaries.push(next_midnight);

    // Clamp to monotonic. A local span that a daylight-saving jump collapsed becomes
    // zero-length rather than negative, and zero-length spans are dropped below.
    for index in 1..boundaries.len() {
        if boundaries[index].0 < boundaries[index - 1].0 {
            boundaries[index].0 = boundaries[index - 1].0;
        }
    }

    points
        .iter()
        .enumerate()
        .filter_map(|(index, (_, phase))| {
            let (start, start_uncertainty) = boundaries[index].clone();
            let (end, end_uncertainty) = boundaries[index + 1].clone();
            let interval = Interval::new(start, end).ok()?;
            Some(DaySegment {
                interval,
                phase: *phase,
                start_uncertainty,
                end_uncertainty,
                rule: rule.clone(),
            })
        })
        .collect()
}

fn governing_rule(rules: &[Rule], date: Date) -> Option<&Rule> {
    rules
        .iter()
        .filter(|rule| rule.applies_on(date))
        .max_by_key(|rule| (rule.kind.precedence(), rule.applies.start))
}

fn to_utc_at(
    zone: &TimeZone,
    date: Date,
    time: Time,
    published: &Uncertainty,
) -> (UtcInstant, Uncertainty) {
    let (instant, resolution) = to_utc(zone, date.to_datetime(time));
    (instant, resolution.unwrap_or_else(|| published.clone()))
}

/// Converts a venue-local civil datetime to UTC, preserving what the conversion could not
/// settle.
///
/// The second element is `Some` only when the local time is ambiguous or nonexistent — in
/// which case it is the uncertainty that must be carried, because it is wider than any
/// publication granularity.
fn to_utc(zone: &TimeZone, dt: DateTime) -> (UtcInstant, Option<Uncertainty>) {
    let ambiguous = zone.to_ambiguous_timestamp(dt);
    match ambiguous.offset() {
        AmbiguousOffset::Unambiguous { offset } => (instant_of(offset, dt), None),
        AmbiguousOffset::Fold { before, after } => {
            let earlier = instant_of(before, dt);
            let later = instant_of(after, dt);
            let (earlier, later) = if earlier <= later {
                (earlier, later)
            } else {
                (later, earlier)
            };
            (
                earlier,
                Some(Uncertainty::AmbiguousLocalTime { earlier, later }),
            )
        }
        AmbiguousOffset::Gap { before, after } => {
            let as_if_before_shift = instant_of(before, dt);
            let as_if_after_shift = instant_of(after, dt);
            (
                as_if_before_shift.max(as_if_after_shift),
                Some(Uncertainty::NonexistentLocalTime {
                    as_if_before_shift,
                    as_if_after_shift,
                }),
            )
        }
    }
}

fn instant_of(offset: jiff::tz::Offset, dt: DateTime) -> UtcInstant {
    offset.to_timestamp(dt).map_or_else(
        |_| UtcInstant::from_nanos_since_unix_epoch(0),
        |timestamp| UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond()),
    )
}

fn events_in(rules: &[EventRule], zone: &TimeZone, interval: Interval) -> Vec<EventOccurrence> {
    let mut occurrences = Vec::new();
    let mut date = civil_datetime(zone, interval.start).date();
    let last = civil_datetime(zone, interval.end).date();

    loop {
        for rule in rules.iter().filter(|rule| rule.applies.contains(date)) {
            for time in &rule.times {
                let (instant, resolution) = to_utc(zone, date.to_datetime(*time));
                if interval.contains(instant) {
                    occurrences.push(EventOccurrence {
                        kind: rule.kind,
                        instant,
                        uncertainty: resolution.unwrap_or_else(|| rule.uncertainty.clone()),
                        evidence: vec![rule.evidence.clone()],
                    });
                }
            }
        }
        if date >= last {
            break;
        }
        match date.tomorrow() {
            Ok(next) => date = next,
            Err(_) => break,
        }
    }

    occurrences.sort_by_key(|occurrence| occurrence.instant);
    occurrences
}

/// Joins consecutive segments that describe the same phase under the same rule.
///
/// An always-on venue is one segment across a week, not seven daily segments that happen
/// to touch.
fn merge_adjacent(segments: Vec<TimelineSegment>) -> Vec<TimelineSegment> {
    let mut merged: Vec<TimelineSegment> = Vec::with_capacity(segments.len());

    for segment in segments {
        let Some(previous) = merged.last_mut() else {
            merged.push(segment);
            continue;
        };

        let joinable = match (&previous, &segment) {
            (
                TimelineSegment::Phase {
                    interval: left,
                    answer: left_answer,
                },
                TimelineSegment::Phase {
                    interval: right,
                    answer: right_answer,
                },
            ) => {
                left.end == right.start
                    && left_answer.phase == right_answer.phase
                    && left_answer.evidence == right_answer.evidence
            }
            (
                TimelineSegment::Unknown { interval: left, .. },
                TimelineSegment::Unknown {
                    interval: right, ..
                },
            ) => left.end == right.start,
            _ => false,
        };

        if !joinable {
            merged.push(segment);
            continue;
        }

        match (previous, segment) {
            (
                TimelineSegment::Phase {
                    interval: left,
                    answer: left_answer,
                },
                TimelineSegment::Phase {
                    interval: right,
                    answer: right_answer,
                },
            ) => {
                left.end = right.end;
                left_answer.boundary_end = right_answer.boundary_end;
                left_answer.events.extend(right_answer.events);
                left_answer.uncertainty = left_answer
                    .uncertainty
                    .clone()
                    .widest(right_answer.uncertainty);
            }
            (
                TimelineSegment::Unknown { interval: left, .. },
                TimelineSegment::Unknown {
                    interval: right, ..
                },
            ) => {
                left.end = right.end;
            }
            _ => unreachable!("joinable pairs are matched above"),
        }
    }

    merged
}

fn unknown(
    venue: &VenueId,
    at: UtcInstant,
    coverage: Option<crate::coverage::CoverageRange>,
    revisions: Vec<DatasetRevisionId>,
) -> PhaseOutcome {
    PhaseOutcome::Unknown(Box::new(gap(venue, at, coverage, revisions)))
}

fn gap(
    venue: &VenueId,
    at: UtcInstant,
    coverage: Option<crate::coverage::CoverageRange>,
    dataset_revisions: Vec<DatasetRevisionId>,
) -> CoverageGap {
    CoverageGap {
        venue: venue.clone(),
        queried_at: at,
        coverage,
        dataset_revisions,
    }
}
