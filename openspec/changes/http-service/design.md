## Design

`market-time-server` loads a dataset once, then routes read-only GET requests to existing core
queries. Axum owns routing and extraction, Tokio owns the concurrent runtime, and Tower middleware
owns the production HTTP envelope. The service owns JSON presentation and no domain rules. The
server accepts `--dataset <path>` and optional `--bind <address>` arguments.

Endpoints:

- `GET /health`
- `GET /v1/venues`
- `GET /v1/status`
- `GET /v1/status?at=<UTC RFC3339 instant>`

The response keeps known, unknown, uncertainty, evidence, and dataset revisions as data. A
malformed `at` is a `400`, an unsupported method is `405`, and an unknown route is `404`. Errors
produced by routing and request timeouts keep the same JSON error contract.

The built-in HTTP envelope provides:

- a process-wide maximum of 256 in-flight requests;
- a 10-second request timeout;
- generated and propagated `x-request-id` values;
- request tracing controlled through `RUST_LOG`;
- GET-only cross-origin access and response compression; and
- graceful Ctrl-C/SIGTERM shutdown.

## Constitution Check

- I — Pass. Status responses preserve the evidence already attached by the core.
- II — Pass. Unknown and uncertainty remain explicit; supplied instants must be UTC RFC3339.
- III — Pass. The ruleset is loaded once and responses report dataset and tzdb revisions.
- IV — Pass. Network and host-clock reads live only in the new shell crate.
- V — Pass. HTTP contracts are written and observed failing before implementation.

## Risks / Trade-offs

- The in-process concurrency limit protects one server instance, but it is not a caller-aware rate
  limit. Per-client quotas belong at an authenticated API edge once caller identity exists.
- The loaded ruleset remains immutable for one process lifetime. Dataset activation still uses a
  restart until an atomic revision-switching contract is specified.
