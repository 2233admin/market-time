## Why

Mark Time can answer one instant through its CLI, but it cannot yet serve those answers to a
long-running consumer. The global trading-hours board needs a small HTTP boundary over the
existing core so a web surface or another machine can query the loaded dataset.

## What Changes

- Add a standalone `market-time-server` crate as a thin HTTP shell.
- Load one immutable dataset revision set at startup.
- Expose health, venue-catalog, and multi-venue phase-status JSON endpoints.
- Read the host clock only in the server shell when a request does not supply `at`.
- Serve a small static global trading-clock MVP that renders only the API's stated status,
  calendar, evidence, revision, and uncertainty semantics.

## Explicitly Not in Scope

- Shipping real venue calendars.
- Authentication, TLS termination, deployment manifests, or a feature-rich web application.
- Moving time, schedule, or uncertainty logic out of `market-time-core`.
