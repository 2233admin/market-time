//! Mark Time — thin CLI shell.
//!
//! Holds no domain logic. Reads the clock (the core must not), loads rule data via
//! `market-time-data` (the core cannot), and renders what the core returns.
//!
//! Where this shell reports "now" it MUST also surface the host's clock discipline
//! bounds as uncertainty rather than presenting the host clock as exact — constitution,
//! Domain and Data Constraints. Wide-area internet time synchronisation does not reach
//! nanoseconds, and no surface here may imply that it does.

use std::{env, error::Error, fmt, path::PathBuf, process::ExitCode};

use jiff::Timestamp;
use market_time_core::{
    BoundaryCharacter, EvidenceRef, PhaseBoundary, PhaseOutcome, Uncertainty, UtcInstant, VenueId,
    resolve_phase,
};
use market_time_data::load_ruleset_from_path;
use serde_json::{Value, json};

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("market-time: {error}");
            ExitCode::from(error.exit_code)
        }
    }
}

fn run(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let command = parse_command(args)?;
    match command {
        Command::Phase {
            ruleset_path,
            venue,
            at,
            format,
        } => {
            let loaded = load_ruleset_from_path(ruleset_path)
                .map_err(|error| CliError::data(error.to_string()))?;
            let at = parse_explicit_utc(&at)?;
            let venue = VenueId::new(venue).map_err(|error| CliError::usage(error.to_string()))?;
            let outcome = resolve_phase(at, venue, &loaded.ruleset);
            let tzdb = market_time_core::tzdata::iana_tzdb_version().unwrap_or("unknown");
            let value = match outcome {
                PhaseOutcome::Known(answer) => json!({
                    "iana_tzdb_revision": tzdb,
                    "query": instant_json(at),
                    "status": "known",
                    "venue": answer.venue.as_str(),
                    "phase": answer.phase.as_str(),
                    "boundary_start": boundary_json(&answer.boundary_start),
                    "boundary_end": boundary_json(&answer.boundary_end),
                    "uncertainty": uncertainty_json(&answer.uncertainty),
                    "evidence": answer.evidence.iter().map(evidence_json).collect::<Vec<_>>(),
                    "dataset_revisions": answer
                        .dataset_revisions
                        .iter()
                        .map(|revision| revision.as_str())
                        .collect::<Vec<_>>(),
                }),
                PhaseOutcome::Unknown(gap) => json!({
                    "iana_tzdb_revision": tzdb,
                    "query": instant_json(at),
                    "status": "unknown",
                    "venue": gap.venue.as_str(),
                    "coverage_gap": gap.coverage.map(|coverage| json!({
                        "valid_from_ns": coverage
                            .valid_from
                            .as_nanos_since_unix_epoch()
                            .to_string(),
                        "valid_until_ns": coverage
                            .valid_until
                            .map(|instant| instant
                                .as_nanos_since_unix_epoch()
                                .to_string()),
                    })),
                    "dataset_revisions": gap
                        .dataset_revisions
                        .iter()
                        .map(|revision| revision.as_str())
                        .collect::<Vec<_>>(),
                }),
            };
            print_value(&value, format)?;
        }
    }
    Ok(())
}

enum Command {
    Phase {
        ruleset_path: PathBuf,
        venue: String,
        at: String,
        format: OutputFormat,
    },
}

#[derive(Clone, Copy)]
enum OutputFormat {
    Json,
    Text,
}

