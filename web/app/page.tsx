"use client";

import { Button } from "@appica/ui-react/button";
import { useLocalStorage } from "@appica/ui-react/hooks/use-local-storage";
import { Input } from "@appica/ui-react/input";
import { Slider } from "@appica/ui-react/slider";
import { Switch } from "@appica/ui-react/switch";
import { animate, createScope, stagger } from "animejs";
import type { CSSProperties } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
	catalogWithServerVenues,
	filterMarketCatalog,
	flagImageUrl,
	type MarketCatalogEntry,
} from "../lib/market-catalog";
import {
	countSnapshot,
	currentSegment,
	cursorPercent,
	isTimelinePayload,
	marketHubState,
	nextTransitionReminder,
	rankVenueAttention,
	type SnapshotMode,
	type TimelinePayload,
	type TimelineSegment,
	utcInstantAtPercent,
	type VenueAttention,
	type VenueAttentionKind,
	type VenueTimeline,
	viewMode,
} from "../lib/market-time";
import {
	defaultReminderPreferences,
	LAST_REMINDER_KEY,
	PRIMARY_ZONE_KEY,
	primaryTimeZones,
	REMINDER_PREFERENCES_KEY,
} from "../lib/preferences";
import { MarketMap } from "./market-map";

const API_ORIGIN = process.env.NEXT_PUBLIC_MARK_TIME_API ?? "";

const phaseLabels: Record<string, string> = {
	closed: "闭市",
	pre_open: "开市前",
	opening_auction: "开盘集合竞价",
	continuous_trading: "连续交易",
	mid_day_break: "午间休市",
	closing_auction: "收盘集合竞价",
	post_close: "收市后",
	non_trading_interruption: "交易中断",
};

const worldClockZones = [
	{ id: "Australia/Sydney", label: "悉尼", short: "SYD" },
	{ id: "Asia/Tokyo", label: "东京", short: "TYO" },
	{ id: "Europe/London", label: "伦敦", short: "LON" },
	{ id: "America/New_York", label: "纽约", short: "NYC" },
];

const marketHubs = [
	{
		id: "new-york",
		label: "纽约",
		code: "NYC",
		zone: "America/New_York",
		longitude: -74.006,
		latitude: 40.7128,
		marketIds: ["xnys", "xnas"],
	},
	{
		id: "london",
		label: "伦敦",
		code: "LON",
		zone: "Europe/London",
		longitude: -0.1276,
		latitude: 51.5072,
		marketIds: ["xlse", "ice-brent", "lme"],
	},
	{
		id: "dubai",
		label: "迪拜",
		code: "DXB",
		zone: "Asia/Dubai",
		longitude: 55.2708,
		latitude: 25.2048,
		marketIds: ["dfm", "dgcx"],
	},
	{
		id: "mumbai",
		label: "孟买",
		code: "BOM",
		zone: "Asia/Kolkata",
		longitude: 72.8777,
		latitude: 19.076,
		marketIds: ["xnse", "mcx"],
	},
	{
		id: "shanghai",
		label: "上海",
		code: "SHA",
		zone: "Asia/Shanghai",
		longitude: 121.4737,
		latitude: 31.2304,
		marketIds: ["xshg", "xshe", "shfe", "dce", "czce", "ine", "gfex", "cffex"],
	},
	{
		id: "tokyo",
		label: "东京",
		code: "TYO",
		zone: "Asia/Tokyo",
		longitude: 139.6917,
		latitude: 35.6895,
		marketIds: ["xtks", "tocom"],
	},
	{
		id: "singapore",
		label: "新加坡",
		code: "SIN",
		zone: "Asia/Singapore",
		longitude: 103.8198,
		latitude: 1.3521,
		marketIds: ["xses"],
	},
	{
		id: "sydney",
		label: "悉尼",
		code: "SYD",
		zone: "Australia/Sydney",
		longitude: 151.2093,
		latitude: -33.8688,
		marketIds: ["xasx", "xnze"],
	},
] as const;

const groupGlyphs: Record<string, string> = {
	equities: "↗",
	spot: "◎",
	futures: "▥",
	digital: "◇",
	connected: "◎",
};

const attentionLabels: Record<
	VenueAttentionKind,
	{ label: string; transition: string }
> = {
	closing: { label: "临近收盘", transition: "收盘" },
	trading: { label: "交易中", transition: "收盘" },
	opening: { label: "即将开盘", transition: "开盘" },
	break: { label: "午间休市", transition: "复市" },
	later: { label: "随后开盘", transition: "开盘" },
	closed: { label: "休市", transition: "下一边界" },
	unknown: { label: "数据未知", transition: "边界" },
};

function MarketSymbol({ symbol, kind }: { symbol: string; kind?: "asset" }) {
	const imageUrl = flagImageUrl(symbol);

	return (
		<span className="market-symbol" data-kind={kind} aria-hidden="true">
			<span>{symbol}</span>
			{imageUrl ? (
				<span
					className="market-flag-image"
					style={{ backgroundImage: `url("${imageUrl}")` }}
				/>
			) : null}
		</span>
	);
}

