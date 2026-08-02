export type TimelineStatus = "known" | "unknown";

export interface TimelineInterval {
	axis_zone: string;
	start: string;
	end: string;
}

export interface TimelineSegment {
	status: TimelineStatus;
	phase: string | null;
	trading: boolean | null;
	current: boolean;
	start: string;
	end: string;
	reason?: string;
	position: {
		start_millionths: number;
		end_millionths: number;
	};
	calendar?: {
		kind: string;
		label: string;
	};
	boundary_uncertainty?: {
		start: string;
		end: string;
	};
}

export interface VenueTimeline {
	id: string;
	display_name: string;
	family: string | null;
	home_zone: string | null;
	location: string | null;
	segments: TimelineSegment[];
	trading_windows: Array<{ start: string; end: string }>;
	next_trading_transition: {
		at: string;
		kind: "opens" | "closes";
		phase: string;
	} | null;
	next_trading_window: {
		start: string;
		end: string;
	} | null;
}

export type VenueAttentionKind =
	| "closing"
	| "trading"
	| "opening"
	| "break"
	| "later"
	| "closed"
	| "unknown";

export interface VenueAttention {
	venue: VenueTimeline;
	kind: VenueAttentionKind;
	transitionAt: string | null;
	minutesToTransition: number | null;
}

export interface TransitionReminder {
	key: string;
	venueId: string;
	venueName: string;
	kind: "opens" | "closes";
	transitionAt: string;
	fireAt: string;
}

export interface TimelinePayload {
	at: string;
	clock: {
		discipline: string;
		source?: string;
	};
	dataset_revisions: string[];
	interval: TimelineInterval;
	tzdb_version: string;
	venues: VenueTimeline[];
}

export interface StatusEvidence {
	source_url: string;
	fetched_at: string;
	effective_from: string;
	publisher_last_changed: string | null;
}

export interface StatusVenue {
	id: string;
	display_name: string;
	status: TimelineStatus;
	phase?: string | null;
	derived_reasoning?: string | null;
	evidence: StatusEvidence[];
}

export interface StatusPayload {
	at: string;
	clock: {
		discipline: string;
		source?: string;
	};
	dataset_revisions: string[];
	tzdb_version: string;
	venues: StatusVenue[];
}

export interface SourceSummary {
	url: string;
	host: string;
	venues: string[];
	fetchedAt: string;
	effectiveFrom: string;
	publisherLastChanged: string | null;
}

export interface SegmentLike {
	current?: boolean;
	status: string;
	trading?: boolean | null;
}

export interface VenueLike {
	segments?: SegmentLike[];
}

export type MarketHubState = "trading" | "idle" | "unknown" | "pending";

export type SnapshotMode = "live" | "inspection";
export type ViewMode =
	| "LOADING"
	| "REFRESHING"
	| "LIVE"
	| "INSPECTION"
	| "STALE"
	| "ERROR";

export function currentSegment<T extends SegmentLike>(venue: {
	segments?: T[];
}): T | null {
	return venue.segments?.find((segment) => segment.current) ?? null;
}

export function countSnapshot(venues: VenueLike[]) {
	const current = venues.map(currentSegment);
	const trading = current.filter(
		(segment) => segment?.status === "known" && segment.trading,
	).length;
	const unknown = current.filter(
		(segment) => !segment || segment.status === "unknown",
	).length;
	return { trading, notTrading: venues.length - trading - unknown, unknown };
}

export function marketHubState(
	venues: Array<VenueLike | undefined>,
): MarketHubState {
	const connected = venues.filter(
		(venue): venue is VenueLike => venue !== undefined,
	);
	if (!connected.length) return "pending";

	const current = connected.map(currentSegment);
	if (
		current.some(
			(segment) => segment?.status === "known" && segment.trading === true,
		)
	)
		return "trading";
	if (current.some((segment) => segment?.status === "known")) return "idle";
	return "unknown";
}