fn parse_command(mut args: impl Iterator<Item = String>) -> Result<Command, CliError> {
    let Some(command) = args.next() else {
        return Err(CliError::usage(usage()));
    };
    if command != "phase" {
        return Err(CliError::usage(format!(
            "unsupported command {command:?}\n{}",
            usage()
        )));
    }
    let mut ruleset_path = None;
    let mut venue = None;
    let mut at = None;
    let mut format = OutputFormat::Json;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| CliError::usage(format!("missing value for {flag}")))?;
        match flag.as_str() {
            "--ruleset" => ruleset_path = Some(PathBuf::from(value)),
            "--venue" => venue = Some(value),
            "--at" => at = Some(value),
            "--format" => {
                format = match value.as_str() {
                    "json" => OutputFormat::Json,
                    "text" => OutputFormat::Text,
                    _ => return Err(CliError::usage("--format must be json or text")),
                };
            }
            _ => return Err(CliError::usage(format!("unrecognized flag {flag:?}"))),
        }
    }
    Ok(Command::Phase {
        ruleset_path: ruleset_path.ok_or_else(|| CliError::usage("missing --ruleset"))?,
        venue: venue.ok_or_else(|| CliError::usage("missing --venue"))?,
        at: at.ok_or_else(|| CliError::usage("missing --at"))?,
        format,
    })
}

fn parse_explicit_utc(value: &str) -> Result<UtcInstant, CliError> {
    if value == "now" {
        return Err(CliError::usage(
            "`--at now` is disabled until host clock-discipline bounds are available",
        ));
    }
    let timestamp: Timestamp = value
        .parse()
        .map_err(|error| CliError::usage(format!("invalid RFC 3339 UTC instant: {error}")))?;
    if !value.ends_with('Z') {
        return Err(CliError::usage(
            "--at must be an RFC 3339 UTC instant ending in Z",
        ));
    }
    Ok(UtcInstant::from_nanos_since_unix_epoch(
        timestamp.as_nanosecond(),
    ))
}

fn instant_json(instant: UtcInstant) -> Value {
    let nanos = instant.as_nanos_since_unix_epoch();
    let rfc3339 = Timestamp::from_nanosecond(nanos)
        .map_or_else(|_| None, |timestamp| Some(timestamp.to_string()));
    json!({
        "scale": "UTC",
        "nanos_since_unix_epoch": nanos.to_string(),
        "rfc3339": rfc3339,
    })
}

fn boundary_json(boundary: &PhaseBoundary) -> Value {
    json!({
        "instant": instant_json(boundary.instant),
        "uncertainty": uncertainty_json(&boundary.uncertainty),
    })
}

fn uncertainty_json(uncertainty: &Uncertainty) -> Value {
    json!({
        "granularity_ns": uncertainty.granularity_ns,
        "published_bound_ns": uncertainty.published_bound_ns,
        "boundary_character": match uncertainty.boundary_character {
            BoundaryCharacter::Instantaneous => "instantaneous",
            BoundaryCharacter::ProcessStart => "process_start",
        },
        "process_spread_ns": uncertainty.process_spread_ns,
        "is_derived": uncertainty.is_derived,
    })
}

fn evidence_json(evidence: &EvidenceRef) -> Value {
    json!({
        "source_url": evidence.source_url,
        "fetched_at": instant_json(evidence.fetched_at),
        "effective_from": instant_json(evidence.effective_from),
        "source_updated_at": evidence.source_updated_at.map(instant_json),
        "is_derived": evidence.is_derived,
        "derivation_reasoning": evidence.derivation_reasoning,
    })
}

fn print_value(value: &Value, format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => {
            let output = serde_json::to_string(value)
                .map_err(|error| CliError::internal(error.to_string()))?;
            println!("{output}");
        }
        OutputFormat::Text => {
            let object = value
                .as_object()
                .ok_or_else(|| CliError::internal("text output expected an object"))?;
            for (key, value) in object {
                println!("{key}={value}");
            }
        }
    }
    Ok(())
}

fn usage() -> &'static str {
    "usage: market-time phase --ruleset <PATH> --venue <ID> --at <RFC3339_UTC> \
     [--format json|text]"
}

struct CliError {
    exit_code: u8,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            message: message.into(),
        }
    }

    fn data(message: impl Into<String>) -> Self {
        Self {
            exit_code: 4,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl fmt::Debug for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CliError")
            .field("exit_code", &self.exit_code)
            .field("message", &self.message)
            .finish()
    }
}

impl Error for CliError {}
