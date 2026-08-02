//! Production HTTP presentation over the pure market-time core.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

use axum::error_handling::HandleErrorLayer;
use axum::extract::{Query, State, rejection::QueryRejection};
use axum::http::{HeaderName, Method, Request as HttpRequest, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, get_service};
use axum::{BoxError, Json, Router};
use market_time_core::{
    EvidenceRef, Interval, NANOS_PER_SECOND, PhaseOutcome, Ruleset, TimelineSegment, UtcInstant,
    resolve_phases, resolve_timeline, tzdata,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tower::ServiceBuilder;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower::timeout::TimeoutLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::{DefaultOnResponse, TraceLayer};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_IN_FLIGHT: usize = 256;
const UTC_DAY_NANOS: i128 = 86_400 * NANOS_PER_SECOND;

#[derive(Clone)]
struct AppState {
    ruleset: Arc<Ruleset>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusQuery {
    at: Option<String>,
}

/// Builds the complete HTTP application for one immutable dataset revision.
pub fn app(ruleset: Ruleset) -> Router {
    let state = AppState {
        ruleset: Arc::new(ruleset),
    };
    let routes = Router::new()
        .route("/", get_service(ServeFile::new(web_asset("index.html"))))
        .route(
            "/audit",
            get(|| async { Redirect::permanent("/settings#source-intelligence") }),
        )
        .route(
            "/settings",
            get_service(ServeFile::new(web_asset("settings.html"))),
        )
        .route(
            "/widget",
            get_service(ServeFile::new(web_asset("widget.html"))),
        )
        .nest_service("/_next", ServeDir::new(web_asset("_next")))
        .route("/health", get(health))
        .route("/v1/venues", get(venues))
        .route("/v1/status", get(statuses))
        .route("/v1/timeline", get(timelines))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .with_state(state);

    production_layers(routes, REQUEST_TIMEOUT)
}

async fn timelines(
    State(state): State<AppState>,
    query: Result<Query<StatusQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => return error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let (at, supplied) = match requested_instant(query.at.as_deref()) {
        Ok(result) => result,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let day_start_nanos = at.as_nanos_since_unix_epoch().div_euclid(UTC_DAY_NANOS) * UTC_DAY_NANOS;
    let interval = Interval::new(
        UtcInstant::from_nanos_since_unix_epoch(day_start_nanos),
        UtcInstant::from_nanos_since_unix_epoch(day_start_nanos + UTC_DAY_NANOS),
    )
    .expect("one UTC day is a valid interval");

    let venues: Vec<Value> = state
        .ruleset
        .venues()
        .into_iter()
        .map(|venue| {
            let timeline = resolve_timeline(interval, &venue, &state.ruleset);
            let profile = state.ruleset.profile(&venue);
            let label = profile.map_or_else(
                || venue.as_str().to_owned(),
                |profile| profile.label(venue.as_str()).to_owned(),
            );
            let rules = state.ruleset.venue(&venue);
            let (next_trading_transition, next_trading_window) =
                upcoming_trading(at, &venue, &state.ruleset);

            json!({
                "id": venue.as_str(),
                "display_name": label,
                "location": profile.and_then(|profile| profile.location.as_deref()),
                "home_zone": rules.map(|rules| rules.home_zone.as_str()),
                "family": profile.and_then(|profile| profile.family.map(market_time_core::AssetFamily::as_str)),
                "tiles_interval": timeline.tiles_interval(),
                "next_trading_transition": next_trading_transition,
                "next_trading_window": next_trading_window,
                "trading_windows": trading_window_values(&timeline.segments),
                "segments": timeline
                    .segments
                    .iter()
                    .map(|segment| timeline_segment_value(segment, at, interval))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();

    success(json!({
        "at": at.to_string(),
        "clock": clock_value(supplied),
        "interval": {
            "start": interval.start.to_string(),
            "end": interval.end.to_string(),
            "axis_zone": "UTC",
        },
        "tzdb_version": tzdata::iana_tzdb_version(),
        "dataset_revisions": revision_ids(&state.ruleset),
        "venues": venues,
    }))
}

fn upcoming_trading(
    at: UtcInstant,
    venue: &market_time_core::VenueId,
    ruleset: &Ruleset,
) -> (Option<Value>, Option<Value>) {
    let Some(coverage) = ruleset.venue(venue).map(|rules| rules.coverage.interval()) else {
        return (None, None);
    };
    let Ok(search) = Interval::new(at, coverage.end) else {
        return (None, None);
    };
    let timeline = resolve_timeline(search, venue, ruleset);
    let current_trading = timeline.segments.iter().find_map(|segment| match segment {
        TimelineSegment::Phase { interval, answer } if interval.contains(at) => {
            Some(answer.phase.is_trading())
        }
        _ => None,
    });

    let transition = current_trading.and_then(|current_trading| {
        timeline.segments.iter().find_map(|segment| match segment {
            TimelineSegment::Phase { interval, answer }
                if interval.start > at && answer.phase.is_trading() != current_trading =>
            {
                Some(json!({
                    "at": interval.start.to_string(),
                    "kind": if answer.phase.is_trading() { "opens" } else { "closes" },
                    "phase": answer.phase.as_str(),
                }))
            }
            _ => None,
        })
    });

    let mut window: Option<(UtcInstant, UtcInstant)> = None;
    for segment in &timeline.segments {
        let TimelineSegment::Phase { interval, answer } = segment else {
            if window.is_some() {
                break;
            }
            continue;
        };
        if !answer.phase.is_trading() {
            if window.is_some() {
                break;
            }
            continue;
        }
        if let Some((_, end)) = &mut window {
            if *end != interval.start {
                break;
            }
            *end = interval.end;
        } else {
            window = Some((interval.start, interval.end));
        }
    }

    let window = window.map(|(start, end)| {
        json!({
            "start": start.to_string(),
            "end": end.to_string(),
        })
    });

    (transition, window)
}

fn trading_window_values(segments: &[TimelineSegment]) -> Vec<Value> {
    let mut windows: Vec<(UtcInstant, UtcInstant)> = Vec::new();
    for segment in segments {
        let TimelineSegment::Phase { interval, answer } = segment else {
            continue;
        };
        if !answer.phase.is_trading() {
            continue;
        }
        if let Some((_, end)) = windows.last_mut()
            && *end == interval.start
        {
            *end = interval.end;
            continue;
        }
        windows.push((interval.start, interval.end));
    }
    windows
        .into_iter()
        .map(|(start, end)| json!({"start": start.to_string(), "end": end.to_string()}))
        .collect()
}

fn web_asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("web")
        .join("out")
        .join(name)
}

async fn health(State(state): State<AppState>) -> Response {
    success(json!({
        "status": "ok",
        "tzdb_version": tzdata::iana_tzdb_version(),
        "dataset_revisions": revision_ids(&state.ruleset),
    }))
}

async fn venues(State(state): State<AppState>) -> Response {
    let venues: Vec<Value> = state
        .ruleset
        .venues()
        .iter()
        .filter_map(|venue| {
            let rules = state.ruleset.venue(venue)?;
            Some(json!({
                "id": venue.as_str(),
                "display_name": rules.profile.display_name,
                "location": rules.profile.location,
                "home_zone": rules.home_zone.as_str(),
                "family": rules
                    .profile
                    .family
                    .map(market_time_core::AssetFamily::as_str),
                "coverage": {
                    "start": rules.coverage.start().to_string(),
                    "end": rules.coverage.end().to_string(),
                },
            }))
        })
        .collect();

    success(json!({
        "tzdb_version": tzdata::iana_tzdb_version(),
        "dataset_revisions": revision_ids(&state.ruleset),
        "venues": venues,
    }))
}

async fn statuses(
    State(state): State<AppState>,
    query: Result<Query<StatusQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => return error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let (at, supplied) = match requested_instant(query.at.as_deref()) {
        Ok(result) => result,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let venues = state.ruleset.venues();
    let answers: Vec<Value> = resolve_phases(at, &venues, &state.ruleset)
        .into_iter()
        .map(|answer| {
            let profile = state.ruleset.profile(&answer.venue);
            let label = profile.map_or_else(
                || answer.venue.as_str().to_owned(),
                |profile| profile.label(answer.venue.as_str()).to_owned(),
            );
            let location = profile.and_then(|profile| profile.location.as_deref());
            let home_zone = state
                .ruleset
                .venue(&answer.venue)
                .map(|rules| rules.home_zone.as_str());

            match answer.outcome {
                PhaseOutcome::Known(answer) => json!({
                    "id": answer.venue.as_str(),
                    "display_name": label,
                    "location": location,
                    "home_zone": home_zone,
                    "status": "known",
                "phase": answer.phase.as_str(),
                "calendar": {
                    "kind": answer.calendar_rule_kind,
                    "label": answer.calendar_label,
                },
                    "boundary_start": {
                        "instant": answer.boundary_start.instant.to_string(),
                        "uncertainty": answer.boundary_start.uncertainty.to_string(),
                    },
                    "boundary_end": {
                        "instant": answer.boundary_end.instant.to_string(),
                        "uncertainty": answer.boundary_end.uncertainty.to_string(),
                    },
                    "uncertainty": answer.uncertainty.to_string(),
                    "derived_reasoning": answer.derived_reasoning,
                    "events": answer.events.iter().map(event_value).collect::<Vec<_>>(),
                    "evidence": answer.evidence.iter().map(evidence_value).collect::<Vec<_>>(),
                    "dataset_revisions": answer.dataset_revisions.iter().map(ToString::to_string).collect::<Vec<_>>(),
                }),
                PhaseOutcome::Unknown(gap) => json!({
                    "id": gap.venue.as_str(),
                    "display_name": label,
                    "location": location,
                    "home_zone": home_zone,
                    "status": "unknown",
                    "reason": gap.describe(),
                    "queried_at": gap.queried_at.to_string(),
                    "coverage": gap.coverage.as_ref().map(|coverage| json!({
                        "start": coverage.start().to_string(),
                        "end": coverage.end().to_string(),
                    })),
                    "dataset_revisions": gap.dataset_revisions.iter().map(ToString::to_string).collect::<Vec<_>>(),
                }),
            }
        })
        .collect();

    success(json!({
        "at": at.to_string(),
        "clock": clock_value(supplied),
        "tzdb_version": tzdata::iana_tzdb_version(),
        "dataset_revisions": revision_ids(&state.ruleset),
        "venues": answers,
    }))
}

fn timeline_segment_value(segment: &TimelineSegment, at: UtcInstant, axis: Interval) -> Value {
    match segment {
        TimelineSegment::Phase { interval, answer } => json!({
            "start": interval.start.to_string(),
            "end": interval.end.to_string(),
            "position": segment_position(*interval, axis),
            "status": "known",
            "phase": answer.phase.as_str(),
            "trading": answer.phase.is_trading(),
            "current": interval.contains(at),
            "calendar": {
                "kind": answer.calendar_rule_kind,
                "label": answer.calendar_label,
            },
            "boundary_uncertainty": {
                "start": answer.boundary_start.uncertainty.to_string(),
                "end": answer.boundary_end.uncertainty.to_string(),
            },
            "events": answer.events.iter().map(event_value).collect::<Vec<_>>(),
        }),
        TimelineSegment::Unknown { interval, gap } => json!({
            "start": interval.start.to_string(),
            "end": interval.end.to_string(),
            "position": segment_position(*interval, axis),
            "status": "unknown",
            "phase": Value::Null,
            "trading": Value::Null,
            "current": interval.contains(at),
            "reason": gap.describe(),
            "coverage": gap.coverage.as_ref().map(|coverage| json!({
                "start": coverage.start().to_string(),
                "end": coverage.end().to_string(),
            })),
        }),
    }
}

fn segment_position(segment: Interval, axis: Interval) -> Value {
    let span = axis.start.saturating_nanos_until(axis.end);
    let start = axis.start.saturating_nanos_until(segment.start) * 1_000_000 / span;
    let end = axis.start.saturating_nanos_until(segment.end) * 1_000_000 / span;
    json!({
        "start_millionths": u32::try_from(start.clamp(0, 1_000_000)).expect("clamped position fits u32"),
        "end_millionths": u32::try_from(end.clamp(0, 1_000_000)).expect("clamped position fits u32"),
    })
}

fn clock_value(supplied: bool) -> Value {
    if supplied {
        json!({"discipline": "supplied"})
    } else {
        json!({
            "discipline": "unmeasured",
            "source": "host system clock; no NTP or PTP bound available to this process",
        })
    }
}

fn requested_instant(at: Option<&str>) -> Result<(UtcInstant, bool), String> {
    let Some(at) = at else {
        return system_now().map(|now| (now, false));
    };
    if !at.ends_with('Z') {
        return Err("at must be a UTC RFC 3339 instant ending in Z".to_owned());
    }
    let timestamp: jiff::Timestamp = at
        .parse()
        .map_err(|_| "at must be a valid UTC RFC 3339 instant".to_owned())?;
    Ok((
        UtcInstant::from_nanos_since_unix_epoch(timestamp.as_nanosecond()),
        true,
    ))
}

fn system_now() -> Result<UtcInstant, String> {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "the host clock is set before 1970".to_owned())?;
    let nanos =
        i128::from(since_epoch.as_secs()) * 1_000_000_000 + i128::from(since_epoch.subsec_nanos());
    Ok(UtcInstant::from_nanos_since_unix_epoch(nanos))
}

fn event_value(event: &market_time_core::EventOccurrence) -> Value {
    json!({
        "kind": event.kind.as_str(),
        "instant": event.instant.to_string(),
        "uncertainty": event.uncertainty.to_string(),
        "evidence": event.evidence.iter().map(evidence_value).collect::<Vec<_>>(),
    })
}

fn evidence_value(evidence: &EvidenceRef) -> Value {
    json!({
        "source_url": evidence.source_url(),
        "fetched_at": evidence.fetched_at().to_string(),
        "effective_from": evidence.effective_from(),
        "publisher_last_changed": evidence.publisher_last_changed(),
    })
}

fn revision_ids(ruleset: &Ruleset) -> Vec<String> {
    ruleset
        .revision_ids()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn success(body: Value) -> Response {
    (StatusCode::OK, Json(body)).into_response()
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({"error": message.into()}))).into_response()
}

async fn not_found() -> Response {
    error(StatusCode::NOT_FOUND, "route not found")
}

async fn method_not_allowed() -> Response {
    error(StatusCode::METHOD_NOT_ALLOWED, "only GET is supported")
}

async fn handle_middleware_error(error_value: BoxError) -> Response {
    if error_value.is::<tower::timeout::error::Elapsed>() {
        return error(StatusCode::REQUEST_TIMEOUT, "request timed out");
    }
    tracing::error!(error = %error_value, "HTTP middleware failed");
    error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

fn production_layers(router: Router, timeout: Duration) -> Router {
    let request_id = HeaderName::from_static("x-request-id");
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET])
        .expose_headers([request_id.clone()]);

    router.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(request_id.clone(), MakeRequestUuid))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &HttpRequest<axum::body::Body>| {
                        let request_id = request
                            .headers()
                            .get("x-request-id")
                            .and_then(|value| value.to_str().ok())
                            .unwrap_or("-");
                        tracing::info_span!(
                            "http_request",
                            %request_id,
                            method = %request.method(),
                            uri = %request.uri(),
                        )
                    })
                    .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
            )
            .layer(PropagateRequestIdLayer::new(request_id))
            .layer(CompressionLayer::new())
            .layer(cors)
            .layer(HandleErrorLayer::new(handle_middleware_error))
            .layer(TimeoutLayer::new(timeout))
            // ponytail: a single process-wide cap is sufficient until load tests justify
            // per-tenant limits or an edge rate limiter.
            .layer(GlobalConcurrencyLimitLayer::new(MAX_IN_FLIGHT)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn timeout_errors_keep_the_json_contract() {
        let routes = Router::new().route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Json(json!({"status": "late"}))
            }),
        );
        let response = production_layers(routes, Duration::from_millis(1))
            .oneshot(Request::get("/slow").body(Body::empty()).unwrap())
            .await
            .expect("router is infallible");
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("body reads");
        let json: Value = serde_json::from_slice(&body).expect("timeout body is JSON");

        assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
        assert_eq!(json["error"], "request timed out");
    }
}
