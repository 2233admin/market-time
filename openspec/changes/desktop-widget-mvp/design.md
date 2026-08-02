## Design

The `market-time-desktop` Tauri crate is an outer adapter. Rust owns only the native window,
system tray, close-to-hide behavior, and notification plugin. It loads the static `/widget` page
from the existing Next export.

The widget polls `GET /v1/timeline` and then `GET /v1/status?at=<timeline.at>`. Rendering and
reminder scheduling accept only validated response contracts. The client never reconstructs a
calendar or session. A successful response instant is paired with `performance.now()` so visible
countdowns advance on a monotonic clock; when the HTTP connection becomes stale, the instant and
status freeze as a labelled last-known snapshot and reminders are cancelled.

Reminder preferences remain local presentation state. An absent venue filter means every venue;
an empty custom selection means none, and otherwise only selected server venue IDs may notify. Each server transition yields at most one
approaching reminder and one reached-boundary reminder, deduplicated by transition identity.

## Constitution Check

- I — Pass. The widget reads evidence from `/v1/status` and never manufactures provenance.
- II — Pass. Unknown remains distinct from closed, offline data is labelled stale, and displayed
  host time is second-level UI without an accuracy claim.
- III — Pass. Dataset and TZDB revisions remain visible from server responses.
- IV — Pass. Core is unchanged; HTTP, UI, tray, clock reads, and notifications stay in adapters.
- V — Pass. Reminder selection/scheduling and the new static route are locked by failing tests
  before implementation.

## Risks / Trade-offs

- The MVP expects an already-running HTTP service at `http://127.0.0.1:8080`. A build may supply
  one exact HTTP(S) origin through `NEXT_PUBLIC_MARK_TIME_API`; the same value pins the desktop CSP.
  Starting or bundling the service is deliberately deferred.
- Native notifications are most representative from an installed Windows build; development
  notifications may use the host process identity.
