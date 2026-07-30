## ADDED Requirements

### Requirement: Regional session bands are derived, not published

The system MAY present the trading day as regional session bands — the conventional Sydney,
Tokyo, London, and New York view of the FX day — and each band SHALL be derived from the
schedules of the venues that constitute it, marked derived, and carry the reasoning and the
venue set it was derived from.

#### Scenario: A band is inspected

- **WHEN** a session band is inspected
- **THEN** it is identified as derived
- **AND** it names the venues and rules it was derived from

#### Scenario: No published band is claimed

- **WHEN** no venue publishes a "London session" as a schedule of its own
- **THEN** the band is never presented as a published fact

### Requirement: Overlap windows are computed, not asserted

Where two regional bands overlap, the overlap window SHALL be computed from the bands it
overlaps and SHALL carry uncertainty no narrower than the widest uncertainty of its inputs.

#### Scenario: Two bands overlap

- **WHEN** two bands share a stretch of time
- **THEN** the overlap window is that shared stretch
- **AND** its uncertainty is at least as wide as the least precise band boundary involved

#### Scenario: An input is out of coverage

- **WHEN** one constituent venue is outside its declared coverage for part of the interval
- **THEN** the band is unknown for that part rather than being drawn from the remaining venues
