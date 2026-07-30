## ADDED Requirements

### Requirement: Every answer carries its evidence

Every answer SHALL carry the evidence for the rules that produced it: the source document, when
it was retrieved, and the date from which it takes effect. (FR-009)

#### Scenario: Evidence is reachable

- **WHEN** any answer is returned
- **THEN** it includes at least one source reference a person can open and independently verify

#### Scenario: Evidence is mandatory, not optional

- **WHEN** a rule is constructed without evidence
- **THEN** it cannot be built, so no answer can rest on an unevidenced rule

### Requirement: Derived rules are marked

The system SHALL mark any rule that was derived or inferred rather than published, and SHALL
carry the reasoning with it. Derived SHALL NOT be presented as observed. (FR-010)

#### Scenario: A derived rule ships with its reasoning

- **WHEN** a rule is marked derived
- **THEN** its derivation reasoning is present
- **AND** a derived rule without reasoning is rejected

### Requirement: Uncertainty on every answer

Every answer SHALL carry an uncertainty statement that reflects the precision of its underlying
source, and SHALL NOT present an answer as more precise than its evidence supports. (FR-011)

#### Scenario: Precision is not accuracy

- **WHEN** a rule's source publishes boundaries to the minute
- **THEN** the returned uncertainty is no narrower than one minute, regardless of the internal
  nanosecond representation

### Requirement: Published bounds are a floor

Where a venue publishes its own imprecision, the stated uncertainty SHALL be no narrower than
what the venue published. (FR-011a)

#### Scenario: Binance publishes a deviation

- **WHEN** a funding settlement time is returned
- **THEN** its uncertainty is at least the venue-published deviation of 15 seconds
- **AND** that bound is taken from the venue rather than estimated

### Requirement: A process start is not an instant

Where a venue's published boundary is the scheduled start of a process rather than an instant at
which the market changes state, the answer SHALL reflect that the boundary has a spread. A
process start SHALL NOT be presented as an instantaneous transition. (FR-011b)

#### Scenario: The NYSE open

- **WHEN** the start of continuous trading at New York Stock Exchange is returned
- **THEN** it is characterised as the scheduled start of a security-by-security opening process
- **AND** it is not presented as a market-wide state change at an exact instant

### Requirement: Published and observed stay distinct

The system SHALL keep a venue's published schedule and its observed behaviour as distinct
claims, and SHALL NOT present one as the other. (FR-012)

#### Scenario: Published-only slice

- **WHEN** an answer is produced from published schedules
- **THEN** it is identifiable as published rather than observed, so observed data can be added
  later without redefining the answer
