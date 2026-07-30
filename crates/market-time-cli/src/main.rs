//! Mark Time — thin CLI shell.
//!
//! Holds no domain logic. Reads the clock (the core must not), loads rule data via
//! `market-time-data` (the core cannot), and renders what the core returns.
//!
//! Where this shell reports "now" it MUST also surface the host's clock discipline
//! bounds as uncertainty rather than presenting the host clock as exact — constitution,
//! Domain and Data Constraints. Wide-area internet time synchronisation does not reach
//! nanoseconds, and no surface here may imply that it does.

use market_time_board::{BoardView, ClockDiscipline, NowMarker, SegmentDetail};
use market_time_core::TimelineSegment;
use market_time_core::VenueId;
use market_time_core::{
    CivilInstant, CivilResolution, IanaZoneId, Phase, PhaseOutcome, Ruleset, resolve_phase,
    resolve_timeline,
};
use market_time_core::{Interval, UtcInstant};
use market_time_data::load_ruleset;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "\
market-time — what phase is a venue in, and how well is that known?

USAGE
  market-time phase    --dataset <path> [--venue <id>] [--at <rfc3339|now>] [--format text|json]
  market-time evidence --dataset <path>  --venue <id> [--at <rfc3339|now>] [--format text|json]
  market-time board    --dataset <path> [--at <rfc3339|now>] [--zone <IANA>] [--hours <n>] [--format text|svg]
  market-time timeline --dataset <path>  --venue <id> [--at <rfc3339|now>] [--hours <n>] [--format text|json]
  market-time venues   --dataset <path>

NOTES
  No venue data ships with this tool. Point --dataset at a dataset you assembled under
  your own relationship with each venue (see DATA-LICENSING.md). A synthetic dataset for
  trying the tool out lives at crates/market-time-data/fixtures/synthetic-venues.json.

  --at defaults to now. This shell reads the clock; the core never does, and the answer
  always carries how well that instant is known.

  --at-zone reads --at as a wall-clock time in that zone instead of as UTC. Twice a year a
  wall-clock time names two instants or none; those are refused with both candidates rather
  than resolved by a coin flip. Name the instant you mean with a UTC --at.

  --format svg on `board` writes a self-contained SVG to stdout: redirect it to a file and
  open it. Colour is never the only channel there either — every row is labelled, and an
  out-of-coverage stretch is hatched rather than merely paler.

  --format json is the shape for machines. Uncertainty and unknown are fields there, not
  prose: an unknown answer has \"phase\": null and a stated reason, never \"closed\".
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("market-time: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(USAGE.to_owned());
    };

    if matches!(command, "-h" | "--help" | "help") {
        return Ok(USAGE.to_owned());
    }

    let options = Options::parse(&args[1..])?;
    let dataset = options
        .dataset
        .clone()
        .ok_or_else(|| "--dataset <path> is required".to_owned())?;
    let ruleset = load_ruleset(&dataset).map_err(|error| error.to_string())?;
    let now = options.resolve_now()?;

    match command {
        "phase" => phase_command(&ruleset, &options, now),
        "evidence" => evidence_command(&ruleset, &options, now),
        "board" => board_command(&ruleset, &options, now),
        "timeline" => timeline_command(&ruleset, &options, now),
        "venues" => Ok(ruleset
            .venues()
            .iter()
            .map(|venue| venue.as_str().to_owned())
            .collect::<Vec<_>>()
            .join("\n")),
        other => Err(format!("unknown command {other:?}\n\n{USAGE}")),
    }
}

