# CLAUDE.md — Mark Time

Briefing for any Claude / LLM agent entering this repository.

**Read [`AGENTS.md`](AGENTS.md) first — it is the full rule set and this file does not repeat it.**
What follows is the short orientation plus the Claude-specific operating notes.

## Orientation

Auditable time infrastructure for global financial markets. Two layers, in order:

1. **Baseline** — a global trading-hours board: what is open, closed, or in between, across
   venues, at a glance.
2. **The product** — every answer is nanosecond-represented, traceable to a dated source
   document, and honest about how precisely it is known.

Layer 1 is table stakes. Layer 2 is why this exists.

**Primary consumer is an autonomous agent that trades**, not a person. A human sees that a
rendered number looks approximate; an agent does not. So uncertainty and unknown live in the
returned value and in the type system, never only in the rendering. If a fact would only reach a
human through visual nuance, it is not yet modelled.

Open source, public repository — but **no venue data ever ships from here**. See AGENTS.md
"No venue data ships from this repository."

## Standalone

Mark Time is an independent product. It is **not** a subsystem of `k-atana` or any other
local project, and must not be shaped by a single downstream consumer. See AGENTS.md
"Standalone" for what that forbids concretely.

If you arrived here from a session rooted in another repository, note that this repository's
rules are the ones that apply. Another project's data contract, naming, or PIT conventions
are not precedent here.

## Before you write code

1. **Read the constitution**: `.specify/memory/constitution.md`. It is ratified governance,
   not a style guide. Five principles; two are non-negotiable. Version is in its footer.
2. **Read `DATA-LICENSING.md`** before touching any upstream source. Code license does not
   extend to rule data.
3. **Follow spec-kit**: `/speckit-specify` → `/speckit-plan` → `/speckit-tasks` →
   `/speckit-implement`. Skills are in `.agents/skills/`. Active feature is recorded in
   `.specify/feature.json`.

## Non-negotiables you will be tempted to violate

- **The core reads no clock.** `now` is passed in. This is what makes golden vectors replay
  deterministically.
- **Nanosecond representation is not an accuracy claim.** Venues publish to the second, in
  local wall time, and some deliberately randomise the open. Uncertainty must say so.
- **Outside coverage returns unknown.** Never extrapolate a calendar past its verified range,
  never fall back to "probably the usual schedule".
- **Dataset revisions are immutable.** Correcting a rule produces a new revision.
- **Evidence is mandatory, not decorative.** A rule without `source_url` / `fetched_at` /
  `effective_from` does not ship, however obviously correct it looks.

## Working in this repo

- Licensing: code is `MIT OR Apache-2.0`. Copyright holder is `The Mark Time Authors`.
- `Cargo.lock` is deliberately committed. Do not add it to `.gitignore`.
- Repository is **public**. Remote is `github.com/2233admin/market-time`, default branch `main`.
  Assume anything committed here is world-readable the moment it is pushed.

## Host quirks (this machine)

- **Do not use PowerShell here-string syntax (`@'...'@`) in the Bash tool.** Bash passes the
  `@` through literally and it lands in your commit message. Use `git commit -F <file>` for
  any multi-line message. This has already happened once and needed a history rewrite.
- Apostrophes inside single-quoted bash strings close the quote early. Another reason to use
  `-F`.
- Prefer the Read / Edit / Write / Glob / Grep tools over shell equivalents.

## Verification before claiming something works

- `cargo test` covers the change, and golden vectors are added or extended per Principle V.
- Rule-data changes are reviewed as data: revision, evidence fields, and coverage
  declaration are each inspected. A change without evidence is rejected.
- Constitution Check passes at both plan gates.
