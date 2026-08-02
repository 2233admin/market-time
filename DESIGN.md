---
name: Mark Time
---

## Product Context

Mark Time is auditable time infrastructure for global financial markets. The product surface has
two distinct layers: a broad market directory for orientation and a rule-backed timeline for
claims about trading state. A directory entry may show its local civil clock without claiming its
market is open, closed, or normally open at a particular time.

## Visual Direction

The visual identity is **a live market horizon**: a midnight dealing-desk canvas by default, a
cool mineral light alternative, one cobalt UTC ruler, and a safety-orange now line. It should feel
like an international dealing desk crossed with an airport connections board—not a generic admin
dashboard, cyberpunk terminal, or copy of an aggregator.

Four signature components define the page:

- a large tabular city clock;
- a service-ranked market relay paired with world-map nodes for trading, idle, unknown, and
  unconfigured hubs;
- a compact world-clock strip connecting Sydney, Tokyo, London, and New York;
- one shared 00–24 UTC ruler;
- a searchable, family-grouped market directory with aligned local time, rule track, and state.

## Colors

| Token | Value | Use |
| --- | --- | --- |
| `--paper` | `#0b1018` / `#eef1f4` | dark-default / light page ground |
| `--paper-light` | `#111824` / `#f9fbfd` | relay and table surfaces |
| `--ink` | `#e8edf5` / `#111720` | primary text and structure |
| `--blue` | `#7897ff` / `#315dff` | navigation, UTC orientation, explicit unknown |
| `--orange` | `#ff7147` / `#f15b2a` | current UTC instant |
| `--green` | `#38c69b` / `#087a5b` | evidence-backed trading state |
| `--yellow` | `#e9b84b` / `#e5ad18` | auction and selected asset accents |

Colour never carries state alone. Every row also names `交易中`, `休市`, `UNKNOWN`, or `未配置`.
Unsupported rule tracks use neutral hatching and never resemble a trading session.

## Typography

Use `Noto Sans SC` for Chinese reading and `IBM Plex Mono` for clocks, offsets, identifiers, and
axis labels. The main clock is deliberately oversized with tabular figures; table text stays
compact but not below 12px-equivalent for essential content. System fallbacks preserve meaning if
the font network is unavailable.

## Layout and Behavior

The desktop page is an open 96rem canvas rather than one giant boxed dashboard. A restrained top
bar leads into a full-width clock command strip, then a wide market relay, the shared UTC ruler,
and the market explorer. Inside the relay, the map and service-ranked market queue receive useful
reading widths instead of nesting a narrow secondary rail. The map lets a hub click change the
primary civil clock; the queue shows only connected venues from service state. The explorer opens
on equities, uses category navigation plus search, and keeps digital assets available as a
visually secondary family; choosing a family reduces the table without changing the underlying
catalog.

Source intelligence lives at `/settings`, outside the timetable scan path. That workspace
progressively discloses two operator tasks: scan the evidence attached to the current `/v1/status`
snapshot, or inspect the controlled intake protocol. Repeated source URLs may be condensed for
scanning, but every summary retains the real publisher link, venue coverage, fetch time, and
effective date.

Each market row reads in this order: identity → local civil time → service-supplied UTC timeline →
current service state. Local civil time is formatted from the same server-anchored absolute
instant using the catalog or returned venue zone. Session segments, next boundaries, calendar
exceptions, uncertainty, and unknown status only come from `/v1/timeline`.

At 980px the category rail becomes a horizontal category bar. At 680px rows stack into identity
and local time, full-width rule track, then textual state; no page-level horizontal scrolling is
required. Historical instant inspection stays on the timetable rulers; time preferences, evidence,
revisions, and appearance live together at `/settings`. Dark is the product default; a saved light
choice is restored before hydration to avoid a theme flash, and its control appears only in settings.

## Data Semantics

- The catalog contains equities, international spot/FX, commodities/futures, and digital assets.
- Catalog presence means “the product knows this target venue,” not “rules are connected.”
- Empty hatching means the operator has not configured a compliant rule dataset.
- Digital assets are not assumed to be unconditionally 24×7; maintenance, funding, and
  instrument-specific rules still require evidence.
- The browser advances the displayed server snapshot and formats civil clocks. It does not author
  a session, holiday, opening time, closing time, or fallback schedule.
- Source intelligence is derived from `/v1/status`; a condensed source row is presentation only
  and never replaces the raw evidence or revision that produced a market claim.
- An agent may discover official sources and extract candidate rules. Terms registration, golden
  vectors, review, and approval still gate publication of an immutable dataset revision.

## Source Decisions

- Adopted: the generated editorial-finance reference as a foundation, then replaced its decorative
  orbit with a task-led relay and an original dark market-horizon map over cool mineral neutrals.
- Adopted: Next.js 16 App Router, React 19, Tailwind CSS 4, and existing Appica UI controls. No new
  package was added. Appica's existing Base UI primitives supply the accessible tabs, badges, and
  progress indicator for source intelligence.
- Retained: the user-supplied Jin10 page as information-architecture evidence for a shared UTC
  ruler only. No branding, schedule data, copy, or visual styling is copied.
- Rejected: copied dark aggregator styling, fake 24×7 bars, browser-side schedule calculations,
  generic rounded cards, glass effects, and decorative motion.
- Rejected: adding shadcn/ui beside Appica/Base UI, or presenting AI Elements/assistant-ui chat as
  the product. Those stacks are useful for a real conversational operator surface, not this
  timetable and evidence workflow.
