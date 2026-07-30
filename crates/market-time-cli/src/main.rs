//! Mark Time — thin CLI shell.
//!
//! Holds no domain logic. Reads the clock (the core must not), loads rule data via
//! `market-time-data` (the core cannot), and renders what the core returns.
//!
//! Where this shell reports "now" it MUST also surface the host's clock discipline
//! bounds as uncertainty rather than presenting the host clock as exact — constitution,
//! Domain and Data Constraints. Wide-area internet time synchronisation does not reach
//! nanoseconds, and no surface here may imply that it does.

use market_time_board::{BoardView, ClockDiscipline, NowMarker};
use market_time_core::TimelineSegment;
use market_time_core::VenueId;
use market_time_core::{Interval, UtcInstant};
use market_time_core::{PhaseOutcome, Ruleset, resolve_phase, resolve_timeline};
use market_time_data::load_ruleset;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "\
market-time — what phase is a venue in, and how well is that known?

USAGE
  market-time phase    --dataset <path> [--venue <id>] [--at <rfc3339|now>]
  market-time board    --dataset <path> [--at <rfc3339|now>] [--zone <IANA>] [--hours <n>]
  market-time timeline --dataset <path>  --venue <id> [--at <rfc3339|now>] [--hours <n>]
  market-time venues   --dataset <path>

NOTES
  No venue data ships with this tool. Point --dataset at a dataset you assembled under
  your own relationship with each venue (see DATA-LICENSING.md). A synthetic dataset for
  trying the tool out lives at crates/market-time-data/fixtures/synthetic-venues.json.

  --at defaults to now. This shell reads the clock; the core never does, and the answer
  always carries how well that instant is known.
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
    let mut out = String::new();

    for venue in venues {
        let outcome = resolve_phase(now.instant, &venue, ruleset);
        out.push_str(&render_outcome(&venue, &outcome));
        out.push('\n');
    }

    out.push_str(&format!("as of {}\n", now.discipline.describe()));
    Ok(out)
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

    Ok(market_time_board::render(&view))
}

fn timeline_command(
    ruleset: &Ruleset,
    options: &Options,
    now: NowMarker,
) -> Result<String, String> {
    let venue = options
        .venue
        .as_deref()
        .ok_or_else(|| "--venue <id> is required for a timeline".to_owned())
        .and_then(|value| VenueId::new(value).map_err(|error| error.to_string()))?;

    let interval = options.window(now.instant)?;
    let timeline = resolve_timeline(interval, &venue, ruleset);

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

struct Options {
    dataset: Option<PathBuf>,
    venue: Option<String>,
    at: Option<String>,
    zone: String,
    hours: i64,
    columns: usize,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            dataset: None,
            venue: None,
            at: None,
            zone: "UTC".to_owned(),
            hours: 24,
            columns: 72,
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
                "--zone" => options.zone = value()?,
                "--hours" => {
                    options.hours = value()?
                        .parse()
                        .map_err(|_| "--hours needs a whole number".to_owned())?;
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
