## ADDED Requirements

### Requirement: Phase at an instant

The system SHALL return the phase a named venue is in for any queried instant that falls within
that venue's declared coverage. (FR-001)

#### Scenario: Continuous trading

- **WHEN** the phase of Shanghai Stock Exchange is requested at 10:00 Shanghai time on a normal
  trading day
- **THEN** the answer is continuous trading

#### Scenario: A state that is neither open nor closed

- **WHEN** the phase of Shanghai Stock Exchange is requested at 12:00 Shanghai time on a normal
  trading day
- **THEN** the answer is the mid-day break — not closed and not open

#### Scenario: An always-on venue

- **WHEN** the phase of Binance is requested at any instant inside coverage
- **THEN** the answer is a trading phase, and closed never occurs for that venue

### Requirement: Explicit unknown outside coverage

The system SHALL return an explicit unknown for any instant outside declared coverage, and SHALL
NOT extrapolate a schedule past its verified range. (FR-002)

#### Scenario: Beyond the loaded calendar

- **WHEN** a phase is requested for an instant beyond the declared coverage of the venue's
  dataset
- **THEN** the answer is an explicit unknown that names the coverage boundary
- **AND** no extrapolated phase is returned

#### Scenario: Unknown is a value, not an error

- **WHEN** a caller receives an unknown answer
- **THEN** the unknown is a variant of the answer type, distinct from closed, and cannot be
  silently rendered as a phase

### Requirement: Phase start and end

The system SHALL report the start and end of the returned phase, subject to the same evidence
and uncertainty rules as the phase itself. (FR-003)

#### Scenario: Boundaries carry their own uncertainty

- **WHEN** a phase is returned for New York Stock Exchange around the scheduled open
- **THEN** the start is reported with the uncertainty appropriate to a process start rather than
  as an exact instant

### Requirement: Deterministic boundary ownership

The system SHALL resolve boundary instants deterministically, so that every instant within
coverage belongs to exactly one phase. (FR-004)

#### Scenario: The exact boundary instant

- **WHEN** a phase is requested for the exact instant at which one phase ends and the next
  begins
- **THEN** the answer states unambiguously which phase owns that instant

#### Scenario: First and last instants of coverage

- **WHEN** a phase is requested for the first or last instant inside declared coverage
- **THEN** a phase is returned, and the instants immediately outside return unknown

### Requirement: Multi-venue view at one instant

Users SHALL be able to obtain the phases of all tracked venues at a single instant in one
request. (FR-019)

#### Scenario: Three venues, three different states

- **WHEN** the global view is requested at an instant where Shanghai is on its mid-day break,
  New York is closed, and Binance is trading
- **THEN** all three states appear correctly in one response

#### Scenario: Per-venue entries are full answers

- **WHEN** any single venue's entry in the global view is inspected
- **THEN** it carries the same evidence and uncertainty a single-venue query would return

### Requirement: One coverage gap does not void the view

In a multi-venue request, a venue outside its coverage SHALL report unknown without suppressing
the other venues' answers. (FR-020)

#### Scenario: One venue out of coverage

- **WHEN** the global view is requested at an instant where one venue is outside its declared
  coverage
- **THEN** that venue reports unknown
- **AND** the remaining venues still report their phases
