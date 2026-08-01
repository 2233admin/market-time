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
            .contains("MARK / TIME")
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
