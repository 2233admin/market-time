## ADDED Requirements

### Requirement: Nanosecond instants on a declared scale

The system SHALL accept and return instants at nanosecond resolution with an explicitly declared
time scale. (FR-013)

#### Scenario: Answers are published on one scale

- **WHEN** an answer is returned
- **THEN** its instants are UTC at nanosecond resolution

#### Scenario: Another scale must be labelled

- **WHEN** an input instant is supplied on a non-UTC scale
- **THEN** it is accepted only when its scale is explicitly labelled
- **AND** conversion into UTC happens at the one declared seam, not scattered through the core

### Requirement: Daylight saving is resolved, not assumed

The system SHALL correctly resolve venues in zones that observe daylight saving, including local
times that do not exist and local times that occur twice. (FR-014)

#### Scenario: Spring forward

- **WHEN** a New York local wall-clock time that does not exist on the day the clocks move
  forward is resolved
- **THEN** the result is a nonexistent-time outcome
- **AND** no instant is silently invented

#### Scenario: Fall back

- **WHEN** a New York local wall-clock time that occurs twice is resolved
- **THEN** the result either identifies which occurrence is meant or reports the ambiguity
- **AND** one occurrence is not picked silently

#### Scenario: Zone rules are pinned

- **WHEN** the same query is run on two different hosts
- **THEN** both resolve zone rules from the build's pinned tzdata rather than the host's
  zoneinfo, so both return the same answer

### Requirement: Holidays and shortened sessions win

The system SHALL apply venue holidays and shortened sessions in preference to the normal weekly
schedule. (FR-015)

#### Scenario: A market holiday

- **WHEN** a phase is requested for New York Stock Exchange on a US market holiday
- **THEN** the answer is closed
- **AND** the holiday is identified

#### Scenario: An early close

- **WHEN** a phase is requested during a New York Stock Exchange early-close session
- **THEN** the shortened schedule is reflected rather than the normal one