fn phase_command(ruleset: &Ruleset, options: &Options, now: NowMarker) -> Result<String, String> {
    let venues = options.venues(ruleset)?;

    if options.format == Format::Json {
        let answers: Vec<Value> = venues
            .iter()
            .map(|venue| outcome_json(venue, &resolve_phase(now.instant, venue, ruleset)))
            .collect();
        return Ok(pretty(&json!({
            "at": show(now.instant),
            "clock": now.discipline.describe(),
            "venues": answers,
        })));
    }

    let mut out = String::new();
    for venue in venues {
        let outcome = resolve_phase(now.instant, &venue, ruleset);
        out.push_str(&render_outcome(&venue, &outcome));
        out.push('\n');
    }

    out.push_str(&format!("as of {}\n", now.discipline.describe()));
    Ok(out)
}

/// What the answer at `--at` rests on: the documents, the dates, and the reasoning.
///
/// This and a graphical board call the same [`market_time_board::inspect`], so a segment
/// someone clicks and a segment someone asks about here cannot drift apart.
fn evidence_command(
    ruleset: &Ruleset,
    options: &Options,
    now: NowMarker,
) -> Result<String, String> {
    let venue = options.required_venue("evidence")?;
    let interval = options.window(now.instant)?;
    let timeline = resolve_timeline(interval, &venue, ruleset);

    let detail = market_time_board::inspect(&timeline, now.instant)
        .ok_or_else(|| format!("no segment covers {} for {venue}", show(now.instant)))?;

    if options.format == Format::Json {
        return Ok(pretty(&detail_json(&detail)));
    }

    let mut out = format!("{venue} at {}\n", show(now.instant));
    match detail.phase {
        Some(phase) => {
            out.push_str(&format!("  phase    {phase}\n"));
            out.push_str(&format!(
                "  from     {} .. {}\n",
                show(detail.interval.start),
                show(detail.interval.end)
            ));
            if let Some(uncertainty) = &detail.start_uncertainty {
                out.push_str(&format!("  start    {uncertainty}\n"));
            }
            if let Some(uncertainty) = &detail.end_uncertainty {
                out.push_str(&format!("  end      {uncertainty}\n"));
            }
        }
        None => {
            out.push_str("  phase    not known\n");
            if let Some(reason) = &detail.not_known_because {
                out.push_str(&format!("  reason   {reason}\n"));
            }
            out.push_str("  note     an unknown is not a closed market\n");
        }
    }

    if let Some(reasoning) = &detail.derived_reasoning {
        out.push_str(&format!("  derived  {reasoning}\n"));
    }
    for source in &detail.sources {
        out.push_str(&format!(
            "  source   {} (fetched {}, effective from {})\n",
            source.url, source.fetched_at, source.effective_from
        ));
        if let Some(changed) = &source.publisher_last_changed {
            out.push_str(&format!("           publisher last changed {changed}\n"));
        }
    }
    for revision in &detail.dataset_revisions {
        out.push_str(&format!("  revision {revision}\n"));
    }
    out.push_str(&format!("  as of    {}\n", now.discipline.describe()));
    Ok(out)
}

fn detail_json(detail: &SegmentDetail) -> Value {
    json!({
        "venue": detail.venue.to_string(),
        "interval": {
            "start": show(detail.interval.start),
            "end": show(detail.interval.end),
        },
        "phase": detail.phase.map(Phase::as_str),
        "not_known_because": detail.not_known_because,
        "start_uncertainty": detail.start_uncertainty,
        "end_uncertainty": detail.end_uncertainty,
        "derived_reasoning": detail.derived_reasoning,
        "sources": detail
            .sources
            .iter()
            .map(|source| json!({
                "url": source.url,
                "fetched_at": source.fetched_at,
                "effective_from": source.effective_from,
                "publisher_last_changed": source.publisher_last_changed,
            }))
            .collect::<Vec<Value>>(),
        "dataset_revisions": detail.dataset_revisions,
    })
}

