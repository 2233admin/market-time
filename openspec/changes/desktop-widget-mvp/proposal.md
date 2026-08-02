## Why

Operators need a glanceable Windows surface for Market Time without keeping the full board in
front of other work. The desktop surface must remain an optional client of the existing HTTP
service, not a second schedule engine.

## What Changes

- Add a small always-on-top Tauri window around a dedicated Next static page.
- Add tray show, hide, and quit controls; closing the window hides it instead of terminating it.
- Render important market state, local venue time, the next server-provided transition, and a
  monotonic countdown anchored to the server response instant.
- Send optional native reminders for approaching and reached open/close transitions, globally or
  for selected venues.
- Keep unknown, offline snapshots, evidence availability, clock discipline, and dataset revisions
  explicit.

## Explicitly Not in Scope

- NTP, PTP, GNSS, hardware clocks, or any claim of nanosecond clock accuracy.
- Trading-session, holiday, or transition calculation in the desktop client.
- Shipping venue data, starting the HTTP service, auto-update, or single-instance handling.
