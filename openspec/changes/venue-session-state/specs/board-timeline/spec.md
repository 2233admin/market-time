## ADDED Requirements

### Requirement: Phase timeline over an interval

The system SHALL return, for a named venue and a queried interval, the ordered sequence of
phases covering that interval, each with its start, its end, its evidence, and its uncertainty.

#### Scenario: A full trading day

- **WHEN** the timeline for Shanghai Stock Exchange is requested over one local trading day
- **THEN** the returned segments tile the whole interval with no gaps and no overlaps
- **AND** the mid-day break appears as its own segment rather than being merged into trading

#### Scenario: A day with several trading blocks

- **WHEN** the timeline for a venue whose day contains more than one trading block is requested
- **THEN** each block appears as a separate segment with its own boundaries

#### Scenario: Partial coverage inside the interval

- **WHEN** the requested interval extends past the venue's declared coverage
- **THEN** the covered part returns phase segments
- **AND** the remainder returns unknown segments naming the coverage boundary, rather than the
  interval being rejected

### Requirement: The board renders a timeline, not a status word

The board SHALL present each venue as its phases laid out across the queried interval, with a
marker for the instant being viewed, and SHALL derive every segment from the timeline the core
returned rather than from any schedule of its own.

#### Scenario: Segments come from the core

- **WHEN** the board renders a venue row
- **THEN** every segment's kind, start, and end come from the core's timeline answer
- **AND** the board contains no venue schedule, no phase-derivation logic, and no fallback
  schedule of its own

#### Scenario: Many venues at once

- **WHEN** the board renders all tracked venues over the same interval
- **THEN** each row is the same interval on the same axis, so venues are comparable by position

### Requirement: The viewer's time zone is presentation only

The board SHALL let the viewer choose the zone the timeline axis is labelled in, and changing
that choice SHALL NOT change which phases are returned or when they start and end.

#### Scenario: Switching the axis zone

- **WHEN** the viewer switches the axis from one zone to another
- **THEN** the segments occupy the same absolute instants
- **AND** only their labelled positions on the axis change

#### Scenario: The core stays UTC

- **WHEN** the board requests a timeline
- **THEN** it passes and receives UTC instants, and performs zone labelling itself

### Requirement: The now marker carries the host clock's bound

Where the board marks the present instant, it SHALL read `now` from its own host, pass it into
the core, and present the host's clock discipline bound as uncertainty. It SHALL NOT present the
host clock as exact.

#### Scenario: Rendering the present

- **WHEN** the board draws the marker for the present instant
- **THEN** the marker is accompanied by the bound within which the host's clock is known
- **AND** the core is never asked to read a clock

### Requirement: Unknown is visually distinct from closed

The board SHALL render an out-of-coverage stretch as unknown, in a form a viewer cannot mistake
for closed, and SHALL NOT resolve a display difficulty by hiding uncertainty.

#### Scenario: Beyond coverage

- **WHEN** part of the rendered interval falls outside a venue's declared coverage
- **THEN** that stretch reads as "not known", distinct from the closed rendering
- **AND** no schedule is drawn across it

#### Scenario: A boundary that is a process

- **WHEN** a segment boundary is the scheduled start of a process rather than an instant
- **THEN** the rendering shows that the boundary has a spread rather than drawing a hard edge

### Requirement: Every rendered segment reaches its evidence

From any segment on the board, a viewer SHALL be able to reach the source document behind the
rules that produced it, without leaving the product to look it up.

#### Scenario: Inspecting a segment

- **WHEN** a viewer inspects a rendered segment
- **THEN** the source document, its retrieval date, its effective date, and whether the rule was
  published or derived are all reachable from that segment
