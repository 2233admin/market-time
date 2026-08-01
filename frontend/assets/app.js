const state = {
  mode: "live",
  inspectionAt: null,
  payload: null,
  refreshTimer: null,
  requestSequence: 0,
};

const elements = {
  clock: document.querySelector("#utc-time"),
  discipline: document.querySelector("#clock-discipline"),
  form: document.querySelector("#instant-form"),
  input: document.querySelector("#instant-input"),
  live: document.querySelector("#live-button"),
  summary: document.querySelector("#snapshot-summary"),
  meta: document.querySelector("#snapshot-meta"),
  grid: document.querySelector("#market-grid"),
};

const phaseLabels = {
  closed: "Closed",
  pre_open: "Pre-open",
  opening_auction: "Opening auction",
  continuous_trading: "Continuous trading",
  mid_day_break: "Mid-day break",
  closing_auction: "Closing auction",
  post_close: "Post-close",
  non_trading_interruption: "Non-trading interruption",
};

const calendarKinds = {
  weekly_pattern: "常规日历",
  holiday: "节假日例外",
  shortened_session: "缩短交易日",
  announced_change: "公告变更",
};

function currentInstant() {
  return state.mode === "live" ? new Date() : state.inspectionAt;
}

function isoWithoutMilliseconds(value) {
  return value.toISOString().replace(".000Z", "Z");
}

function inputValue(value) {
  return value.toISOString().slice(0, 19);
}

function readableTime(value, zone) {
  return new Intl.DateTimeFormat("en-GB", {
    timeZone: zone,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  }).format(value);
}

function readableDate(value, zone) {
  return new Intl.DateTimeFormat("en-GB", {
    timeZone: zone,
    weekday: "short",
    day: "2-digit",
    month: "short",
    year: "numeric",
  }).format(value);
}

function readableInstant(value, zone = "UTC") {
  return new Intl.DateTimeFormat("en-GB", {
    timeZone: zone,
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
    timeZoneName: "short",
  }).format(new Date(value));
}

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function appendLabeledText(parent, label, value, className) {
  const line = element("p", className);
  const strong = element("strong", null, `${label}: `);
  line.append(strong, document.createTextNode(value));
  parent.append(line);
}

function phaseLabel(phase) {
  return phaseLabels[phase] ?? phase.replaceAll("_", " ");
}

function updateClockOnly() {
  const instant = currentInstant();
  elements.clock.dateTime = instant.toISOString();
  elements.clock.textContent = isoWithoutMilliseconds(instant);
  document.querySelectorAll("[data-zone]").forEach((clock) => {
    const zone = clock.dataset.zone;
    clock.textContent = readableTime(instant, zone);
    clock.nextElementSibling.textContent = readableDate(instant, zone);
  });
}

function safeEvidenceLink(source) {
  try {
    const url = new URL(source.source_url);
    if (!["https:", "http:"].includes(url.protocol)) return document.createTextNode(source.source_url);
    const link = element("a", null, source.source_url);
    link.href = url.toString();
    link.target = "_blank";
    link.rel = "noreferrer";
    return link;
  } catch {
    return document.createTextNode(source.source_url);
  }
}

function evidenceDetails(venue) {
  const details = element("details");
  details.append(element("summary", null, "证据、修订与不确定性"));
  const content = element("div");
  appendLabeledText(content, "Uncertainty", venue.uncertainty ?? "not supplied", "uncertainty");
  if (venue.derived_reasoning) appendLabeledText(content, "Derived", venue.derived_reasoning, "uncertainty");

  const sources = element("ul", "source-list");
  for (const source of venue.evidence ?? []) {
    const item = element("li");
    item.append(safeEvidenceLink(source));
    item.append(document.createTextNode(` · fetched ${source.fetched_at} · effective ${source.effective_from}`));
    sources.append(item);
  }
  content.append(sources);
  content.append(element("p", "revisions", `Revision: ${(venue.dataset_revisions ?? []).join(", ") || "not supplied"}`));
  details.append(content);
  return details;
}

