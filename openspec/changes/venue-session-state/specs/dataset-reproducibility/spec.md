## ADDED Requirements

### Requirement: Answers name their dataset revisions

The system SHALL report which dataset revisions produced an answer. (FR-016)

#### Scenario: Attribution on every answer

- **WHEN** any answer is returned
- **THEN** it carries the identifiers of the dataset revisions that produced it

#### Scenario: Revisions are immutable

- **WHEN** a venue's rules change
- **THEN** a new revision is created with a link to the revision it supersedes
- **AND** the superseded revision is not edited in place

### Requirement: Same inputs, same answer

Given the same dataset revisions and the same query, the system SHALL return the same answer.
(FR-017)

#### Scenario: Repeated query

- **WHEN** a sample of queries is re-run against unchanged dataset revisions
- **THEN** the answers are byte-identical

#### Scenario: Reproducible across hosts

- **WHEN** the same query runs against the same revisions on a different machine
- **THEN** the answer is identical, because zone rules come from the build's pinned tzdata
  release rather than from the host

### Requirement: Coverage is declared, not assumed

Each dataset SHALL declare the range of time it is valid for, and the explicit-unknown rule
SHALL be enforced against that declaration. (FR-018)

#### Scenario: A shallow range is honest

- **WHEN** a dataset covers only the range its venue has published
- **THEN** that range is declared
- **AND** queries beyond it return unknown rather than an extrapolated schedule

#### Scenario: The declaration is what unknown is measured against

- **WHEN** coverage is checked for a queried instant
- **THEN** the check reads the dataset's own declared range, not an inferred or hardcoded one
