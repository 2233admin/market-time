# Quickstart: Venue Session State Validation

**Feature**: `specs/001-venue-session-state` | **Constitution**: v1.2.0 | **Spec**: [spec.md](./spec.md) | **Research**: [research.md](./research.md) | **Plan**: [plan.md](./plan.md)

This is a validation guide, not a tutorial and not an implementation. Every scenario below
exists to prove — or catch a failure to prove — one of SC-001 through SC-008, or to seed a
named golden vector required by Principle V. It does not restate `data-model.md` or
`contracts/`; where an exact interface isn't built yet, the command is written as the
**intended shape** and marked as such rather than invented with false precision.

Field names, exact CLI flags, and response schemas belong to `data-model.md` and `contracts/`
(sibling Phase 1 deliverables). Reconcile the sketches below against those documents once they
land — do not treat this file as their source of truth.

---

## Prerequisites

1. **Toolchain**: Rust matching the MSRV pinned in the workspace `Cargo.toml` at first commit
   (plan.md, Technical Context: "Rust (2024 edition; MSRV pinned at first commit)").
   ```
   cargo build --workspace
   ```

2. **`tzdb-bundle-always` MUST be enabled — hard requirement, not a preference (plan.md G4
   note).** `jiff`'s default Unix behaviour reads the host's unpinned `/usr/share/zoneinfo`,
   which silently breaks Principle III reproducibility (research.md D2). Every validation run
   in this guide assumes the bundled, pinned tzdata:
   ```
   cargo test --workspace --features tzdb-bundle-always
   ```
   Confirm this is actually wired into `market-time-core`'s `Cargo.toml` before trusting any
   result below — if the feature isn't present or isn't the default, G4 is not actually
   satisfied and every "reproducible" claim in this guide is void.

3. **A loaded dataset revision** for each launch venue, under `market-time-data`. As of this
   writing (research.md D6, carried into plan.md Phase 0 Outcome) this is **not yet fully
   unblocked**:
   - **SSE** — non-redistributable (Trading Rules Art. 5.1.3); per `DATA-LICENSING.md` the
     calendar is *not vendored*, only fetched at run time with its evidence record retained.
     A live or cached-with-permission fetch path must be configured before SSE scenarios can
     run for real.
   - **NYSE** — ingestion blocked until `ice.com/terms-of-use` is fetched and recorded
     verbatim (research.md carried-forward item 1).
   - **Binance** — ingestion blocked until its terms are re-fetched with a JS-capable browser
     and recorded verbatim (carried-forward item 2). The FAQ quotes used for funding rules
     (D5, D5a) are usable as rule evidence regardless; this blocker is specifically about the
     *licensing* gate for ingesting the terms page itself.
   - The NYSE early-close footnote's exact wording still needs a byte-exact pass
     (carried-forward item 3); its *substance* — 13:00 close, day after Thanksgiving and
     Christmas Eve — is established enough to write vectors against now.
   - `jiff`'s `Disambiguation::Reject` behaviour must itself be confirmed by a golden vector
     rather than trusted from documentation (carried-forward item 4). NYSE-6 and NYSE-7 below
     **are** that confirmation — until they exist and pass, that carried-forward item is open.

   Until all four are resolved, treat any vector below marked "illustrative" or "placeholder"
   as a specification of what the vector must check, not evidence that it currently passes.

