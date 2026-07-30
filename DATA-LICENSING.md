# Data Licensing Policy

This document is **project policy, not legal advice**. It explains how Mark Time
licenses what it ships, and specifically that the code license does not extend to
the rule data (exchange calendars, holiday tables, session times, venue notices)
that the project models and, where permitted, redistributes.

This matters because Mark Time is a public repository. Publishing upstream-sourced
exchange calendar data under a permissive license the project does not hold the
rights to would be a real legal problem, not a formality — so the boundary below is
explicit and applies before any data ships, not after.

## The three tiers

| Tier | What it covers | How it is licensed |
|---|---|---|
| Code | The Rust crates in this repository | `MIT OR Apache-2.0` |
| Original project data | Work genuinely authored here: the phase/session model, venue-to-phase mappings, schema definitions | **CC0-1.0** (see rationale below) |
| Upstream-sourced data | Exchange calendars, holiday tables, published session times, venue maintenance notices | **Not relicensed.** Each record retains its own source and terms |

**Code** is Mark Time's own work product and carries the same dual license as the
rest of the project: `MIT OR Apache-2.0`, at the user's option.

**Original project data** is data Mark Time authored itself — not copied or derived
from a specific upstream publication. The canonical example is the phase
vocabulary required by the constitution's Domain and Data Constraints (mapping
auction-plus-break cash equities, DST-sensitive continuous markets, and 24/7 crypto
funding/maintenance windows onto one documented phase model), plus the schemas
that shape how any rule is recorded. This project's choice is **CC0-1.0**: this
data has no factual content worth protecting on its own (it is structure, not a
disputed fact), so the least friction for downstream reuse — outright public
domain dedication — beats a share-alike or attribution license that would just
create paperwork nobody needs.

**Upstream-sourced data** is not Mark Time's to relicense. An exchange's holiday
calendar, a published session-time table, a crypto venue's maintenance
announcement — these are facts and documents owned or published by someone else,
under whatever terms that someone else set. Mark Time cannot grant rights to that
data that it does not itself hold. Each such record keeps its own source and
terms, tier for tier, record for record.

## The rule before ingestion

Per the constitution's licensing constraint (Domain and Data Constraints): before
any upstream source is ingested programmatically, its terms MUST be checked and
the finding recorded at source registration. Redistribution rights are never
assumed. This document does not relax that rule — it restates it as the operative
licensing gate. A source with no clear redistribution grant is treated as
non-redistributable until proven otherwise, not the reverse.

## Provenance as the licensing audit trail

Principle I of the constitution already requires every rule to carry
`source_url`, `fetched_at`, `effective_from`, and — where the upstream publishes
one — `source_updated_at`. These fields exist primarily to make an answer
auditable, but they double as the licensing audit trail: `source_url` identifies
whose terms apply to a given record, and `fetched_at` timestamps when this
project's use of that source began. There is no separate licensing ledger to
maintain — the evidence fields already mandated for correctness are the same
fields consulted when a licensing question about a specific record comes up.

## When redistribution is not permitted

Mark Time may still model and reference an upstream source whose terms do not
permit redistribution. What changes is the shape of the integration: the dataset
is **not vendored into this repository**. Instead, the source is registered with
its evidence record as usual, and access to its data is a fetch-at-build-time or
fetch-at-run-time step against the upstream directly, with the evidence record
retained alongside. The project references the source; it does not carry a copy
of the source's data in its own history. This is policy for every non-redistributable
source, not a case-by-case judgment call made later.

## Downstream users' responsibility

Consuming Mark Time's code under `MIT OR Apache-2.0` does not grant any rights to
upstream data the code may reference or fetch. Where Mark Time reads from a
source whose terms restrict redistribution, use, or relicensing, downstream users
of Mark Time are responsible for satisfying those upstream terms themselves —
the same way using an HTTP client does not grant rights to whatever a server
happens to return.

The checked-in `crates/market-time-data/fixtures/synthetic-venues.json` is authored test
material, not a calendar, schedule, or factual claim about any real venue. Its three
`SYNTH-` venues exist only to exercise the loader, the tiling invariant, the resolver, the
timeline query, and the board — auctions with a mid-day break, a daylight-saving zone whose
open is a process, and an always-on venue with scheduled events. It is covered by the
repository's code license and does not create an exception to the rule that no venue data
ships here.

## Scope of this document

This is policy, not a legal opinion, and it is not a substitute for reading a
specific source's actual terms. Questions about whether a particular upstream
source may be vendored, cached, or redistributed are resolved once, at source
registration time, when that source's terms are checked and recorded — not
re-litigated later at query time. If a source's status is unclear, treat it as
non-redistributable (see above) until the registration record says otherwise.