function formatTime(value: Date | string, zone: string, seconds = false) {
	const date = value instanceof Date ? value : new Date(value);
	return new Intl.DateTimeFormat("en-GB", {
		timeZone: zone,
		hour: "2-digit",
		minute: "2-digit",
		second: seconds ? "2-digit" : undefined,
		hourCycle: "h23",
	}).format(date);
}

function safeTimeZone(zone: string) {
	try {
		new Intl.DateTimeFormat("en", { timeZone: zone }).format();
		return zone;
	} catch {
		return "Asia/Shanghai";
	}
}

function formatDate(value: Date, zone: string) {
	return new Intl.DateTimeFormat("zh-CN", {
		timeZone: zone,
		year: "numeric",
		month: "long",
		day: "numeric",
		weekday: "long",
	}).format(value);
}

function formatUtcOffset(value: Date, zone: string) {
	const offset = new Intl.DateTimeFormat("en", {
		timeZone: zone,
		hour: "2-digit",
		timeZoneName: "shortOffset",
	})
		.formatToParts(value)
		.find((part) => part.type === "timeZoneName")?.value;
	return (offset ?? "GMT").replace("GMT", "UTC");
}

function formatSessionDate(value: string, zone: string) {
	return new Intl.DateTimeFormat("zh-CN", {
		timeZone: zone,
		month: "2-digit",
		day: "2-digit",
		weekday: "short",
	}).format(new Date(value));
}

function formatTradingWindow(
	window: { start: string; end: string },
	zone: string,
) {
	const startDate = formatSessionDate(window.start, zone);
	const endDate = formatSessionDate(window.end, zone);
	const start = `${startDate} ${formatTime(window.start, zone)}`;
	const end =
		startDate === endDate
			? formatTime(window.end, zone)
			: `${endDate} ${formatTime(window.end, zone)}`;
	return `${start}—${end}`;
}

function formatCountdown(minutes: number | null) {
	if (minutes === null) return null;
	if (minutes < 1) return "不足 1 分钟";
	if (minutes < 60) return `${minutes} 分钟`;
	const hours = Math.floor(minutes / 60);
	const remainder = minutes % 60;
	return remainder ? `${hours} 小时 ${remainder} 分` : `${hours} 小时`;
}

function attentionDetail(attention: VenueAttention, entry: MarketCatalogEntry) {
	if (!attention.transitionAt)
		return attention.kind === "trading" ? "覆盖内持续交易" : "覆盖内无变化";
	const zone = attention.venue.home_zone || entry.timeZone;
	const countdown = formatCountdown(attention.minutesToTransition);
	return `${formatTime(attention.transitionAt, zone)} ${attentionLabels[attention.kind].transition}${countdown ? ` · ${countdown}` : ""}`;
}

function segmentTitle(
	segment: TimelineSegment,
	venue: VenueTimeline,
	payload: TimelinePayload,
) {
	const zone = venue.home_zone || "UTC";
	const coversDay =
		segment.start === payload.interval.start &&
		segment.end === payload.interval.end;
	const range = coversDay
		? "完整 UTC 日"
		: `${formatTime(segment.start, zone)}–${formatTime(segment.end, zone)}`;
	if (segment.status === "unknown")
		return `${range} · unknown · ${segment.reason ?? "覆盖范围外"}`;
	const exception =
		segment.calendar?.kind === "weekly_pattern"
			? ""
			: ` · ${segment.calendar?.label ?? "日历例外"}`;
	return `${range} · ${phaseLabels[segment.phase ?? ""] ?? segment.phase ?? "已知状态"}${exception}`;
}

function rowState(venue?: VenueTimeline) {
	if (!venue) return "pending";
	const segment = currentSegment(venue);
	if (!segment || segment.status === "unknown") return "unknown";
	return segment.trading ? "trading" : "closed";
}

function rowStatus(venue?: VenueTimeline) {
	if (!venue) return { label: "未配置", detail: "等待规则数据源" };
	const segment = currentSegment(venue);
	if (!segment || segment.status === "unknown")
		return {
			label: "UNKNOWN",
			detail: segment?.reason ?? "服务端未覆盖",
		};
	const next = venue.next_trading_transition;
	const zone = venue.home_zone || "UTC";
	return {
		label: segment.trading ? "交易中" : "休市",
		detail: next
			? `${next.kind === "opens" ? "开" : "收"} ${formatTime(next.at, zone)}`
			: venue.next_trading_window
				? formatTradingWindow(venue.next_trading_window, zone)
				: "覆盖内无变化",
	};
}

