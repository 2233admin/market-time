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

**Primary consumer: autonomous agents that trade**, and other systems. Humans reading a board
are a real but secondary audience.

That is a design constraint, not positioning. A human sees that a rendered number looks
approximate; an agent does not. So uncertainty and unknown live in the returned value and in the
type system, never only in the rendering. An unknown and a closed market are different values,
not one value shown two ways. If a fact would only reach a human through visual nuance, it is not
yet modelled.

Open source, public repository. The code, the phase model, the mappings, and the schema are the
open part. **Venue data is never shipped here** — see "Data sourcing rules".

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

[`CONSTITUTION.md`](CONSTITUTION.md) is the ratified constitution and it wins over habit,
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

Work flows through [OpenSpec](https://github.com/Fission-AI/OpenSpec) 1.6.0. Spec material is
committed under `openspec/`; the agent skills and slash commands that drive it are generated,
not committed. Run once per clone:

```
npx @fission-ai/openspec@1.6.0 init --tools claude,codex
```

Then:

```
/opsx:propose → /opsx:apply → /opsx:archive
```

Layout:

- `openspec/changes/<change>/proposal.md` — why, what changes, which capabilities
- `openspec/changes/<change>/design.md` — technical decisions and trade-offs
- `openspec/changes/<change>/tasks.md` — the work, in order
- `openspec/changes/<change>/specs/<capability>/spec.md` — requirement deltas
- `openspec/specs/<capability>/spec.md` — current truth, written by archiving a change

The constitution gates design. Every principle above must be checked before the work starts and
again after the design settles. A violation carried forward is recorded in `design.md` under
Risks / Trade-offs with the rejected simpler alternative — never left silent.

Deep reference material for a slice (research, data model, contracts, quickstart) lives in
`docs/<slice>/` rather than in the change folder, because it outlives the change.

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

### No venue data ships from this repository. Ever.

All three launch venues were checked in full and none permits commercial redistribution of its
published schedule; none carves out factual or calendar data.

| Venue | Governing text | Position |
|---|---|---|
| SSE | Trading Rules Art. 5.1.3 | use or publication requires Exchange permission |
| NYSE | ICE Terms of Use | personal, non-commercial only; "systematic retrieval to create collections, compilations, databases" named explicitly |
| Binance | ADGM Terms cl. 27 | non-commercial personal or internal business use only |

Consequences that bind implementation:

- `market-time-data` is the **entire ingestion architecture**, not a convenience layer.
  Fetch-at-run-time is the only compliant shape.
- A pull request that vendors a venue calendar into this repository is rejected on sight,
  regardless of how small or how obviously factual the data looks.
- Mark Time is a **client, not a redistributor**. The operator fetches under their own
  relationship with each venue and is responsible for their own compliance.
- Where a venue offers a permission path, taking it is the intended route. SSE states one
  explicitly.
- A source's terms are recorded at registration alongside its evidence, so "under what terms
  did we obtain this" is answerable per record — same discipline as `source_url` and
  `fetched_at`, and for the same reason.

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
