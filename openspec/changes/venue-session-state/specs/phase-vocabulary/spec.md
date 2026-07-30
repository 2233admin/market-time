## ADDED Requirements

### Requirement: One shared phase vocabulary

The system SHALL express every venue's state using one shared phase vocabulary. No venue may
introduce a phase name of its own. (FR-005)

#### Scenario: A venue-specific session maps to a shared phase

- **WHEN** Shanghai Stock Exchange's after-hours fixed-price session is described
- **THEN** it is expressed as a phase drawn from the shared vocabulary
- **AND** no venue-specific phase name is introduced

#### Scenario: All launch venues in one vocabulary

- **WHEN** the schedules of Shanghai, New York, and Binance are described
- **THEN** every phase used comes from the shared vocabulary

### Requirement: Vocabulary coverage

The phase vocabulary SHALL be able to express, at minimum: closed, pre-open, opening auction,
continuous trading, mid-day break, closing auction, post-close, and non-trading interruption.
(FR-006)

#### Scenario: An interruption is expressible

- **WHEN** a venue publishes an unscheduled halt or maintenance window inside a stretch its
  schedule says is trading
- **THEN** that stretch is expressed as a non-trading interruption rather than as closed

### Requirement: Events are not phases

The system SHALL represent scheduled recurring occurrences that are not states — such as crypto
funding settlements — as events overlaid on a phase, not as phases themselves. (FR-007)

#### Scenario: Funding settlement during continuous trading

- **WHEN** a Binance funding settlement occurs
- **THEN** it is returned as an event overlaid on the phase in effect
- **AND** the phase at that instant remains the trading phase

#### Scenario: Events and phases cannot be conflated

- **WHEN** an event kind and a phase kind are compared
- **THEN** they are values of separate closed vocabularies, so one cannot be substituted for the
  other

### Requirement: Phases tile time, events do not

Phases SHALL cover all time within coverage without gaps or overlaps; events SHALL NOT be
required to do so. (FR-008)

#### Scenario: No gap inside coverage

- **WHEN** any instant inside declared coverage is queried
- **THEN** exactly one phase applies — never zero, never two

#### Scenario: Incomplete source data cannot be papered over

- **WHEN** a venue's captured schedule leaves an interval with no assigned phase
- **THEN** the dataset fails the tiling invariant and is not shippable
- **AND** the missing interval is resolved from the venue's published rules, never by inference

#### Scenario: Sparse events are legal

- **WHEN** a stretch of covered time contains no scheduled events
- **THEN** the answer is valid with an empty event set
