## ADDED Requirements

### Requirement: Optional desktop widget

The system SHALL provide a small draggable always-on-top desktop client that can be shown, hidden,
or fully exited without changing the HTTP service or core library.

#### Scenario: Operator closes the widget

- **WHEN** the operator closes the floating window
- **THEN** the window hides and remains available from the system tray

#### Scenario: Operator exits from the tray

- **WHEN** the operator selects Quit
- **THEN** the desktop process terminates completely

### Requirement: Server-owned market truth

The widget SHALL render current state and future boundaries only from validated stable HTTP
responses and SHALL keep unknown distinct from closed.

#### Scenario: HTTP service becomes unavailable

- **WHEN** the latest response becomes stale or a refresh fails
- **THEN** the widget labels any retained payload as a frozen last-known snapshot, stops live
  countdowns, and suppresses reminders

### Requirement: Honest desktop clock and provenance

The widget SHALL expose server clock discipline, evidence availability, TZDB, and dataset revision
without claiming that the displayed host time has nanosecond accuracy.

### Requirement: Scoped boundary reminders

The widget SHALL allow all reminders to be disabled and SHALL support limiting reminders to
selected server venue IDs. Notifications SHALL be derived only from `next_trading_transition`.
