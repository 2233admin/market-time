# Mark Time — agent rules

Read this before writing anything in this repository.

## What this is

Auditable time infrastructure for global financial markets: what time is it, and what is
open, where. Global city clocks, exchange trading sessions and phases, crypto funding and
maintenance windows.

Two layers, in this order:

1. **Baseline** — a global trading-hours board. What is open, what is closed, what is in
   between, across venues, at a glance.
2. **The reason this exists** — every answer on that board is nanosecond-represented,
   traceable to a dated source document, and honest about how precisely it is known.

Plenty of products do layer 1. Layer 2 is the product.

## Standalone. Not a subsystem of anything.

Mark Time is an independent product with its own repository, its own release cycle, and its
own reasons. It is **not** a component of `k-atana` or of any other project on this machine,
and it must not be shaped by any single downstream consumer.

Concretely:

- Venue coverage is chosen on Mark Time's own merits, never because some other project
  happens to trade that venue.
- Schema, field names, and contracts are designed for this domain, not copied from another
  repository's data contract to make integration convenient.
- Nothing in this repository may import, reference, or depend on another local project.
- Downstream consumers integrate against Mark Time's published interface. Mark Time does not
  know they exist.

If a requirement can only be justified as "project X needs it", it does not belong here yet.

## Governance

`.specify/memory/constitution.md` is the ratified constitution and it wins over habit,
runbooks, and convenience. Read it before designing anything. Current version is recorded in
its footer.

Five principles, two non-negotiable:

| | Principle | Short form |
|---|---|---|
| I | Evidence-Backed Rules **(NON-NEGOTIABLE)** | Every rule carries `source_url`, `fetched_at`, `effective_from`. Derived is never presented as observed. |
| II | Explicit Instants, Explicit Uncertainty | Nanosecond representation, declared time scale, leap-second-aware conversion. Outside coverage returns explicit unknown. Precision is not accuracy. |
| III | Reproducible Rule Data | Dataset revisions are immutable. Same revisions plus same query returns the same answer. |
| IV | Library-First Core, Thin Shells | Core has no I/O, no network, and **no clock reads** in its decision path. |
| V | Test-First with Golden Vectors **(NON-NEGOTIABLE)** | DST both directions, holidays, shortened sessions, boundaries, out-of-coverage. Every defect becomes a permanent vector. Vectors are never deleted to make a build pass. |

Amendments go through the constitution's own amendment procedure and a version bump. Do not
work around a principle; amend it or comply.

## Workflow

Work flows through spec-kit. Skills live in `.agents/skills/`.

```
/speckit-specify → /speckit-plan → /speckit-tasks → /speckit-implement
```

`.specify/feature.json` points at the active feature directory. Constitution Check in the
plan template gates on the principles above and must pass before Phase 0 research and again
after Phase 1 design. A violation carried forward is recorded in the plan's Complexity
Tracking table with the rejected simpler alternative — never left silent.

## Data sourcing rules

- Read `DATA-LICENSING.md` before ingesting anything. Code license is `MIT OR Apache-2.0`;
  it does not extend to rule data.
- An upstream's terms are checked and recorded **at source registration**, before any
  programmatic ingestion. Redistribution is never assumed.
- Sources whose terms forbid redistribution are referenced and fetched, never vendored into
  this repository.
- Prefer first-party sources. A venue's own published schedule is evidence; an aggregator
  repeating it is a second-hand claim and a licensing problem. (Observed 2026-07-29: a
  well-known Chinese financial aggregator's terms explicitly forbid reuse of its data for AI
  training or commercial purposes. Aggregators are useful as design references for what to
  display, never as data sources.)

## Time correctness rules

- The core never reads a clock. `now` is passed in from a shell.
- UTC is the published scale. TAI, GNSS system time, and host monotonic scales are accepted
  only when explicitly tagged, and converted leap-second-aware — never by adding a constant.
- Wide-area internet time synchronisation does not reach nanoseconds. A surface reporting
  "now" must expose the host's own clock discipline bounds as uncertainty.
- Published boundaries and observed boundaries are different claims. Never conflate them.

## Anti-patterns

- Shaping a schema or a venue list around what one downstream project needs.
- Treating nanosecond representation as a nanosecond accuracy claim.
- Extrapolating a calendar past its declared coverage instead of returning unknown.
- Overwriting a dataset in place rather than producing a new revision.
- Putting domain logic in a shell (CLI, service, board) instead of the core.
- Vendoring upstream data whose terms were never checked.
