## ADDED Requirements

### Requirement: Long-running HTTP shell

The system SHALL expose the existing market-time queries through a long-running HTTP process
without adding domain logic to the shell.

#### Scenario: Service starts from an operator dataset

- **WHEN** the operator supplies a valid dataset path and bind address
- **THEN** the service loads the dataset once and listens until stopped

### Requirement: Machine-readable venue catalog

The system SHALL return each venue identifier, display metadata, home zone, asset family, and
declared coverage together with dataset and tzdb revisions.

### Requirement: One-instant multi-venue status

The system SHALL answer all venues against one shared instant and preserve known, unknown,
uncertainty, evidence, events, and dataset revisions in JSON.

#### Scenario: Caller omits an instant

- **WHEN** `GET /v1/status` has no `at` parameter
- **THEN** the service reads the host clock once and marks its discipline unmeasured

#### Scenario: Caller supplies an instant

- **WHEN** `GET /v1/status?at=<UTC RFC3339>` is valid
- **THEN** every venue is resolved against exactly that instant

#### Scenario: Caller supplies invalid input

- **WHEN** the method, route, or `at` parameter is unsupported or malformed
- **THEN** the service returns a JSON error with an appropriate 4xx status

### Requirement: Bounded observable HTTP runtime

The system SHALL process requests concurrently within a finite in-flight bound, time out stalled
requests, assign a request identifier, emit configurable request traces, and compress eligible
responses without exposing those transport concerns to the calculation core.

#### Scenario: Browser reads the public timetable

- **WHEN** a browser sends a cross-origin GET request
- **THEN** the service allows the request and exposes its generated request identifier

#### Scenario: Request exceeds the processing deadline

- **WHEN** request processing exceeds the configured service deadline
- **THEN** the service returns a machine-readable timeout error

#### Scenario: Operator stops the service

- **WHEN** the process receives Ctrl-C or SIGTERM
- **THEN** it stops accepting new work and permits in-flight responses to complete