function CatalogMarketRow({
	attention,
	entry,
	venue,
	payload,
	instant,
}: {
	attention?: VenueAttentionKind;
	entry: MarketCatalogEntry;
	venue?: VenueTimeline;
	payload: TimelinePayload | null;
	instant: Date | null;
}) {
	const state = rowState(venue);
	const status = rowStatus(venue);
	const current = venue ? currentSegment(venue) : null;
	const exception = venue?.segments.find(
		(segment) =>
			segment.calendar?.kind && segment.calendar.kind !== "weekly_pattern",
	)?.calendar;
	const zone = venue?.home_zone || entry.timeZone;
	const trackLabel =
		venue && payload
			? `${entry.label} 的服务端全天交易阶段：${venue.segments
					.map((segment) => segmentTitle(segment, venue, payload))
					.join(
						"；",
					)}；当前时刻 ${instant ? `${formatTime(instant, "UTC")} UTC` : "尚未同步"}`
			: `${entry.label} 尚无服务端交易时间规则`;

	return (
		<article
			className="catalog-row"
			data-attention={attention}
			data-state={state}
			id={`market-${entry.id}`}
			style={{ "--market-accent": entry.accent } as CSSProperties}
		>
			<div className="catalog-identity">
				<i aria-hidden="true" />
				<MarketSymbol symbol={entry.symbol} kind={entry.symbolKind} />
				<span>
					<strong>{entry.label}</strong>
					<small>
						{entry.detail} · {entry.id.toUpperCase()}
					</small>
				</span>
			</div>

			<div className="catalog-local-time">
				<time dateTime={instant?.toISOString()}>
					{instant ? formatTime(instant, zone) : "--:--"}
				</time>
				<small>{instant ? formatUtcOffset(instant, zone) : "UTC—"}</small>
			</div>

			<div className="catalog-track" role="img" aria-label={trackLabel}>
				{venue && payload
					? venue.segments.map((segment) => {
							const start = Number(segment.position?.start_millionths ?? 0);
							const end = Number(segment.position?.end_millionths ?? start);
							return (
								<span
									className="catalog-segment"
									data-phase={segment.phase}
									data-status={segment.status}
									key={`${segment.start}-${segment.end}-${segment.status}-${segment.phase ?? "none"}`}
									style={
										{
											"--segment-start": `${start / 10_000}%`,
											"--segment-width": `${Math.max(0, end - start) / 10_000}%`,
										} as CSSProperties
									}
									title={segmentTitle(segment, venue, payload)}
								/>
							);
						})
					: null}
			</div>

			<div className="catalog-status">
				<strong>{status.label}</strong>
				<small title={status.detail}>{status.detail}</small>
				{exception ? <em>{exception.label}</em> : null}
				{current?.status === "unknown" ? <em>数据边界</em> : null}
			</div>
		</article>
	);
}

