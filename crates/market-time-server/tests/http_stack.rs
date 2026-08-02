use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use market_time_core::Ruleset;
use market_time_data::load_ruleset;
use market_time_server::app;
use serde_json::Value;
use std::path::PathBuf;
use tower::ServiceExt;

fn ruleset() -> Ruleset {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../market-time-data/fixtures/synthetic-venues.json");
    load_ruleset(&fixture).expect("synthetic fixture loads")
}

async fn send(
    router: Router,
    method: Method,
    uri: &str,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let response = router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::ORIGIN, "https://clock.example")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router is infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body reads");
    let json = serde_json::from_slice(&body).expect("response is JSON");
    (status, headers, json)
}

#[tokio::test]
async fn production_router_preserves_the_public_contract() {
    let router = app(ruleset());

    let (status, headers, health) = send(router.clone(), Method::GET, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health["status"], "ok");
    assert!(headers.contains_key("x-request-id"));
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");

    let (status, _, venues) = send(router.clone(), Method::GET, "/v1/venues").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(venues["venues"].as_array().map(Vec::len), Some(3));

    let (status, _, answers) = send(
        router,
        Method::GET,
        "/v1/status?at=2026-07-30T02%3A00%3A00Z",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(answers["at"], "2026-07-30T02:00:00Z");
    assert_eq!(answers["clock"]["discipline"], "supplied");
}

#[tokio::test]
async fn status_exposes_calendar_exceptions_from_the_core() {
    let (status, _, body) = send(
        app(ruleset()),
        Method::GET,
        "/v1/status?at=2026-10-01T02%3A00%3A00Z",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let venue = body["venues"]
        .as_array()
        .and_then(|venues| venues.iter().find(|venue| venue["id"] == "SYNTH-AUCT"))
        .expect("auction venue is returned");
    assert_eq!(venue["location"], "Shanghai");
    assert_eq!(venue["home_zone"], "Asia/Shanghai");
    assert_eq!(venue["calendar"]["kind"], "holiday");
    assert_eq!(venue["calendar"]["label"], "Synthetic National Day");
}

#[tokio::test]
async fn timeline_exposes_one_utc_day_of_core_segments() {
    let (status, _, body) = send(
        app(ruleset()),
        Method::GET,
        "/v1/timeline?at=2026-07-30T02%3A00%3A00Z",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["at"], "2026-07-30T02:00:00Z");
    assert_eq!(body["interval"]["start"], "2026-07-30T00:00:00Z");
    assert_eq!(body["interval"]["end"], "2026-07-31T00:00:00Z");
    assert_eq!(body["interval"]["axis_zone"], "UTC");

    let venues = body["venues"].as_array().expect("venues are returned");
    assert_eq!(venues.len(), 3);
    assert!(venues.iter().all(|venue| venue["tiles_interval"] == true));

    let auction = venues
        .iter()
        .find(|venue| venue["id"] == "SYNTH-AUCT")
        .expect("auction venue is returned");
    let segments = auction["segments"]
        .as_array()
        .expect("timeline segments are returned");
    assert!(segments.iter().any(|segment| {
        segment["status"] == "known"
            && segment["phase"] == "continuous_trading"
            && segment["current"] == true
            && segment["calendar"]["kind"] == "weekly_pattern"
    }));
    assert_eq!(auction["trading_windows"].as_array().map(Vec::len), Some(2));
    assert_eq!(auction["next_trading_transition"]["kind"], "closes");
    assert_eq!(
        auction["next_trading_transition"]["at"],
        "2026-07-30T03:30:00Z"
    );

    let always_on = venues
        .iter()
        .find(|venue| venue["id"] == "SYNTH-ALWAYS")
        .expect("always-on venue is returned");
    assert!(always_on["next_trading_transition"].is_null());
}

#[tokio::test]
async fn timeline_keeps_unknown_distinct_from_closed() {
    let (status, _, body) = send(
        app(ruleset()),
        Method::GET,
        "/v1/timeline?at=2030-01-01T00%3A00%3A00Z",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["venues"].as_array().is_some_and(|venues| {
        venues.iter().all(|venue| {
            let no_transition = venue["next_trading_transition"].is_null();
            let no_window = venue["next_trading_window"].is_null();
            venue["segments"].as_array().is_some_and(|segments| {
                segments
                    .iter()
                    .all(|segment| segment["status"] == "unknown" && segment["phase"].is_null())
            }) && no_transition
                && no_window
        })
    }));

    for uri in [
        "/v1/timeline?at=not-a-time",
        "/v1/timeline?extra=value",
        "/v1/timeline?at=2026-07-30T02%3A00%3A00Z&at=2026-07-30T03%3A00%3A00Z",
    ] {
        let (status, _, body) = send(app(ruleset()), Method::GET, uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string());
    }
}

#[tokio::test]
async fn timeline_reports_the_next_trading_change_not_the_day_clip() {
    let (status, _, body) = send(
        app(ruleset()),
        Method::GET,
        "/v1/timeline?at=2026-08-01T03%3A00%3A00Z",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let auction = body["venues"]
        .as_array()
        .and_then(|venues| venues.iter().find(|venue| venue["id"] == "SYNTH-AUCT"))
        .expect("auction venue is returned");
    assert_eq!(auction["segments"][0]["end"], "2026-08-02T00:00:00Z");
    assert_eq!(auction["next_trading_transition"]["kind"], "opens");
    assert_eq!(
        auction["next_trading_transition"]["at"],
        "2026-08-03T01:25:00Z"
    );
    assert_eq!(
        auction["next_trading_window"]["start"],
        "2026-08-03T01:25:00Z"
    );
    assert_eq!(
        auction["next_trading_window"]["end"],
        "2026-08-03T03:30:00Z"
    );
}

#[tokio::test]
async fn frontend_is_served_without_relaxing_api_route_errors() {
    let response = app(ruleset())
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router is infallible");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .expect("content type is ASCII")
            .starts_with("text/html")
    );
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body reads");
    assert!(
        String::from_utf8(body.to_vec())
            .expect("HTML is UTF-8")
            .contains("data-ui-framework=\"next-appica\"")
    );

    let audit = app(ruleset())
        .oneshot(
            Request::builder()
                .uri("/audit")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router is infallible");
    assert_eq!(audit.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(
        audit.headers()[header::LOCATION],
        "/settings#source-intelligence"
    );

    let settings = app(ruleset())
        .oneshot(
            Request::builder()
                .uri("/settings")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router is infallible");
    assert_eq!(settings.status(), StatusCode::OK);
    assert!(
        settings.headers()[header::CONTENT_TYPE]
            .to_str()
            .expect("content type is ASCII")
            .starts_with("text/html")
    );

    let widget = app(ruleset())
        .oneshot(
            Request::builder()
                .uri("/widget")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router is infallible");
    assert_eq!(widget.status(), StatusCode::OK);
    assert!(
        widget.headers()[header::CONTENT_TYPE]
            .to_str()
            .expect("content type is ASCII")
            .starts_with("text/html")
    );
}

#[tokio::test]
async fn router_returns_json_for_method_and_route_errors() {
    let router = app(ruleset());

    let (status, _, method) = send(router.clone(), Method::POST, "/v1/status").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert!(method["error"].is_string());

    let (status, _, missing) = send(router, Method::GET, "/missing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(missing["error"].is_string());
}

#[tokio::test]
async fn status_preserves_clock_and_coverage_semantics() {
    let router = app(ruleset());

    let (status, _, current) = send(router.clone(), Method::GET, "/v1/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(current["clock"]["discipline"], "unmeasured");

    let (status, _, outside) = send(
        router.clone(),
        Method::GET,
        "/v1/status?at=2030-01-01T00%3A00%3A00Z",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        outside["venues"]
            .as_array()
            .is_some_and(|venues| venues.iter().all(|venue| venue["status"] == "unknown"))
    );

    for uri in [
        "/v1/status?at=not-a-time",
        "/v1/status?extra=value",
        "/v1/status?at=2026-07-30T02%3A00%3A00Z&at=2026-07-30T03%3A00%3A00Z",
    ] {
        let (status, _, body) = send(router.clone(), Method::GET, uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string());
    }
}

#[tokio::test]
async fn large_json_responses_are_compressed_when_requested() {
    let response = app(ruleset())
        .oneshot(
            Request::builder()
                .uri("/v1/status?at=2026-07-30T02%3A00%3A00Z")
                .header(header::ACCEPT_ENCODING, "gzip")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router is infallible");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
}