export function rankVenueAttention(
	venues: VenueTimeline[],
	at: string | Date,
): VenueAttention[] {
	const now = at instanceof Date ? at.getTime() : Date.parse(at);
	const priority: Record<VenueAttentionKind, number> = {
		closing: 0,
		trading: 1,
		opening: 2,
		break: 3,
		later: 4,
		closed: 5,
		unknown: 6,
	};

	return venues
		.map((venue): VenueAttention => {
			const segment = currentSegment(venue);
			const transition = venue.next_trading_transition;
			const transitionTime = transition
				? Date.parse(transition.at)
				: Number.NaN;
			const minutes =
				Number.isFinite(now) && Number.isFinite(transitionTime)
					? Math.max(0, Math.ceil((transitionTime - now) / 60_000))
					: null;
			let kind: VenueAttentionKind = "closed";

			if (!segment || segment.status === "unknown") kind = "unknown";
			else if (
				segment.phase === "pre_open" ||
				segment.phase === "opening_auction"
			)
				kind = "opening";
			else if (
				segment.trading &&
				transition?.kind === "closes" &&
				minutes !== null &&
				minutes <= 90
			)
				kind = "closing";
			else if (segment.trading) kind = "trading";
			else if (segment.phase === "mid_day_break") kind = "break";
			else if (
				transition?.kind === "opens" &&
				minutes !== null &&
				minutes <= 120
			)
				kind = "opening";
			else if (transition?.kind === "opens") kind = "later";

			return {
				venue,
				kind,
				transitionAt: transition?.at ?? null,
				minutesToTransition: minutes,
			};
		})
		.sort((left, right) => {
			const byKind = priority[left.kind] - priority[right.kind];
			if (byKind) return byKind;
			const leftTime = left.transitionAt
				? Date.parse(left.transitionAt)
				: Number.POSITIVE_INFINITY;
			const rightTime = right.transitionAt
				? Date.parse(right.transitionAt)
				: Number.POSITIVE_INFINITY;
			return (
				leftTime - rightTime ||
				left.venue.display_name.localeCompare(right.venue.display_name)
			);
		});
}

export function nextTransitionReminder(
	venues: VenueTimeline[],
	at: string | Date,
	leadMinutes: number,
): TransitionReminder | null {
	const now = at instanceof Date ? at.getTime() : Date.parse(at);
	if (!Number.isFinite(now) || !Number.isFinite(leadMinutes)) return null;
	const lead = Math.max(0, Math.floor(leadMinutes)) * 60_000;

	return (
		venues
			.flatMap((venue) => {
				const transition = venue.next_trading_transition;
				if (!transition) return [];
				const transitionAt = Date.parse(transition.at);
				if (!Number.isFinite(transitionAt) || transitionAt <= now) return [];
				return [
					{
						key: `${venue.id}:${transition.kind}:${transition.at}:${lead}`,
						venueId: venue.id,
						venueName: venue.display_name,
						kind: transition.kind,
						transitionAt: transition.at,
						fireAt: new Date(transitionAt - lead).toISOString(),
					},
				];
			})
			.sort(
				(left, right) =>
					Date.parse(left.transitionAt) - Date.parse(right.transitionAt),
			)[0] ?? null
	);
}

export function cursorPercent(
	at: string | Date,
	interval: Pick<TimelineInterval, "start" | "end">,
): number | null {
	const instant = at instanceof Date ? at.getTime() : Date.parse(at);
	const start = Date.parse(interval.start);
	const end = Date.parse(interval.end);
	if (
		![instant, start, end].every(Number.isFinite) ||
		end <= start ||
		instant < start ||
		instant >= end
	)
		return null;
	return ((instant - start) / (end - start)) * 100;
}

export function utcInstantAtPercent(
	percent: number,
	interval: Pick<TimelineInterval, "start" | "end">,
): string | null {
	const start = Date.parse(interval.start);
	const end = Date.parse(interval.end);
	if (![percent, start, end].every(Number.isFinite) || end <= start)
		return null;
	const bounded = Math.min(100, Math.max(0, percent));
	const instant = start + Math.round(((end - start - 1) * bounded) / 100);
	return new Date(instant).toISOString();
}

