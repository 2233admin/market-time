## 1. Contract

- [x] 1.1 Add failing tests for health, venues, status, invalid input, methods, and routes.

## 2. Implementation

- [x] 2.1 Add the standalone service crate and startup argument parsing.
- [x] 2.2 Implement the thin JSON router and persistent HTTP loop.
- [x] 2.3 Use the production async HTTP stack with timeouts, concurrency bounds, request IDs,
      tracing, CORS, compression, and graceful shutdown.

## 3. Verification

- [x] 3.1 Run format, Clippy, workspace tests, and a real HTTP smoke check.
- [x] 3.2 Exercise the real router and middleware stack in integration tests.

## 4. Global Trading Clock MVP

- [x] 4.1 Serve the dependency-free, responsive clockboard and its static assets.
- [x] 4.2 Render server-provided phases, boundaries, calendar exceptions, evidence, revisions,
      and unknown outcomes without reproducing session calculation in the browser.