/// The machine shape of one venue's outcome.
///
/// `phase` is `null` for an unknown rather than absent or `"closed"`: a consumer reading
/// only that field still cannot mistake a coverage gap for a closed market.
fn outcome_json(venue: &VenueId, outcome: &PhaseOutcome) -> Value {
    match outcome {
        PhaseOutcome::Known(answer) => json!({
            "venue": venue.to_string(),
            "known": true,
            "phase": answer.phase.as_str(),
            "boundary_start": {
                "instant": show(answer.boundary_start.instant),
                "uncertainty": answer.boundary_start.uncertainty.to_string(),
            },
            "boundary_end": {
                "instant": show(answer.boundary_end.instant),
                "uncertainty": answer.boundary_end.uncertainty.to_string(),
            },
            "uncertainty": answer.uncertainty.to_string(),
            "derived_reasoning": answer.derived_reasoning,
            "events": answer
                .events
                .iter()
                .map(|event| json!({
                    "kind": event.kind.as_str(),
                    "instant": show(event.instant),
                    "uncertainty": event.uncertainty.to_string(),
                }))
                .collect::<Vec<Value>>(),
            "sources": answer
                .evidence
                .iter()
                .map(|evidence| json!({
                    "url": evidence.source_url(),
                    "fetched_at": show(evidence.fetched_at()),
                    "effective_from": evidence.effective_from(),
                }))
                .collect::<Vec<Value>>(),
            "dataset_revisions": answer
                .dataset_revisions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
        }),
        PhaseOutcome::Unknown(gap) => json!({
            "venue": venue.to_string(),
            "known": false,
            "phase": Value::Null,
            "not_known_because": gap.describe(),
            "dataset_revisions": gap
                .dataset_revisions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
        }),
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Renders an instant for people: RFC 3339 in UTC.
///
/// Formatting is a shell concern. The core answers in nanoseconds since the epoch and has
/// no opinion about how anyone writes them down.
fn show(at: UtcInstant) -> String {
    jiff::Timestamp::from_nanosecond(at.as_nanos_since_unix_epoch())
        .map_or_else(|_| at.to_string(), |timestamp| timestamp.to_string())
}

fn render_outcome(venue: &VenueId, outcome: &PhaseOutcome) -> String {
    match outcome {
        PhaseOutcome::Known(answer) => {
            let mut text = format!(
                "{venue}: {phase}\n  starts   {start} ({start_unc})\n  ends     {end} ({end_unc})",
                phase = answer.phase,
                start = show(answer.boundary_start.instant),
                start_unc = answer.boundary_start.uncertainty,
                end = show(answer.boundary_end.instant),
                end_unc = answer.boundary_end.uncertainty,
            );
            for event in &answer.events {
                text.push_str(&format!(
                    "\n  event    {} at {} ({})",
                    event.kind,
                    show(event.instant),
                    event.uncertainty
                ));
            }
            if let Some(reasoning) = &answer.derived_reasoning {
                text.push_str(&format!("\n  derived  {reasoning}"));
            }
            for evidence in &answer.evidence {
                text.push_str(&format!(
                    "\n  source   {} (fetched {}, effective from {})",
                    evidence.source_url(),
                    show(evidence.fetched_at()),
                    evidence.effective_from()
                ));
            }
            for revision in &answer.dataset_revisions {
                text.push_str(&format!("\n  revision {revision}"));
            }
            text
        }
        PhaseOutcome::Unknown(gap) => {
            format!(
                "{venue}: not known\n  reason   {}\n  note     an unknown is not a closed market",
                gap.describe()
            )
        }
    }
}

fn board_command(ruleset: &Ruleset, options: &Options, now: NowMarker) -> Result<String, String> {
    let interval = options.window(now.instant)?;
    let rows = ruleset
        .venues()
        .iter()
        .map(|venue| resolve_timeline(interval, venue, ruleset))
        .collect();

    let view = BoardView {
        interval,
        rows,
        now: Some(now),
        axis_zone: options.zone.clone(),
        columns: options.columns,
    };

    if options.format == Format::Svg {
        return Ok(market_time_board::render_svg(&view));
    }

    Ok(market_time_board::render(&view))
}

fn timeline_command(
    ruleset: &Ruleset,
    options: &Options,
    now: NowMarker,
) -> Result<String, String> {
    let venue = options.required_venue("a timeline")?;
    let interval = options.window(now.instant)?;
    let timeline = resolve_timeline(interval, &venue, ruleset);

    if options.format == Format::Json {
        let segments: Vec<Value> = timeline
            .segments
            .iter()
            .map(|segment| match segment {
                TimelineSegment::Phase { interval, answer } => json!({
                    "start": show(interval.start),
                    "end": show(interval.end),
                    "known": true,
                    "phase": answer.phase.as_str(),
                }),
                TimelineSegment::Unknown { interval, gap } => json!({
                    "start": show(interval.start),
                    "end": show(interval.end),
                    "known": false,
                    "phase": Value::Null,
                    "not_known_because": gap.describe(),
                }),
            })
            .collect();

        return Ok(pretty(&json!({
            "venue": venue.to_string(),
            "interval": {"start": show(interval.start), "end": show(interval.end)},
            "tiles_interval": timeline.tiles_interval(),
            "segments": segments,
        })));
    }

    let mut out = format!("{venue}\n");
    for segment in &timeline.segments {
        match segment {
            TimelineSegment::Phase { interval, answer } => out.push_str(&format!(
                "  {} .. {}  {}\n",
                show(interval.start),
                show(interval.end),
                answer.phase
            )),
            TimelineSegment::Unknown { interval, gap } => out.push_str(&format!(
                "  {} .. {}  not known ({})\n",
                show(interval.start),
                show(interval.end),
                gap.describe()
            )),
        }
    }
    out.push_str(&format!(
        "  tiles the queried interval: {}\n",
        timeline.tiles_interval()
    ));
    Ok(out)
}

/// How an answer is written down. Presentation only; the answer is the same either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    /// For people.
    Text,
    /// For machines. Uncertainty and unknown are fields, never prose.
    Json,
    /// For eyes. The board as a self-contained SVG document.
    Svg,
}

struct Options {
    dataset: Option<PathBuf>,
    venue: Option<String>,
    at: Option<String>,
    at_zone: Option<String>,
    zone: String,
    hours: i64,
    columns: usize,
    format: Format,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            dataset: None,
            venue: None,
            at: None,
            at_zone: None,
            zone: "UTC".to_owned(),
            hours: 24,
            columns: 72,
            format: Format::Text,
        };

        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = || {
                args.get(index + 1)
                    .cloned()
                    .ok_or_else(|| format!("{flag} needs a value"))
            };
            match flag {
                "--dataset" => options.dataset = Some(PathBuf::from(value()?)),
                "--venue" => options.venue = Some(value()?),
                "--at" => options.at = Some(value()?),
                "--at-zone" => options.at_zone = Some(value()?),
                "--zone" => options.zone = value()?,
                "--hours" => {
                    options.hours = value()?
                        .parse()
                        .map_err(|_| "--hours needs a whole number".to_owned())?;
                }
                "--format" => {
                    options.format = match value()?.as_str() {
                        "text" => Format::Text,
                        "json" => Format::Json,
                        "svg" => Format::Svg,
                        other => {
                            return Err(format!("--format {other:?} is not text, json, or svg"));
                        }
                    };
                }
                "--columns" => {
                    options.columns = value()?
                        .parse()
                        .map_err(|_| "--columns needs a whole number".to_owned())?;
                }
                other => return Err(format!("unknown option {other:?}")),
            }
            index += 2;
        }

        Ok(options)
    }

    /// The venues to answer for: the one that was named, or every venue in the dataset.
    ///
    /// # Errors
    ///
    /// Returns the identifier error when `--venue` was blank.
    /// The venue a single-venue command was asked about.
    ///
    /// # Errors
    ///
    /// Returns a message when `--venue` is absent or blank.
    fn required_venue(&self, command: &str) -> Result<VenueId, String> {
        self.venue
            .as_deref()
            .ok_or_else(|| format!("--venue <id> is required for {command}"))
            .and_then(|value| VenueId::new(value).map_err(|error| error.to_string()))
    }

    fn venues(&self, ruleset: &Ruleset) -> Result<Vec<VenueId>, String> {
        match &self.venue {
            Some(venue) => VenueId::new(venue)
                .map(|venue| vec![venue])
                .map_err(|error| error.to_string()),
            None => Ok(ruleset.venues()),
        }
    }

    /// Resolves the instant to answer for, and says how well it is known.
    ///
    /// Reading the clock is this shell's job precisely so the core never does it. The
    /// discipline bound is reported as unmeasured rather than guessed: this process has no
    /// way to ask the host how well its clock is steered, and a made-up bound would be a
    /// worse claim than an honest absence.
    fn resolve_now(&self) -> Result<NowMarker, String> {
        match self.at.as_deref() {
            None | Some("now") => {
                let since_epoch = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| "the host clock is set before 1970".to_owned())?;
                let nanos = i128::from(since_epoch.as_secs()) * 1_000_000_000
                    + i128::from(since_epoch.subsec_nanos());
                Ok(NowMarker {
                    instant: UtcInstant::from_nanos_since_unix_epoch(nanos),
                    discipline: ClockDiscipline::Unmeasured {
                        source: "host system clock; no NTP or PTP bound available to this process"
                            .to_owned(),
                    },
                })
            }
            Some(text) if self.at_zone.is_some() => self.resolve_civil(text),
            Some(text) => {
                let timestamp: jiff::Timestamp = text
                    .parse()
                    .map_err(|_| format!("--at {text:?} is not an RFC 3339 instant"))?;
                Ok(NowMarker {
                    instant: UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond()),
                    discipline: ClockDiscipline::Given {
                        source: "--at on the command line".to_owned(),
                    },
                })
            }
        }
    }

    /// Reads `--at` as a wall-clock time in `--at-zone`.
    ///
    /// A wall-clock time that occurs twice, or that does not occur at all, is refused with
    /// both candidate instants. The alternative would be for this shell to pick one and
    /// say nothing, which is precisely the silent guess the whole product exists to avoid.
    fn resolve_civil(&self, text: &str) -> Result<NowMarker, String> {
        let zone = self
            .at_zone
            .as_deref()
            .ok_or_else(|| "--at-zone is required to read --at as a wall-clock time".to_owned())?;
        let zone_id = IanaZoneId::new(zone).map_err(|error| error.to_string())?;
        let civil = CivilInstant::parse(zone_id, text).map_err(|error| error.to_string())?;

        match civil.to_utc() {
            CivilResolution::Unambiguous(instant) => Ok(NowMarker {
                instant,
                discipline: ClockDiscipline::Given {
                    source: format!("--at {text} read in {zone}"),
                },
            }),
            CivilResolution::Ambiguous { earlier, later } => Err(format!(
                "{text} occurs twice in {zone}: {} and {}. Which one is meant is yours to say — pass one of them as a UTC --at",
                show(earlier),
                show(later)
            )),
            CivilResolution::Nonexistent {
                as_if_before_shift,
                as_if_after_shift,
            } => Err(format!(
                "{text} does not exist in {zone}: the clocks jumped from {} to {}. No instant bears that reading",
                show(as_if_after_shift),
                show(as_if_before_shift)
            )),
        }
    }

    fn window(&self, now: UtcInstant) -> Result<Interval, String> {
        let span = i128::from(self.hours) * 3_600 * 1_000_000_000;
        if span == 0 {
            return Err("--hours must not be zero".to_owned());
        }
        let (start, end) = if span > 0 {
            (now, now.saturating_add_nanos(span))
        } else {
            (now.saturating_add_nanos(span), now)
        };
        Interval::new(start, end).map_err(|error| error.to_string())
    }
}
