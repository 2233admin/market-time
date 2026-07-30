//! Reading Shanghai Stock Exchange's published session table into rules.
//!
//! # What is here and what is deliberately not
//!
//! This module contains a **parser and a mapping**. It contains no session times. SSE's
//! Trading Rules state that use or publication of their material requires the Exchange's
//! permission (research D6a), so the schedule stays in the operator's fetched document and
//! is read out of it at assembly time. What this repository contributes is the reading:
//! which of SSE's published session names corresponds to which phase in the shared
//! vocabulary, and the refusal to invent anything the document does not say.
//!
//! # The refusal is the point
//!
//! SSE's session table does not tile the trading day. Two intervals sit between published
//! sessions, and the obvious guesses for them are plausible and unsourced — exactly what
//! Principle I forbids. [`day_schedule`] will not fill a gap. It returns
//! [`SseError::UnassignedIntervals`] naming each one, and the caller must supply a
//! [`GapRuling`] carrying the reasoning that settles it. Both SSE gaps were settled against
//! the Trading Rules and the reasoning is recorded in `docs/venue-session-state/research.md`
//! under D4b and D4c; it is passed in rather than compiled in, because it is a reading of a
//! document this repository does not carry.

use crate::fetch::FetchedDocument;
use crate::format::{ChangePointRecord, EvidenceRecord};
use crate::revision::evidence_from;
use jiff::civil::Time;
use market_time_core::Phase;
use std::fmt;

/// One published session, as the document names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedSession {
    /// The venue's own label for the session, kept verbatim.
    pub label: String,
    /// The phase it maps to in the shared vocabulary.
    pub phase: Phase,
    /// Local start, inclusive.
    pub start: Time,
    /// Local end, exclusive.
    pub end: Time,
}

/// A ruling on an interval the published table leaves unassigned.
///
/// The reasoning is required, and it ends up on the rule as a derivation note. An interval
/// settled "because it is obvious" is not settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapRuling {
    /// Start of the interval this ruling covers.
    pub start: Time,
    /// End of the interval.
    pub end: Time,
    /// The phase it takes.
    pub phase: Phase,
    /// Why, in enough detail that a reader can check it against the source.
    pub reasoning: String,
}

/// How SSE's session labels map to the shared vocabulary.
///
/// This table is our reading, not the venue's wording, and it is the whole of what this
/// module asserts about SSE. Matching is by substring so a label may carry qualifiers the
/// venue adds around it.
const LABEL_MAPPING: [(&str, Phase); 7] = [
    ("开盘集合竞价", Phase::OpeningAuction),
    ("收盘集合竞价", Phase::ClosingAuction),
    ("连续竞价", Phase::ContinuousTrading),
    ("盘后固定价格交易", Phase::PostClose),
    ("午间休市", Phase::MidDayBreak),
    ("中午休市", Phase::MidDayBreak),
    ("集合竞价", Phase::OpeningAuction),
];

/// Reads published sessions out of a fetched document.
///
/// Accepts the forms SSE's publications use for a time range on a line that also names the
/// session: `9:15 至 9:25`, `9:15-9:25`, `09:15–09:25`. A line may carry several ranges, as
/// the continuous-auction line does.
///
/// # Errors
///
/// Returns [`SseError::NotText`] when the document is not UTF-8, [`SseError::NoSessions`]
/// when nothing recognisable was found — which means the document changed shape and a
/// human needs to look, not that SSE stopped trading — and [`SseError::Overlapping`] when
/// two parsed sessions overlap.
pub fn parse_sessions(document: &FetchedDocument) -> Result<Vec<PublishedSession>, SseError> {
    let text = document.text().map_err(|_| SseError::NotText)?;
    let mut sessions = Vec::new();

    for line in text.lines() {
        let Some((label, phase)) = LABEL_MAPPING
            .iter()
            .find(|(label, _)| line.contains(label))
            .map(|(label, phase)| ((*label).to_owned(), *phase))
        else {
            continue;
        };

        for (start, end) in time_ranges(line) {
            sessions.push(PublishedSession {
                label: label.clone(),
                phase,
                start,
                end,
            });
        }
    }

    if sessions.is_empty() {
        return Err(SseError::NoSessions);
    }

    sessions.sort_by_key(|session| session.start);
    for pair in sessions.windows(2) {
        if pair[1].start < pair[0].end {
            return Err(SseError::Overlapping {
                first: pair[0].label.clone(),
                second: pair[1].label.clone(),
            });
        }
    }

    Ok(sessions)
}