export default function Page() {
	const [payload, setPayload] = useState<TimelinePayload | null>(null);
	const [clockZone, setClockZone] = useLocalStorage(
		PRIMARY_ZONE_KEY,
		"Asia/Shanghai",
	);
	const [reminderPreferences, setReminderPreferences] = useLocalStorage(
		REMINDER_PREFERENCES_KEY,
		defaultReminderPreferences,
	);
	const [activeGroup, setActiveGroup] = useState("equities");
	const [query, setQuery] = useState("");
	const [catalogSidebarCollapsed, setCatalogSidebarCollapsed] = useLocalStorage(
		"mark-time-catalog-sidebar-collapsed",
		false,
	);
	const [renderedAt, setRenderedAt] = useState<Date | null>(null);
	const [receivedAt, setReceivedAt] = useState(0);
	const [, setTick] = useState(0);
	const [loading, setLoading] = useState(true);
	const [stale, setStale] = useState(false);
	const [snapshotMode, setSnapshotMode] = useState<SnapshotMode | null>(null);
	const [inspectionAt, setInspectionAt] = useState<string | null>(null);
	const [scrubPreview, setScrubPreview] = useState<Date | null>(null);
	const [notice, setNotice] = useState<string | null>(null);
	const [preferenceNotice, setPreferenceNotice] = useState<string | null>(null);
	const [notificationPermission, setNotificationPermission] = useState<
		NotificationPermission | "unsupported"
	>("unsupported");
	const [reload, setReload] = useState(0);
	const requestSequence = useRef(0);
	const payloadRef = useRef<TimelinePayload | null>(null);
	const pageRef = useRef<HTMLElement>(null);
	const motionScope = useRef<ReturnType<typeof createScope> | null>(null);

	useEffect(() => {
		motionScope.current = createScope({
			root: pageRef,
			mediaQueries: {
				reducedMotion: "(prefers-reduced-motion: reduce)",
			},
		}).add((self) => {
			if (!self) return;
			self.add("tickClock", () => {
				if (self.matches.reducedMotion) return;
				const secondsNode = pageRef.current?.querySelector(".clock-seconds");
				const separatorNode =
					pageRef.current?.querySelector(".clock-separator");
				if (!secondsNode || !separatorNode) return;
				animate(secondsNode, {
					opacity: [0.35, 1],
					y: [8, 0],
					scale: [0.96, 1],
					duration: 420,
					ease: "out(4)",
				});
				animate(separatorNode, {
					opacity: [0.3, 1],
					scaleY: [0.72, 1],
					duration: 280,
					ease: "out(3)",
				});
			});
			self.add("revealPriorityMarkets", () => {
				if (self.matches.reducedMotion) return;
				const priorityRows = pageRef.current?.querySelectorAll(
					'.catalog-row[data-attention="closing"], .catalog-row[data-attention="trading"], .catalog-row[data-attention="opening"]',
				);
				if (!priorityRows?.length) return;
				animate(priorityRows, {
					opacity: [0.66, 1],
					x: [-10, 0],
					delay: stagger(55),
					duration: 560,
					ease: "out(4)",
				});
			});
		});

		return () => motionScope.current?.revert();
	}, []);

	useEffect(() => {
		const timer = window.setInterval(
			() => setTick((value) => value + 1),
			1_000,
		);
		return () => window.clearInterval(timer);
	}, []);

	useEffect(() => {
		setNotificationPermission(
			"Notification" in window ? Notification.permission : "unsupported",
		);
	}, []);

	useEffect(() => {
		void reload;
		const request = ++requestSequence.current;
		const controller = new AbortController();

		async function loadTimeline() {
			setLoading(true);
			try {
				const queryAt = inspectionAt
					? `?at=${encodeURIComponent(inspectionAt)}`
					: "";
				const response = await fetch(`${API_ORIGIN}/v1/timeline${queryAt}`, {
					headers: { Accept: "application/json" },
					signal: controller.signal,
				});
				const body: unknown = await response.json().catch(() => ({}));
				if (!response.ok) {
					const message =
						typeof body === "object" && body && "error" in body
							? String(body.error)
							: `HTTP ${response.status}`;
					throw new Error(message);
				}
				if (!isTimelinePayload(body))
					throw new Error("服务端 timeline 格式无效");
				if (request !== requestSequence.current) return;
				payloadRef.current = body;
				setPayload(body);
				setSnapshotMode(inspectionAt ? "inspection" : "live");
				setRenderedAt(new Date(body.at));
				setReceivedAt(inspectionAt ? 0 : performance.now());
				setScrubPreview(null);
				setStale(false);
				setNotice(null);
			} catch (error) {
				if (controller.signal.aborted || request !== requestSequence.current)
					return;
				setStale(Boolean(payloadRef.current));
				setNotice(
					`读取失败：${error instanceof Error ? error.message : "未知错误"}`,
				);
			} finally {
				if (request === requestSequence.current) setLoading(false);
			}
		}

		void loadTimeline();
		return () => controller.abort();
	}, [inspectionAt, reload]);

	useEffect(() => {
		if (snapshotMode !== "live" || !payload || stale || loading) return;
		const now = renderedAt
			? renderedAt.getTime() + performance.now() - receivedAt
			: Date.now();
		const boundaries = payload.venues
			.map(currentSegment)
			.map((segment) => (segment?.end ? Date.parse(segment.end) : Number.NaN))
			.filter((value) => Number.isFinite(value) && value > now);
		const delay = boundaries.length
			? Math.min(...boundaries) - now + 250
			: 60_000;
		const timer = window.setTimeout(
			() => setReload((value) => value + 1),
			Math.min(Math.max(delay, 1_000), 60_000),
		);
		return () => window.clearTimeout(timer);
	}, [loading, payload, receivedAt, renderedAt, snapshotMode, stale]);

	const instant = (() => {
		if (!renderedAt) return null;
		if (snapshotMode === "live" && !loading && !stale && receivedAt)
			return new Date(renderedAt.getTime() + performance.now() - receivedAt);
		return renderedAt;
	})();

	const venuesById = useMemo(
		() =>
			new Map(
				(payload?.venues ?? []).map((venue) => [venue.id.toLowerCase(), venue]),
			),
		[payload],
	);
	const catalogGroups = useMemo(
		() => catalogWithServerVenues(payload?.venues ?? []),
		[payload],
	);
	const marketCount = catalogGroups.reduce(
		(total, group) => total + group.entries.length,
		0,
	);
	const catalogEntriesById = useMemo(
		() =>
			new Map(
				catalogGroups.flatMap((group) =>
					group.entries.map((entry) => [entry.id, entry] as const),
				),
			),
		[catalogGroups],
	);
	const connectedVenues = payload?.venues ?? [];
	const filteredGroups = useMemo(
		() => filterMarketCatalog(activeGroup, query, catalogGroups),
		[activeGroup, catalogGroups, query],
	);
	const visibleCount = filteredGroups.reduce(
		(total, group) => total + group.entries.length,
		0,
	);
	const counts = countSnapshot(connectedVenues);
	const coverageByGroup = catalogGroups.map((group) => ({
		...group,
		connected: group.entries.filter((entry) => venuesById.has(entry.id)).length,
	}));
	const hubs = marketHubs.map((hub) => ({
		...hub,
		connected: hub.marketIds.filter((id) => venuesById.has(id)).length,
		state: marketHubState(hub.marketIds.map((id) => venuesById.get(id))),
	}));
	const attentionQueue = instant
		? rankVenueAttention(connectedVenues, instant)
		: [];
	const focusQueue = instant
		? attentionQueue
				.filter((attention) => attention.kind !== "unknown")
				.slice(0, 4)
				.flatMap((attention) => {
					const entry = catalogEntriesById.get(
						attention.venue.id.toLowerCase(),
					);
					return entry ? [{ attention, entry }] : [];
				})
		: [];
	const attentionByVenueId = new Map(
		attentionQueue.map((attention) => [
			attention.venue.id.toLowerCase(),
			attention,
		]),
	);
	const attentionOrder = new Map(
		attentionQueue.map((attention, index) => [
			attention.venue.id.toLowerCase(),
			index,
		]),
	);
	const prioritizedGroups = filteredGroups.map((group) => ({
		...group,
		entries: [...group.entries].sort(
			(left, right) =>
				(attentionOrder.get(left.id) ?? Number.MAX_SAFE_INTEGER) -
				(attentionOrder.get(right.id) ?? Number.MAX_SAFE_INTEGER),
		),
	}));
	const cursorInstant = scrubPreview ?? instant;
	const cursor =
		payload && cursorInstant
			? cursorPercent(cursorInstant, payload.interval)
			: null;
	const cursorStyle = {
		"--cursor-position": `${cursor ?? 0}%`,
	} as CSSProperties;
	const mode = viewMode({
		hasPayload: Boolean(payload),
		loading,
		stale,
		snapshotMode,
	});
	const displayClockZone = safeTimeZone(clockZone);
	const clockText = instant
		? formatTime(instant, displayClockZone, true)
		: "--:--:--";
	const [hours = "--", minutes = "--", seconds = "--"] = clockText.split(":");
	const cursorMinute = Math.round(((cursor ?? 0) / 100) * 1_439);
	const cursorLabel =
		scrubPreview || snapshotMode === "inspection" ? "所选" : "现在";

	function inspectAt(at: string) {
		if (inspectionAt === at) setReload((value) => value + 1);
		else setInspectionAt(at);
	}

	function scrubToMinute(value: number | readonly number[]) {
		if (!payload) return null;
		const minute = typeof value === "number" ? value : (value[0] ?? 0);
		const percent = (minute / 1_439) * 100;
		const at = utcInstantAtPercent(percent, payload.interval);
		if (at) setScrubPreview(new Date(at));
		return at;
	}

	function commitScrubber(value: number | readonly number[]) {
		const at = scrubToMinute(value);
		if (at) inspectAt(at);
	}

	useEffect(() => {
		void seconds;
		motionScope.current?.methods.tickClock?.();
	}, [seconds]);

	useEffect(() => {
		void activeGroup;
		void payload?.at;
		motionScope.current?.methods.revealPriorityMarkets?.();
	}, [activeGroup, payload?.at]);
	const activeZone = primaryTimeZones.find(
		(zone) => zone.id === displayClockZone,
	) ?? {
		id: displayClockZone,
		label: "本机时区",
		code: "LOCAL",
	};
	const modeLabel = {
		LOADING: "同步中",
		REFRESHING: "更新中",
		LIVE: "实时连接",
		INSPECTION: "历史快照",
		STALE: "连接过期",
		ERROR: "连接错误",
	}[mode];
	const selectedLabel =
		activeGroup === "all"
			? "全部市场"
			: (catalogGroups.find((group) => group.id === activeGroup)?.label ??
				"全部市场");
	const relayHeadline = focusQueue.length
		? counts.trading > 0
			? `${counts.trading} 个市场正在交易`
			: focusQueue.some(({ attention }) => attention.kind === "opening")
				? "下一场交易正在靠近"
				: "下一次开盘已排队"
		: connectedVenues.length
			? "当前状态仍待服务端确认"
			: "等待真实市场规则";
	const relayEmptyTitle = connectedVenues.length
		? "规则已经接入，但当前时刻仍是 Unknown。"
		: "目录已经就位，规则数据还没有。";
	const relayEmptyBody = connectedVenues.length
		? "当前快照超出已接入数据的覆盖范围，首屏不会把未知状态猜成休市。"
		: "接入带证据的市场数据后，交易中、盘前与临近收盘会自动进入首屏。";
	const coverageStyle = {
		"--coverage-percent": `${(connectedVenues.length / marketCount) * 100}%`,
	} as CSSProperties;
	const revisionLabel = payload?.dataset_revisions.at(-1) ?? "未连接规则集";
	const nextReminder = instant
		? nextTransitionReminder(
				connectedVenues,
				instant,
				reminderPreferences.leadMinutes,
			)
		: null;
	const nextReminderZone = nextReminder
		? (catalogEntriesById.get(nextReminder.venueId.toLowerCase())?.timeZone ??
			"UTC")
		: "UTC";
	const reminderFireAt = nextReminder?.fireAt;
	const reminderKey = nextReminder?.key;
	const reminderKind = nextReminder?.kind;
	const reminderTransitionAt = nextReminder?.transitionAt;
	const reminderVenueName = nextReminder?.venueName;

	useEffect(() => {
		if (
			!reminderPreferences.enabled ||
			notificationPermission !== "granted" ||
			!reminderFireAt ||
			!reminderKey ||
			!reminderKind ||
			!reminderTransitionAt ||
			!reminderVenueName
		)
			return;
		const serverNow = renderedAt
			? renderedAt.getTime() + performance.now() - receivedAt
			: Date.now();
		const delay = Date.parse(reminderFireAt) - serverNow;
		if (delay > 60 * 60 * 1_000) return;

		const timer = window.setTimeout(
			() => {
				try {
					if (localStorage.getItem(LAST_REMINDER_KEY) === reminderKey) return;
					new Notification(
						`${reminderVenueName}${reminderKind === "opens" ? "即将开盘" : "即将收盘"}`,
						{
							body: `${reminderPreferences.leadMinutes ? `${reminderPreferences.leadMinutes} 分钟后` : "现在"} · ${formatTime(reminderTransitionAt, nextReminderZone)}`,
							tag: reminderKey,
						},
					);
					localStorage.setItem(LAST_REMINDER_KEY, reminderKey);
				} catch {
					setPreferenceNotice("浏览器未能发送交易边界提醒。");
				}
			},
			Math.max(0, delay),
		);
		return () => window.clearTimeout(timer);
	}, [
		nextReminderZone,
		notificationPermission,
		receivedAt,
		reminderFireAt,
		reminderKey,
		reminderKind,
		reminderPreferences.enabled,
		reminderPreferences.leadMinutes,
		reminderTransitionAt,
		reminderVenueName,
		renderedAt,
	]);

	async function toggleReminders(enabled: boolean) {
		setPreferenceNotice(null);
		if (!enabled) {
			setReminderPreferences((current) => ({ ...current, enabled: false }));
			return;
		}
		if (!("Notification" in window)) {
			setPreferenceNotice("当前浏览器不支持系统通知。");
			return;
		}
		const permission = await Notification.requestPermission();
		setNotificationPermission(permission);
		if (permission !== "granted") {
			setPreferenceNotice("需要浏览器通知权限才能开启提醒。");
			return;
		}
		setReminderPreferences((current) => ({ ...current, enabled: true }));
	}

	return (
		<main className="page-shell" data-ui-framework="next-appica" ref={pageRef}>
			<h1 className="sr-only">MARK TIME 全球交易时间表</h1>

			<nav className="topbar" aria-label="主导航">
				<a className="wordmark" href="/" aria-label="MARK TIME 首页">
					MARKET <b>TIME</b>
				</a>
				<div className="topbar-links">
					<a href="/" aria-current="page">
						全球交易时间表
					</a>
					<a href="/settings">时间字典与设置</a>
				</div>
				<div className="connection-state" aria-live="polite">
					<i data-mode={mode} aria-hidden="true" />
					<span>{modeLabel}</span>
					<Button
						size="icon-sm"
						variant="ghost"
						onClick={() => setReload((value) => value + 1)}
						aria-label="刷新服务端时间表"
					>
						↻
					</Button>
				</div>
			</nav>

			{notice ? (
				<aside className="status-notice" role="status">
					<span>{payload ? `${notice}，保留上一次时间表。` : notice}</span>
					<Button
						size="sm"
						variant="outline"
						onClick={() => setReload((value) => value + 1)}
					>
						重新读取
					</Button>
				</aside>
			) : null}
			{preferenceNotice ? (
				<aside className="status-notice" role="status">
					<span>{preferenceNotice}</span>
					<a href="/settings#time-preferences">打开提醒设置</a>
				</aside>
			) : null}

			<section className="hero" aria-label="全球当前时间">
				<div className="hero-clock">
					<p className="eyebrow">
						{activeZone.label}当前时间
						<span>
							{instant ? formatUtcOffset(instant, displayClockZone) : "UTC—"}
						</span>
					</p>
					<time className="primary-clock" dateTime={instant?.toISOString()}>
						<span>{hours}</span>
						<i className="clock-separator">:</i>
						<span>{minutes}</span>
						<span className="clock-second-block">
							<small className="clock-seconds">{seconds}</small>
							<b>秒</b>
						</span>
					</time>
					<div className="clock-date">
						<p>
							<span>今天</span>
							<span>
								{instant
									? formatDate(instant, displayClockZone)
									: "等待服务端时间"}
							</span>
						</p>
						<div className="reminder-toggle">
							<span>
								<b>交易边界提醒</b>
								<small>
									{reminderPreferences.enabled && nextReminder
										? `${nextReminder.venueName} · ${formatTime(nextReminder.transitionAt, catalogEntriesById.get(nextReminder.venueId.toLowerCase())?.timeZone ?? "UTC")}`
										: "按服务端下一开收盘提醒"}
								</small>
							</span>
							<Switch
								aria-label="交易边界提醒"
								checked={reminderPreferences.enabled}
								onCheckedChange={(checked) => void toggleReminders(checked)}
								size="sm"
							/>
						</div>
					</div>
					<fieldset className="zone-switch">
						<legend className="sr-only">主时钟城市</legend>
						{primaryTimeZones.map((zone) => (
							<Button
								key={zone.id}
								size="sm"
								variant={displayClockZone === zone.id ? "primary" : "ghost"}
								onClick={() => setClockZone(zone.id)}
								aria-pressed={displayClockZone === zone.id}
							>
								{zone.label}
							</Button>
						))}
					</fieldset>
				</div>

				<section className="market-relay" aria-label="此刻优先市场">
					<header className="relay-header">
						<div>
							<p>LIVE MARKET RELAY</p>
							<h2>{relayHeadline}</h2>
						</div>
						<ul className="relay-summary" aria-label="当前服务端市场状态摘要">
							<li data-state="trading">交易中 {counts.trading}</li>
							<li data-state="closed">已知休市 {counts.notTrading}</li>
							<li data-state="unknown">Unknown {counts.unknown}</li>
							<li>
								规则 {connectedVenues.length}/{marketCount}
							</li>
						</ul>
					</header>

					<div className="relay-stage" aria-live="polite">
						<MarketMap hubs={hubs} onSelectZone={setClockZone} />

						<div className="relay-side">
							{focusQueue.length ? (
								<div className="relay-focus-list">
									{focusQueue.map(({ attention, entry }, index) => (
										<a
											className="relay-item"
											data-kind={attention.kind}
											href={`#market-${entry.id}`}
											key={entry.id}
										>
											<b>{String(index + 1).padStart(2, "0")}</b>
											<MarketSymbol
												symbol={entry.symbol}
												kind={entry.symbolKind}
											/>
											<span>
												<strong>{entry.label}</strong>
												<small>
													{attentionLabels[attention.kind].label} ·{" "}
													{attentionDetail(attention, entry)}
												</small>
											</span>
											<time dateTime={instant?.toISOString()}>
												{instant
													? formatTime(
															instant,
															attention.venue.home_zone || entry.timeZone,
														)
													: "--:--"}
											</time>
										</a>
									))}
								</div>
							) : (
								<div className="relay-empty">
									<div>
										<strong>{relayEmptyTitle}</strong>
										<p>{relayEmptyBody}</p>
										<a href="/settings#source-intelligence">查看数据缺口</a>
									</div>
									<ul className="relay-gap-list" aria-label="各类市场规则覆盖">
										{coverageByGroup.map((group) => (
											<li
												data-secondary={group.id === "digital"}
												key={group.id}
											>
												<i aria-hidden="true">{groupGlyphs[group.id]}</i>
												<b>{group.label}</b>
												<small>
													{group.connected} / {group.entries.length}
												</small>
											</li>
										))}
									</ul>
								</div>
							)}
						</div>
					</div>

					<fieldset className="world-time-strip">
						<legend className="sr-only">主要金融城市时间</legend>
						{worldClockZones.map((zone) => (
							<button
								key={zone.id}
								type="button"
								onClick={() => setClockZone(zone.id)}
							>
								<span>{zone.label}</span>
								<time dateTime={instant?.toISOString()}>
									{instant ? formatTime(instant, zone.id) : "--:--"}
								</time>
								<small>{zone.short}</small>
							</button>
						))}
					</fieldset>

					<footer className="relay-coverage" style={coverageStyle}>
						<div className="coverage-meter">
							<span>
								规则覆盖 {connectedVenues.length} / {marketCount}
							</span>
							<i aria-hidden="true" />
						</div>
						<div className="relay-provenance">
							<span>RULESET</span>
							<b title={revisionLabel}>{revisionLabel}</b>
							<span>TZDB</span>
							<b>{payload?.tzdb_version ?? "—"}</b>
						</div>
						<a href="/settings#source-intelligence">
							{marketCount - connectedVenues.length} 个等待数据源 →
						</a>
					</footer>
				</section>
			</section>

			<section
				className="utc-ruler"
				style={cursorStyle}
				aria-label="UTC 日内标尺"
			>
				<div className="ruler-title">
					<strong>UTC</strong>
					<span>
						{snapshotMode === "inspection"
							? "正在检查指定时刻"
							: "拖动检查任意时刻"}
					</span>
					{snapshotMode === "inspection" ? (
						<button
							className="ruler-live-reset"
							type="button"
							onClick={() => {
								setScrubPreview(null);
								setInspectionAt(null);
							}}
						>
							返回实时
						</button>
					) : null}
				</div>
				<div className="ruler-track">
					<div className="ruler-hours" aria-hidden="true">
						{Array.from({ length: 13 }, (_, index) => index * 2).map(
							(hour, index) => (
								<i
									key={hour}
									style={
										{
											"--axis-position": `${(index / 12) * 100}%`,
										} as CSSProperties
									}
								>
									{String(hour).padStart(2, "0")}
								</i>
							),
						)}
					</div>
					<div className="ruler-line" />
					<Slider
						className="timeline-slider timeline-slider-global"
						disabled={!payload}
						max={1_439}
						min={0}
						onValueChange={scrubToMinute}
						onValueCommitted={commitScrubber}
						step={1}
						thumbAriaLabel="选择要检查的 UTC 时刻"
						tooltipVisibility="never"
						value={cursorMinute}
					/>
					{cursor !== null && cursorInstant ? (
						<span className="now-marker">
							<b>{cursorLabel}</b>
							<small>{formatTime(cursorInstant, "UTC")}</small>
						</span>
					) : null}
				</div>
			</section>

			<section className="market-explorer" aria-label="市场目录与交易时段">
				<header className="explorer-header">
					<div>
						<p className="section-kicker">MARKET DIRECTORY</p>
						<h2>{selectedLabel}</h2>
						<p>
							当前显示 {visibleCount}{" "}
							个市场；当地时间由同一服务端时刻换算，交易状态只读取规则数据。
						</p>
					</div>
					<div className="explorer-stats">
						<span>
							<i className="state-dot is-trading" aria-hidden="true" />
							交易中 {counts.trading}
						</span>
						<span>
							<i className="state-dot is-closed" aria-hidden="true" />
							休市 {counts.notTrading}
						</span>
						<span>
							<i className="state-dot is-unknown" aria-hidden="true" />
							Unknown {counts.unknown}
						</span>
					</div>
				</header>

				<div
					className="catalog-layout"
					data-sidebar-collapsed={catalogSidebarCollapsed}
				>
					<aside
						className="catalog-sidebar"
						data-collapsed={catalogSidebarCollapsed}
					>
						<div className="catalog-sidebar-toolbar">
							<strong>市场筛选</strong>
							<button
								aria-controls="catalog-sidebar-content"
								aria-expanded={!catalogSidebarCollapsed}
								aria-label={
									catalogSidebarCollapsed ? "展开市场筛选栏" : "收起市场筛选栏"
								}
								onClick={() =>
									setCatalogSidebarCollapsed((collapsed) => !collapsed)
								}
								title={catalogSidebarCollapsed ? "展开筛选栏" : "收起筛选栏"}
								type="button"
							>
								<span aria-hidden="true">
									{catalogSidebarCollapsed ? "→" : "←"}
								</span>
							</button>
						</div>

						{catalogSidebarCollapsed ? null : (
							<Input
								aria-label="搜索市场、交易所或代码"
								className="market-search"
								clearable
								inputSize="md"
								onChange={(event) => setQuery(event.target.value)}
								onClear={() => setQuery("")}
								placeholder="搜索市场或代码"
								startSlot={<span aria-hidden="true">⌕</span>}
								value={query}
							/>
						)}

						<nav
							className="group-nav"
							aria-label="市场类别"
							id="catalog-sidebar-content"
						>
							<button
								aria-label={`全部市场，${marketCount} 个`}
								aria-pressed={activeGroup === "all"}
								className={activeGroup === "all" ? "is-active" : undefined}
								onClick={() => setActiveGroup("all")}
								type="button"
							>
								<i aria-hidden="true">⌘</i>
								<span>全部市场</span>
								<small>{marketCount}</small>
							</button>
							{catalogGroups.map((group) => (
								<button
									aria-label={`${group.label}，${group.entries.length} 个`}
									aria-pressed={activeGroup === group.id}
									className={activeGroup === group.id ? "is-active" : undefined}
									data-secondary={group.id === "digital"}
									key={group.id}
									onClick={() => setActiveGroup(group.id)}
									type="button"
								>
									<i aria-hidden="true">{groupGlyphs[group.id]}</i>
									<span>{group.label}</span>
									<small>{group.entries.length}</small>
								</button>
							))}
						</nav>

						{catalogSidebarCollapsed ? null : (
							<div className="source-principle">
								<i aria-hidden="true">✓</i>
								<p>
									<b>不猜开休市</b>
									<span>空轨道表示运营方尚未配置带证据的规则数据。</span>
								</p>
							</div>
						)}
					</aside>

					<div className="market-table-scroll">
						<div
							className="market-table"
							aria-busy={loading}
							data-has-cursor={cursor !== null}
							style={cursorStyle}
						>
							<div className="table-axis">
								<span aria-hidden="true">市场 / 交易所</span>
								<span aria-hidden="true">当地时间</span>
								<div className="axis-hours">
									<b aria-hidden="true">服务端 UTC 交易阶段 · 拖动选时刻</b>
									<Slider
										className="timeline-slider timeline-slider-table"
										disabled={!payload}
										max={1_439}
										min={0}
										onValueChange={scrubToMinute}
										onValueCommitted={commitScrubber}
										step={1}
										thumbAriaLabel="从市场时间轴选择要检查的 UTC 时刻"
										tooltipVisibility="never"
										value={cursorMinute}
									/>
									{cursor !== null && cursorInstant ? (
										<span className="axis-now">
											<strong>{cursorLabel}</strong>
											<small>{formatTime(cursorInstant, "UTC")} UTC</small>
										</span>
									) : null}
									{Array.from({ length: 7 }, (_, index) => index * 4).map(
										(hour, index) => (
											<i
												aria-hidden="true"
												key={hour}
												style={
													{
														"--axis-position": `${(index / 6) * 100}%`,
													} as CSSProperties
												}
											>
												{String(hour).padStart(2, "0")}
											</i>
										),
									)}
								</div>
								<span aria-hidden="true">当前状态</span>
							</div>

							{prioritizedGroups.map((group) => (
								<section className="catalog-group" key={group.id}>
									<header className="catalog-group-header">
										<div>
											<i aria-hidden="true">{groupGlyphs[group.id]}</i>
											<h3>{group.label}</h3>
											<small>{group.entries.length}</small>
										</div>
										{group.id === "equities" ? (
											<p>按交易中、临近边界与下一开盘自动排序。</p>
										) : group.id === "digital" ? (
											<p>
												维护、资金费率与产品规则仍需服务端证据，目录不等于默认
												24×7。
											</p>
										) : null}
									</header>
									{group.entries.map((entry) => (
										<CatalogMarketRow
											attention={attentionByVenueId.get(entry.id)?.kind}
											entry={entry}
											instant={instant}
											key={entry.id}
											payload={payload}
											venue={venuesById.get(entry.id)}
										/>
									))}
								</section>
							))}

							{filteredGroups.length === 0 ? (
								<div className="empty-results">
									<strong>没有匹配的市场</strong>
									<button type="button" onClick={() => setQuery("")}>
										清除搜索
									</button>
								</div>
							) : null}
						</div>
					</div>
				</div>
			</section>

			<footer className="instrument-footer">
				<p>
					真实市场规则 {connectedVenues.length} / {marketCount}{" "}
					已接入；空轨道不推断开休市。
				</p>
				<a href="/settings#source-intelligence">查看数据源、证据与修订</a>
				<p>
					{payload
						? `TZDB ${payload.tzdb_version} · 修订详见设置`
						: "等待服务端修订信息"}
				</p>
			</footer>
		</main>
	);
}