export function viewMode({
	hasPayload,
	loading,
	stale,
	snapshotMode,
}: {
	hasPayload: boolean;
	loading: boolean;
	stale: boolean;
	snapshotMode: SnapshotMode | null;
}): ViewMode {
	if (!hasPayload) return loading ? "LOADING" : "ERROR";
	if (loading) return "REFRESHING";
	if (stale) return "STALE";
	if (snapshotMode === "live") return "LIVE";
	if (snapshotMode === "inspection") return "INSPECTION";
	return "ERROR";
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isInstant(value: unknown): value is string {
	return (
		typeof value === "string" &&
		value.endsWith("Z") &&
		Number.isFinite(Date.parse(value))
	);
}

function isNullableString(value: unknown) {
	return value === null || typeof value === "string";
}

function isHttpUrl(value: unknown): value is string {
	if (typeof value !== "string") return false;
	try {
		const url = new URL(value);
		return url.protocol === "http:" || url.protocol === "https:";
	} catch {
		return false;
	}
}

function isEffectiveDate(value: unknown): value is string {
	return typeof value === "string" && Number.isFinite(Date.parse(value));
}

function isStatusEvidence(value: unknown): value is StatusEvidence {
	return (
		isRecord(value) &&
		isHttpUrl(value.source_url) &&
		isInstant(value.fetched_at) &&
		isEffectiveDate(value.effective_from) &&
		(value.publisher_last_changed === null ||
			isInstant(value.publisher_last_changed))
	);
}

function isStatusVenue(value: unknown): value is StatusVenue {
	return (
		isRecord(value) &&
		typeof value.id === "string" &&
		typeof value.display_name === "string" &&
		(value.status === "known" || value.status === "unknown") &&
		(value.phase === undefined || isNullableString(value.phase)) &&
		(value.derived_reasoning === undefined ||
			isNullableString(value.derived_reasoning)) &&
		Array.isArray(value.evidence) &&
		value.evidence.every(isStatusEvidence)
	);
}

export function isStatusPayload(value: unknown): value is StatusPayload {
	return (
		isRecord(value) &&
		isInstant(value.at) &&
		isRecord(value.clock) &&
		typeof value.clock.discipline === "string" &&
		(value.clock.source === undefined ||
			typeof value.clock.source === "string") &&
		Array.isArray(value.dataset_revisions) &&
		value.dataset_revisions.every((revision) => typeof revision === "string") &&
		typeof value.tzdb_version === "string" &&
		Array.isArray(value.venues) &&
		value.venues.every(isStatusVenue)
	);
}

export function summarizeSources(payload: StatusPayload): SourceSummary[] {
	const sources = new Map<
		string,
		Omit<SourceSummary, "venues"> & { venues: Set<string> }
	>();

	for (const venue of payload.venues) {
		for (const evidence of venue.evidence) {
			const current = sources.get(evidence.source_url);
			if (!current) {
				sources.set(evidence.source_url, {
					url: evidence.source_url,
					host: new URL(evidence.source_url).hostname,
					venues: new Set([venue.display_name]),
					fetchedAt: evidence.fetched_at,
					effectiveFrom: evidence.effective_from,
					publisherLastChanged: evidence.publisher_last_changed,
				});
				continue;
			}

			current.venues.add(venue.display_name);
			if (Date.parse(evidence.fetched_at) > Date.parse(current.fetchedAt))
				current.fetchedAt = evidence.fetched_at;
			if (
				Date.parse(evidence.effective_from) < Date.parse(current.effectiveFrom)
			)
				current.effectiveFrom = evidence.effective_from;
			if (
				evidence.publisher_last_changed &&
				(!current.publisherLastChanged ||
					Date.parse(evidence.publisher_last_changed) >
						Date.parse(current.publisherLastChanged))
			)
				current.publisherLastChanged = evidence.publisher_last_changed;
		}
	}

	return [...sources.values()]
		.map((source) => ({ ...source, venues: [...source.venues].sort() }))
		.sort(
			(left, right) => Date.parse(right.fetchedAt) - Date.parse(left.fetchedAt),
		);
}

function isBoundedPosition(value: unknown) {
	if (!isRecord(value)) return false;
	const start = value.start_millionths;
	const end = value.end_millionths;
	return (
		Number.isInteger(start) &&
		Number.isInteger(end) &&
		Number(start) >= 0 &&
		Number(end) <= 1_000_000 &&
		Number(start) <= Number(end)
	);
}

function isWindow(value: unknown) {
	return (
		isRecord(value) &&
		isInstant(value.start) &&
		isInstant(value.end) &&
		Date.parse(value.start) < Date.parse(value.end)
	);
}

function isCalendar(value: unknown) {
	return (
		isRecord(value) &&
		typeof value.kind === "string" &&
		typeof value.label === "string"
	);
}

function isBoundaryUncertainty(value: unknown) {
	return (
		isRecord(value) &&
		typeof value.start === "string" &&
		typeof value.end === "string"
	);
}

function isTimelineSegment(value: unknown): value is TimelineSegment {
	if (
		!isRecord(value) ||
		!isInstant(value.start) ||
		!isInstant(value.end) ||
		Date.parse(value.start) >= Date.parse(value.end) ||
		typeof value.current !== "boolean" ||
		!isBoundedPosition(value.position)
	)
		return false;

	if (value.status === "known")
		return (
			typeof value.phase === "string" &&
			typeof value.trading === "boolean" &&
			isCalendar(value.calendar) &&
			isBoundaryUncertainty(value.boundary_uncertainty)
		);

	return (
		value.status === "unknown" &&
		value.phase === null &&
		value.trading === null &&
		typeof value.reason === "string"
	);
}

function isTransition(value: unknown) {
	return (
		value === null ||
		(isRecord(value) &&
			isInstant(value.at) &&
			(value.kind === "opens" || value.kind === "closes") &&
			typeof value.phase === "string")
	);
}

function isVenueTimeline(value: unknown): value is VenueTimeline {
	if (
		!isRecord(value) ||
		typeof value.id !== "string" ||
		typeof value.display_name !== "string" ||
		!isNullableString(value.family) ||
		!isNullableString(value.home_zone) ||
		!isNullableString(value.location) ||
		!Array.isArray(value.segments) ||
		!value.segments.every(isTimelineSegment) ||
		value.segments.filter((segment) => segment.current).length !== 1 ||
		!Array.isArray(value.trading_windows) ||
		!value.trading_windows.every(isWindow) ||
		!isTransition(value.next_trading_transition) ||
		(value.next_trading_window !== null && !isWindow(value.next_trading_window))
	)
		return false;

	return true;
}

export function isTimelinePayload(value: unknown): value is TimelinePayload {
	if (!isRecord(value) || !isRecord(value.clock) || !isRecord(value.interval))
		return false;
	if (
		!isInstant(value.at) ||
		typeof value.clock.discipline !== "string" ||
		(value.clock.source !== undefined &&
			typeof value.clock.source !== "string") ||
		value.interval.axis_zone !== "UTC" ||
		!isInstant(value.interval.start) ||
		!isInstant(value.interval.end) ||
		Date.parse(value.interval.start) >= Date.parse(value.interval.end) ||
		Date.parse(value.at) < Date.parse(value.interval.start) ||
		Date.parse(value.at) >= Date.parse(value.interval.end) ||
		typeof value.tzdb_version !== "string" ||
		!Array.isArray(value.dataset_revisions) ||
		!value.dataset_revisions.every(
			(revision) => typeof revision === "string",
		) ||
		!Array.isArray(value.venues) ||
		!value.venues.every(isVenueTimeline)
	)
		return false;

	const intervalStart = Date.parse(value.interval.start);
	const intervalEnd = Date.parse(value.interval.end);
	return value.venues.every((venue) =>
		[...venue.segments, ...venue.trading_windows].every(
			(item) =>
				Date.parse(item.start) >= intervalStart &&
				Date.parse(item.end) <= intervalEnd,
		),
	);
}