/// Turns published sessions plus rulings into a day schedule.
///
/// The day is closed before the first session and after the last, which is a statement
/// about the venue being shut rather than a guess about a gap between sessions.
///
/// # Errors
///
/// Returns [`SseError::UnassignedIntervals`] when an interval between two published
/// sessions has no ruling. That is the invariant doing its job: SSE's table genuinely does
/// not tile the day, and filling the hole quietly is the failure this design exists to
/// prevent.
pub fn day_schedule(
    sessions: &[PublishedSession],
    rulings: &[GapRuling],
) -> Result<Vec<ChangePointRecord>, SseError> {
    let mut assigned: Vec<(Time, Time, Phase)> = sessions
        .iter()
        .map(|session| (session.start, session.end, session.phase))
        .collect();

    let mut unassigned = Vec::new();
    for pair in sessions.windows(2) {
        let (gap_start, gap_end) = (pair[0].end, pair[1].start);
        if gap_start >= gap_end {
            continue;
        }
        match rulings
            .iter()
            .find(|ruling| ruling.start <= gap_start && ruling.end >= gap_end)
        {
            Some(ruling) => assigned.push((gap_start, gap_end, ruling.phase)),
            None => unassigned.push(format!("{gap_start}–{gap_end}")),
        }
    }

    if !unassigned.is_empty() {
        return Err(SseError::UnassignedIntervals {
            intervals: unassigned,
        });
    }

    assigned.sort_by_key(|(start, _, _)| *start);

    // Every session opens a phase at its start and closes it at its end. Where one
    // session ends exactly where the next begins, both land on the same instant and the
    // opening wins — a phase that ended is not a phase the venue is in.
    let mut transitions: Vec<(Time, Phase, bool)> = vec![(Time::midnight(), Phase::Closed, false)];
    for (start, end, phase) in assigned {
        transitions.push((start, phase, true));
        transitions.push((end, Phase::Closed, false));
    }
    transitions.sort_by_key(|(at, _, opening)| (*at, !*opening));
    transitions.dedup_by_key(|(at, _, _)| *at);

    let mut change_points: Vec<ChangePointRecord> = transitions
        .into_iter()
        .map(|(at, phase, _)| ChangePointRecord {
            at: format_time(at),
            phase: phase.as_str().to_owned(),
        })
        .collect();

    // Consecutive identical phases would be a boundary that is not a boundary.
    change_points.dedup_by(|later, earlier| later.phase == earlier.phase);
    Ok(change_points)
}

/// The rulings that were applied, as derivation reasoning for the rule.
///
/// A rule whose day schedule needed a ruling is a rule that is partly our reading, and
/// FR-010 says so out loud rather than letting it pass as the venue's own wording.
#[must_use]
pub fn derivation_note(rulings: &[GapRuling]) -> Option<String> {
    if rulings.is_empty() {
        return None;
    }
    Some(
        rulings
            .iter()
            .map(|ruling| {
                format!(
                    "{}–{} mapped to {}: {}",
                    ruling.start, ruling.end, ruling.phase, ruling.reasoning
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Evidence for a rule read out of `document`.
#[must_use]
pub fn evidence(document: &FetchedDocument, effective_from: &str) -> EvidenceRecord {
    evidence_from(document, effective_from)
}

fn format_time(time: Time) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// Finds `H:MM` to `H:MM` ranges on one line, in the separators SSE's publications use.
fn time_ranges(line: &str) -> Vec<(Time, Time)> {
    let times: Vec<(usize, Time)> = time_positions(line);
    let mut ranges = Vec::new();

    for pair in times.windows(2) {
        let (first_end, first) = pair[0];
        let (second_start, second) = pair[1];
        let between = &line[first_end..second_start];
        if is_range_separator(between) && first < second {
            ranges.push((first, second));
        }
    }

    ranges
}

fn time_positions(line: &str) -> Vec<(usize, Time)> {
    let bytes = line.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let hour_text = &line[start..index];
        if index >= bytes.len() || bytes[index] != b':' || hour_text.len() > 2 {
            continue;
        }
        index += 1;
        let minute_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let minute_text = &line[minute_start..index];
        if minute_text.len() != 2 {
            continue;
        }

        let (Ok(hour), Ok(minute)) = (hour_text.parse::<i8>(), minute_text.parse::<i8>()) else {
            continue;
        };
        if let Ok(time) = Time::new(hour, minute, 0, 0) {
            found.push((index, time));
        }
    }

    // Each entry carries the byte offset *after* the time, which is what lets a caller
    // read the separator sitting between two of them.
    found
}

fn is_range_separator(between: &str) -> bool {
    let trimmed = between.trim();
    let stripped: String = trimmed
        .chars()
        .filter(|c| !c.is_ascii_digit() && *c != ':')
        .collect();
    let stripped = stripped.trim();
    matches!(stripped, "至" | "-" | "–" | "—" | "~" | "到" | "、至")
}

/// Why an SSE document could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseError {
    /// The document is not UTF-8 text.
    NotText,
    /// No session the mapping recognises was found.
    NoSessions,
    /// Two parsed sessions overlap.
    Overlapping {
        /// The earlier session's label.
        first: String,
        /// The later session's label.
        second: String,
    },
    /// Intervals between published sessions have no ruling.
    UnassignedIntervals {
        /// The intervals, in local time.
        intervals: Vec<String>,
    },
}

impl fmt::Display for SseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotText => f.write_str("the fetched document is not UTF-8 text"),
            Self::NoSessions => f.write_str(
                "no recognised session was found; the document changed shape and a person \
                 needs to look at it",
            ),
            Self::Overlapping { first, second } => {
                write!(f, "parsed sessions overlap: {first} and {second}")
            }
            Self::UnassignedIntervals { intervals } => write!(
                f,
                "the published table leaves {} unassigned. Supply a GapRuling carrying the \
                 reasoning that settles each one from the source — these are not to be filled \
                 by inference",
                intervals.join(", ")
            ),
        }
    }
}

impl std::error::Error for SseError {}