4. **Reproducibility bookkeeping check** (Principle III: "every build MUST report the
   revisions it is running against"). Before running anything else, confirm the build actually
   reports its revision set:
   ```
   # intended shape — exact subcommand per contracts/
   cargo run -p market-time-cli -- revisions
   ```
   Expected outcome: a list including at minimum the IANA tzdata release identifier (recorded
   as our own fact per D2, since `jiff` exposes no version string) and one entry per venue
   calendar/schedule dataset, each with a stable identifier. If this list is empty, incomplete,
   or absent, stop — nothing downstream can be called reproducible yet.

---

## The golden vector set

Each row names a venue, the instant queried (with its declared zone or scale — Principle II
forbids an implicit mixture), the expected phase, and what it specifically proves. IDs are
stable names for Principle V's "every defect becomes a permanent vector" requirement — when one
of these is promoted to an actual test, keep its ID as the test name.

Where research.md and the spec together do not fully pin a value (an exact calendar date, an
exact published boundary time), the row says so explicitly rather than inventing one. That is
a gap in *this guide's illustrative literals*, not permission to guess in the *product's*
answers — FR-002 and SC-003 still apply to real queries.

### SSE (Asia/Shanghai, UTC+8, no DST — research.md D4)

| ID | Instant queried | Expected phase | What it proves | Source |
|---|---|---|---|---|
| SSE-1 | 10:00 Asia/Shanghai on a non-holiday weekday (illustrative date; confirm against the loaded SSE calendar revision before treating as permanent) | `continuous_trading` | Baseline normal-day answer (spec US1 AS1; SC-001) | D4 |
| SSE-2 | 12:00 Asia/Shanghai, same day | `mid_day_break` | Two-state open/closed cannot express this; break is its own phase (spec US1 AS2) | D4 |
| SSE-3 | 09:20 Asia/Shanghai, same day | `opening_call_auction` | Pre-open auction is a distinct phase per FR-006, not folded into continuous trading | D4 |
| SSE-4 | 14:58 Asia/Shanghai, same day | `closing_call_auction` | Symmetric close-side auction phase | D4 |
| SSE-5 | 15:10 Asia/Shanghai, same day | `post_close` | SSE's after-hours fixed-price window (Trading Rules Art. 3.7.2) maps onto the *shared* vocabulary rather than an SSE-specific name (FR-005; SC-006) | D4 |
| SSE-6 | Any date on the loaded SSE public-holiday calendar (exact date supplied by the ingested revision — not fabricated here) | `closed`, holiday identified | Holiday rule overrides the normal weekly schedule (FR-015; spec US1 AS3 pattern) | D4 + loaded calendar revision |
| SSE-7 (boundary) | Exactly `11:30:00.000000000` Asia/Shanghai | Exactly one of `{continuous_trading, mid_day_break}` — **which one is not yet determined here**; it depends on the half-open interval convention `data-model.md` defines. Not both, not neither. | FR-004: boundary instants resolve deterministically to exactly one phase | D4 + `data-model.md` (convention **needs verification**) |
| SSE-8 (out-of-coverage) | An instant before the loaded SSE revision's declared coverage start, or after its declared end | Explicit unknown naming the coverage boundary | FR-002/FR-018, SC-003 | `coverage.rs` contract (Phase 1) |

**Open discrepancy — do not paper over this one.** Research.md D4's published SSE schedule
table has an unassigned interval: the closing call auction ends at 15:00 and the after-hours
fixed-price window starts at 15:05. **Nothing in D4 covers 15:00–15:05.** FR-008 requires
phases to tile all covered time with zero gaps and zero overlaps, and SSE's own trading rules
state the market is either fully open or fully closed for the day (D4: "the trading hours shall
not be extended") — which does not by itself tell us what phase name applies to this five-minute
stretch. This is a genuine open discrepancy between two documents this guide was told to treat
as authoritative, not a modelling choice for this guide to make. **Before SSE-4/SSE-5 (and any
vector spanning the close) can be called complete**, someone must either (a) find the SSE
Trading Rules provision that names this window (candidates: a closing-auction settlement step,
or simply `closed` for those five minutes) and add it to research.md/data-model.md with
evidence, or (b) confirm no such provision exists and the gap is itself the modelled fact — but
that still requires an explicit phase assignment, because FR-008 does not have an exception for
"the source didn't say." Track this as a blocking item, not a footnote.

### NYSE (America/New_York — DST-observing)

Research.md documents NYSE's *opening process* (D3) and the *substance* of its early-close
schedule (carried-forward item 3: 13:00 close, day after Thanksgiving and Christmas Eve) but
does **not** enumerate NYSE's full published session-boundary table — pre-market start, core
close, after-hours end. The illustrative times below are commonly known market convention, not
sourced from research.md, and **must be verified against NYSE's own published hours page**
before any of NYSE-1/2/3 is promoted to a permanent vector with a claimed source citation.

| ID | Instant queried | Expected phase | What it proves | Source |
|---|---|---|---|---|
| NYSE-1 | Illustrative pre-market instant, e.g. 08:00 America/New_York on a normal weekday (**exact published pre-market start not in research.md — verify**) | `pre_open` | Exercises `pre_open` as distinct from `continuous_trading` (FR-006) | **needs verification** |
| NYSE-2 | Illustrative core-session instant, e.g. 11:00 America/New_York, same day | `continuous_trading` | Baseline core-session answer. This instant is deep inside the session, so D3's process-uncertainty applies to the *09:30 boundary*, not to this query — the two must not be conflated (FR-012) | D3 (boundary caveat only) |
| NYSE-3 | Illustrative after-hours instant, e.g. 17:00 America/New_York, same day (**exact published after-hours end not in research.md — verify**) | `post_close` | After-hours maps onto the same shared vocabulary as SSE's post-close window (SC-006) | **needs verification** |
| NYSE-4 | Any date on the loaded NYSE public-holiday calendar (e.g. Independence Day — exact date, and any weekend-observance shift, supplied by the loaded revision, not fabricated here) | `closed`, holiday identified | FR-015; spec US1 AS3 | loaded calendar revision |
| NYSE-5 (early close) | The day after Thanksgiving, or Christmas Eve, at 13:30 America/New_York — **and**, same day, at 12:30 America/New_York | 13:30 → `closed`; 12:30 → `continuous_trading` under the shortened 13:00-close schedule, not the normal 16:00 schedule | Shortened schedule overrides the normal one (FR-015; spec US1 AS4); early-close *substance* is sourced, exact footnote wording is not yet (carried-forward item 3) | carried-forward item 3 |
| NYSE-6 (spring-forward) | A local wall-clock time in the `02:00`–`02:59` America/New_York hour on the spring-forward Sunday, as determined by the **pinned** `tzdb-bundle-always` release (this hour does not exist that day) | An explicit rejection/typed error — never a silently invented instant | FR-014; spec Edge Case "Spring-forward"; this is the concrete confirmation of `jiff`'s `Disambiguation::Reject` behaviour that research.md carried-forward item 4 requires before it can be trusted | D1, carried-forward item 4 |
| NYSE-7 (fall-back) | A local wall-clock time in the `01:00`–`01:59` America/New_York hour on the fall-back Sunday, same pinned release (this hour occurs twice) | The answer resolves which occurrence is meant, or states plainly that it cannot — never silently picks one | FR-014; spec Edge Case "Fall-back" | D1, carried-forward item 4 |
| NYSE-8 (boundary) | Exactly `16:00:00.000000000` America/New_York on a normal day (continuous → post-close edge; **exact published core-close time not confirmed in research.md — same gap as NYSE-1/NYSE-3, verify against NYSE's published hours before finalizing**); separately, exactly `09:30:00.000000000` | 16:00 case: exactly one phase owns the instant (FR-004). 09:30 case: the phase answer is `continuous_trading`, but its start-boundary uncertainty MUST reflect D3's process spread — it MUST NOT be presented as a market-wide instantaneous transition (FR-011b) | FR-004 + FR-011b | D3 (09:30 only); core-close time **needs verification** |
| NYSE-9 (out-of-coverage) | Before/after the loaded NYSE revision's declared coverage | Explicit unknown naming the coverage boundary | FR-002/FR-018, SC-003 | `coverage.rs` contract |

### Binance USD-M perpetual futures (UTC, always-on — research.md D5, D5a)

| ID | Instant queried | Expected phase / event | What it proves | Source |
|---|---|---|---|---|
| Binance-1 | Any instant on a normal day, declared scale UTC | `continuous_trading` — never `closed` | Confirms the "always-on venue" edge case: the phase model still applies cleanly to a venue that never closes (spec Edge Cases) | D5 intro |
| Binance-2 (funding boundary) | `00:00:00` UTC (or `08:00`/`16:00`), for a contract confirmed by the loaded revision to still be on the 8-hour default | Phase stays `continuous_trading`; a funding-settlement **event** is overlaid at this instant — the phase does not change (FR-007) | Validates the phase-vs-event split against Binance's real recurrence rule, not a hypothetical (checklist Iteration 1 finding #2) | D5 |
| Binance-3 (funding uncertainty) | Same instant as Binance-2 | The funding event's stated uncertainty is **no narrower than ±15 s** | FR-011a; SC-007 — the sharpest available test, since Binance publishes this bound verbatim rather than leaving it to us to estimate | D5a: *"15-second deviation in the actual funding fee transaction time"* |
| Binance-4 (future-dated schedule change) | For a contract in the named 4-hour-funding subset: an instant just before `2023-10-12`, and an instant just after, from the **same** dataset revision | Before: the pre-existing 8-hour recurrence rule is in force. After: the 4-hour rule is in force. Both from one unedited dataset, distinguished only by `effective_from` | Spec Edge Case "Future-dated schedule change"; Principle III (`effective_from`-governed rule change, not two different builds) | D5 |
| Binance-5 (maintenance) | The start instant of one specific, captured "Notice of … Maintenance" announcement (**placeholder** — a real ingested incident record, not a fabricated date) | The announced, scoped product line reports `non_trading_interruption`; an unaffected product line (e.g. USD-M when only COIN-M is named) continues reporting `continuous_trading` | FR-006; validates the unstructured-announcement ingestion path (D7) and that scoping doesn't blanket the whole venue | D5a + D7 |
| Binance-6 (boundary) | Exactly the announced maintenance start/end instant from Binance-5 | Exactly one phase owns the instant on each side | FR-004 | same placeholder caveat as Binance-5 |
| Binance-7 (out-of-coverage) | Before Binance USD-M perpetual futures' own listing date, or after the loaded revision's declared coverage end | Explicit unknown | FR-002/FR-018, SC-003 | `coverage.rs` contract |

**Vector count**: 24 across the three venues (SSE 8, NYSE 9, Binance 7), plus one flagged
open discrepancy (the SSE 15:00–15:05 gap) that must resolve into an assigned phase — and
likely its own vector — before it can be counted as covered.

---

## Validation scenarios

Each scenario names the Success Criteria it demonstrates, an intended-shape command, and a
concrete expected outcome. CLI invocation syntax is a sketch pending `contracts/` — the crate
names (`market-time-cli`, `market-time-core`) come from `plan.md`'s Project Structure; the
flags and response shape do not exist yet and are marked as such.

### 1. Single-venue query with phase boundaries — SC-001

```
# intended shape — reconcile flags against contracts/
cargo run -p market-time-cli -- phase --venue SSE \
  --instant <SSE-1's illustrative instant, UTC or Asia/Shanghai, explicitly tagged> \
  --dataset-revision <as reported by the "revisions" check above>
```

Expected outcome: the response names the phase (`continuous_trading`), and reports the start
and end of that phase — not just the phase name alone (FR-003). Re-run against SSE-2 through
SSE-6 and confirm each expected phase from the table above.

### 2. Every answer carries an openable source reference — SC-002, SC-008

Take the response from Scenario 1. Expected outcome: it contains at least one evidence entry
with a `source_url` a person can open directly, plus `fetched_at` and `effective_from`
(FR-009). Concretely for SSE-1: the evidence chain should resolve to the SSE Trading Rules
(2026 Revision) — the same document research.md D4 cites, not an aggregator repeating it (spec
Assumptions: "aggregators are design references, never sources"). A person unfamiliar with Mark
Time must be able to follow that URL and read the same clause D4 quotes, unassisted (SC-008)
— if they cannot, the scenario fails regardless of whether the phase answer itself was correct.

If a rule was derived rather than published verbatim (for example, if the SSE gap above is ever
resolved by inference rather than an explicit rule text), the evidence MUST be marked derived
and carry the reasoning (FR-010) — never presented as if it were observed fact.

### 3. Out-of-coverage returns explicit unknown, never a guess — SC-003

```
# intended shape
cargo run -p market-time-cli -- phase --venue NYSE --instant <NYSE-9's out-of-coverage instant>
cargo run -p market-time-cli -- phase --venue SSE  --instant <SSE-8's out-of-coverage instant>
cargo run -p market-time-cli -- phase --venue Binance --instant <Binance-7's out-of-coverage instant>
```

Expected outcome for all three: an explicit unknown that names the coverage boundary it fell
outside of — never a phase name, never a "probably closed" fallback (FR-002). Run this as a
deliberate sweep, not a single spot check: SC-003 is explicitly about "across a deliberate sweep
of out-of-range instants, no guessed phase is ever returned" — one passing query does not
establish this.

### 4. Multi-venue board, one venue unknown — SC-004

Pick (or construct, once dataset revisions are loaded) an instant where Shanghai is on its
mid-day break, New York is closed, and Binance is trading (spec US3 AS1) — and separately, an
instant where one of the three is outside its declared coverage while the other two are not
(spec US3 AS2).

```
# intended shape
cargo run -p market-time-cli -- board --instant <the chosen instant>
```

Expected outcome (first instant): all three venues appear in one response, each with its
correct phase. Expected outcome (second instant): the out-of-coverage venue reports unknown;
the other two still report their normal phases in the *same* response — one gap must not
suppress or void the whole view (FR-020). In both cases, each venue's entry in the board
response carries the same evidence and uncertainty fields a single-venue query would return
(spec US3 AS3) — the board must not be a stripped-down summary that drops those fields.

### 5. Reproducibility: same revisions, same query, same answer — SC-005

```
cargo run -p market-time-cli -- phase --venue SSE --instant <SSE-1's instant> --dataset-revision <pinned-id>
# repeat, unchanged environment, unchanged dataset revision:
cargo run -p market-time-cli -- phase --venue SSE --instant <SSE-1's instant> --dataset-revision <pinned-id>
```

Expected outcome: byte-identical output both times (FR-017; SC-005: "byte-identical answers").
Run this across a sample spanning all three venues, not just SSE — SC-005 does not name a
single venue as sufficient. If dataset revisions are genuinely immutable (Principle III), this
should hold trivially; a mismatch here is evidence the revision was mutated in place rather than
superseded by a new revision, which is itself a constitution violation worth its own vector.

### 6. Shared vocabulary, zero venue-specific phase names — SC-006

This is a structural check as much as a query check. Expected outcome: every phase name that
appears across SSE-1..8, NYSE-1..9, and Binance-1..7 above is drawn from the eight names FR-006
enumerates (`closed`, `pre_open`, `opening_call_auction`, `continuous_trading`,
`mid_day_break`, `closing_call_auction`, `post_close`, `non_trading_interruption`) — note in
particular that SSE's after-hours fixed-price window (SSE-5) and NYSE's after-hours window
(NYSE-3) both resolve to `post_close`, not two different names for a similar-but-not-identical
concept.

```
# intended shape — the real check belongs in contracts/ as a closed-enum assertion,
# not a text grep; sketch only:
cargo test -p market-time-core --features tzdb-bundle-always phase_vocabulary_is_closed
```

If any venue-specific rule module needs to introduce a phase name not in that list, FR-005 says
that is a signal to amend the shared model deliberately — not license to special-case it in a
venue module or a shell.

### 7. Stated uncertainty never narrower than the source's precision — SC-007

Binance-3 is the sharpest available test of this criterion, precisely because Binance is the
one launch venue that hands us a numeric bound rather than leaving us to infer one (D5a).

```
# intended shape
cargo run -p market-time-cli -- phase --venue Binance --instant <Binance-2/3's funding instant>
```

Expected outcome: the response's uncertainty for the funding event is a bound of **at least**
±15 seconds — a narrower stated bound (implying more confidence than Binance itself claims) is
a defect, even if the phase and event identity are otherwise correct (FR-011a). Cross-check
against NYSE-8's 09:30 case in the same pass: that boundary's uncertainty must reflect a
process spread per FR-011b, and against NYSE-1/NYSE-3's illustrative pre-market/after-hours
boundaries once their exact published times are verified — none of the three uncertainty
statements should be narrower than what its own source actually published.

---

## Adding a golden vector when a defect is found

This is Principle V's loop, stated as procedure — not a suggestion, the constitution's
non-negotiable requirement that "every defect found in a time or session answer MUST become a
golden vector before it is fixed," and that vectors are permanent.

1. **Reproduce the defect as a minimal query**: venue, the exact instant (with its declared
   scale/zone), the answer the system gave, and the answer it should have given.
2. **Author a new named vector** under `crates/market-time-core/tests/vectors/` (per plan.md's
   Project Structure) capturing: venue, instant, expected phase (and boundaries), expected
   evidence, and expected uncertainty — the same shape as the rows in this document's golden
   vector set, not a bare assertion of the phase name alone.
3. **Confirm it fails** against the current build (red) before touching anything else. A vector
   that passes on arrival proves nothing about the defect it was written for.
4. **Fix the underlying cause, never the vector.** If the defect is in resolution logic, fix
   `resolve.rs` or the relevant core module. If the defect is in rule data (a wrong boundary
   time, a missing holiday, a stale funding-interval rule), correct it by publishing a **new**
   dataset revision — per Principle III a revision is immutable, so "fixing" one in place is
   itself a second defect, not a fix.
5. **Confirm the vector now passes** (green) against the fix.
6. **The vector stays, permanently.** It is never deleted or weakened to make a later build
   pass, even after the defect it caught is long fixed (Principle V; Development Workflow and
   Quality Gates: "golden vectors are added or extended," never subtracted).
7. **Commit the vector and the fix together** (Development Workflow: rule-data changes are
   reviewed as data — dataset revision, evidence fields, and coverage declaration are all
   inspected), and note the change per the Governance section's versioning policy if it touches
   the public API or rule-data schema.

---

## Open items — roll-up

Carried here so they are not lost between documents. None of these are resolved by this guide;
each blocks at least one vector or scenario above from being more than an intended shape.

1. **Blocking, spec-level**: research.md D4's SSE schedule leaves `15:00`–`15:05` Asia/Shanghai
   unassigned to any phase. FR-008 requires full tiling. Resolve before SSE-4/SSE-5 (and any
   close-spanning query) are considered complete.
2. **Needs verification**: the boundary-ownership convention (which side of a phase edge is
   inclusive) referenced in SSE-7 and NYSE-8 — defer to `data-model.md`, do not assume
   half-open-start without confirming it there.
3. **Needs verification**: NYSE's exact published pre-market start, after-hours end, and core
   close times (NYSE-1, NYSE-3, and the 16:00 instant in NYSE-8) — research.md documents the
   opening *process* (D3) and the early-close *substance* (carried-forward item 3), not the
   full published session-boundary table.
4. **Needs verification / illustrative only**: specific calendar dates used for SSE-1, SSE-6,
   NYSE-1..5 are placeholders pending the actual loaded dataset revisions; the Binance dates in
   Binance-4 and the NYSE early-close substance in NYSE-5 are the exceptions — those come
   directly from research.md D5 and carried-forward item 3 and should be used verbatim, not
   treated as illustrative.
5. **Blocking, ingestion-level** (research.md D6, carried forward): `ice.com/terms-of-use`
   (NYSE) and Binance's terms (JS-rendered, re-fetch required) must be fetched and recorded
   verbatim before either venue's data is ingested for real.
6. **Outstanding confirmation**: `jiff`'s `Disambiguation::Reject` behaviour (carried-forward
   item 4) is only confirmed once NYSE-6 and NYSE-7 exist as real, passing tests — this
   document specifies them, it does not run them.
7. **Placeholder data**: Binance-5 and Binance-6 need one real captured maintenance
   announcement from the ingestion pipeline before they stop being placeholders.