function knownCard(venue, reference) {
  const card = element("article", "market-card");
  card.dataset.state = "known";
  const head = element("header", "card-head");
  const title = element("div");
  title.append(
    element("p", "venue-code", venue.id),
    element("h3", "venue-name", venue.display_name),
    element("p", "city", venue.location ? `${venue.location} · ${venue.home_zone}` : venue.home_zone),
  );
  head.append(title, element("p", "state-chip", "known"));

  const body = element("div", "card-body");
  const time = element("time", "local-time");
  time.dataset.zone = venue.home_zone;
  time.dateTime = reference.toISOString();
  const date = element("p", "local-date", readableDate(reference, venue.home_zone));
  body.append(time, date, element("p", "field-label", "Server phase"), element("p", "phase", phaseLabel(venue.phase)));
  const calendarText = `${calendarKinds[venue.calendar?.kind] ?? venue.calendar?.kind ?? "日历规则"} · ${venue.calendar?.label ?? "未提供标签"}`;
  body.append(element("p", "calendar-note", calendarText));
  appendLabeledText(body, "当前服务端窗口结束", readableInstant(venue.boundary_end.instant, venue.home_zone), "boundary");
  appendLabeledText(body, "边界不确定性", venue.boundary_end.uncertainty, "uncertainty");

  if ((venue.events ?? []).length > 0) {
    const events = element("div", "events");
    for (const event of venue.events) events.append(element("span", "event", `${event.kind} · ${readableInstant(event.instant, venue.home_zone)}`));
    body.append(events);
  }
  const foot = element("footer", "card-foot");
  foot.append(evidenceDetails(venue));
  card.append(head, body, foot);
  return card;
}

function unknownCard(venue, reference) {
  const card = element("article", "market-card");
  card.dataset.state = "unknown";
  const head = element("header", "card-head");
  const title = element("div");
  title.append(element("p", "venue-code", venue.id), element("h3", "venue-name", venue.display_name));
  head.append(title, element("p", "state-chip", "unknown"));
  const body = element("div", "card-body");
  const time = element("time", "local-time", readableTime(reference, venue.home_zone));
  time.dataset.zone = venue.home_zone;
  const date = element("p", "local-date", readableDate(reference, venue.home_zone));
  body.append(time, date, element("p", "field-label", "Rule answer"), element("p", "phase", "Unknown"));
  body.append(element("p", "unknown-reason", venue.reason));
  if (venue.coverage) appendLabeledText(body, "声明覆盖", `${venue.coverage.start} — ${venue.coverage.end}`, "boundary");
  const foot = element("footer", "card-foot");
  foot.append(element("p", "revisions", `Revision: ${(venue.dataset_revisions ?? []).join(", ") || "not supplied"}`));
  card.append(head, body, foot);
  return card;
}

function renderPayload(payload) {
  const reference = currentInstant();
  elements.grid.replaceChildren(...payload.venues.map((venue) => (venue.status === "known" ? knownCard(venue, reference) : unknownCard(venue, reference))));
  elements.grid.setAttribute("aria-busy", "false");
  const known = payload.venues.filter((venue) => venue.status === "known").length;
  const unknown = payload.venues.length - known;
  elements.summary.textContent = `${known} known · ${unknown} unknown · 服务端规则快照 ${payload.at}`;
  elements.meta.textContent = `tzdb ${payload.tzdb_version ?? "not supplied"} · revisions ${(payload.dataset_revisions ?? []).join(", ")}`;
  elements.discipline.textContent = payload.clock.discipline === "unmeasured"
    ? `服务端时钟：未测量（${payload.clock.source}）`
    : "服务端按所给 UTC 时刻解析；浏览器不计算交易时段。";
  updateClockOnly();
}

function scheduleRefresh(payload) {
  window.clearTimeout(state.refreshTimer);
  if (state.mode !== "live") return;
  const now = Date.now();
  const boundaries = payload.venues
    .filter((venue) => venue.status === "known")
    .map((venue) => Date.parse(venue.boundary_end.instant))
    .filter(Number.isFinite)
    .filter((value) => value > now);
  const nextBoundary = boundaries.length ? Math.min(...boundaries) : now + 30_000;
  const wait = Math.max(1_000, Math.min(30_000, nextBoundary - now + 200));
  state.refreshTimer = window.setTimeout(() => void refresh(), wait);
}

async function refresh() {
  const sequence = ++state.requestSequence;
  const instant = currentInstant();
  const response = await fetch(`/v1/status?at=${encodeURIComponent(instant.toISOString())}`, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const payload = await response.json();
  if (sequence !== state.requestSequence) return;
  state.payload = payload;
  renderPayload(payload);
  scheduleRefresh(payload);
}

async function load() {
  elements.grid.setAttribute("aria-busy", "true");
  try {
    await refresh();
  } catch (error) {
    elements.grid.setAttribute("aria-busy", "false");
    elements.summary.textContent = `无法读取服务端状态：${error.message}`;
    elements.meta.textContent = "确认 market-time-server 正在运行，然后重试。";
  }
}

elements.form.addEventListener("submit", (event) => {
  event.preventDefault();
  const parsed = new Date(`${elements.input.value}Z`);
  if (Number.isNaN(parsed.valueOf())) return;
  state.mode = "inspection";
  state.inspectionAt = parsed;
  void load();
});

elements.live.addEventListener("click", () => {
  state.mode = "live";
  state.inspectionAt = null;
  elements.input.value = inputValue(new Date());
  void load();
});

elements.input.value = inputValue(new Date());
window.setInterval(updateClockOnly, 1_000);
void load();
