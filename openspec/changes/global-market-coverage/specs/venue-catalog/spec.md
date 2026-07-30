## ADDED Requirements

### Requirement: Venues carry display metadata

Each tracked venue SHALL carry the metadata a board needs to present it: a display name, the
city or region it is identified by, its home time zone, and the asset-class family it belongs to.

#### Scenario: Rendering a venue row

- **WHEN** a board renders a venue
- **THEN** the name, location, and family come from the catalog rather than from the board's own
  table

#### Scenario: Families are a closed set

- **WHEN** a venue is added to the catalog
- **THEN** its family is one of the declared families, so grouping never depends on string
  matching a venue name

### Requirement: Adding a venue is data, not code

Adding a venue SHALL require a catalog record, a dataset revision, and a source registration —
and SHALL NOT require a change to the phase vocabulary, the resolution logic, or the board.

#### Scenario: A fourth venue

- **WHEN** a venue outside the launch set is added
- **THEN** no new phase name is introduced
- **AND** no venue-specific branch is added to resolution

#### Scenario: Terms are checked before ingestion

- **WHEN** a venue is registered as a source
- **THEN** its published terms are recorded with it, before any programmatic ingestion

### Requirement: Coverage is declared per venue

Each venue SHALL declare its own coverage range, and a query outside that range SHALL return
unknown for that venue only.

#### Scenario: Uneven coverage across the catalog

- **WHEN** venues in the catalog have different coverage ranges
- **THEN** each venue is judged against its own declaration
- **AND** a venue outside its range reports unknown without affecting the others
