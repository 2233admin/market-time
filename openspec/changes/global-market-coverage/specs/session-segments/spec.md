## ADDED Requirements

### Requirement: A trading day may hold several blocks

The system SHALL express a venue whose trading day is split into separate blocks — a main block
and a night block, or several blocks with breaks between them — as ordinary phases with the
non-trading stretches between them expressed as phases too.

#### Scenario: A night session

- **WHEN** the timeline for a venue that trades a night block is requested across a full day
- **THEN** both blocks appear as trading segments
- **AND** the stretch between them is a phase of its own, not a gap

#### Scenario: A block that crosses midnight

- **WHEN** a block starts on one local date and ends on the next
- **THEN** it is one continuous segment on the absolute timeline, not two

### Requirement: Block role rides alongside the phase, never as a phase

Where a venue distinguishes a main block from a night or extended block, the system SHALL carry
that distinction as a role attached to the segment, and SHALL NOT introduce a phase name for it.
The shared phase vocabulary stays closed.

#### Scenario: Labelling a night block

- **WHEN** a night block is returned
- **THEN** its phase is drawn from the shared vocabulary
- **AND** its role identifies it as a night block

#### Scenario: A consumer that ignores roles

- **WHEN** a consumer reads only the phase and ignores the role
- **THEN** the answer is still correct — the role adds detail rather than changing meaning

### Requirement: Roles are evidenced like any other rule

A block role SHALL carry the same evidence as the rule that produced the block, and SHALL be
marked derived where the venue does not publish the distinction itself.

#### Scenario: A venue that names its own night session

- **WHEN** the venue publishes the night session as such
- **THEN** the role is evidenced against that publication and is not marked derived

#### Scenario: A distinction we drew ourselves

- **WHEN** the role is our reading rather than the venue's wording
- **THEN** it is marked derived and carries the reasoning
