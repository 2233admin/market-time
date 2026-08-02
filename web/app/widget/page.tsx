"use client";

import { useLocalStorage } from "@appica/ui-react/hooks/use-local-storage";
import { useCallback, useEffect, useMemo, useState } from "react";
import "./widget.css";
import {
	hideDesktopWidget,
	isDesktopRuntime,
	notificationPermission,
	requestNotificationPermission,
	type SystemNotificationPermission,
	sendSystemNotification,
} from "../../lib/desktop";
import {
	currentSegment,
	formatZonedTime,
	isStatusPayload,
	isTimelinePayload,
	rankVenueAttention,
	type StatusPayload,
	serverAnchoredNow,
	staleSnapshotReason,
	statusMatchesTimeline,
	type TimelinePayload,
	transitionReminders,
	type VenueAttention,
} from "../../lib/market-time";
import {
	defaultReminderPreferences,
	REMINDER_PREFERENCES_KEY,
	reminderLeadOptions,
	WIDGET_NOTIFIED_REMINDERS_KEY,
} from "../../lib/preferences";

const API_ORIGIN =
	process.env.NEXT_PUBLIC_MARK_TIME_API ?? "http://127.0.0.1:8080";
const POLL_MS = 30_000;
const STALE_AFTER_MS = 75_000;
const MAX_NOTIFICATION_DELAY_MS = 24 * 60 * 60 * 1_000;
const MAX_LATE_NOTIFICATION_MS = 5 * 60 * 1_000;

interface Snapshot {
	timeline: TimelinePayload;
	status: StatusPayload | null;
	receivedAt: number;
}

interface ConnectionError {
	message: string;
	observedAt: number;
}

const phaseLabels: Record<string, string> = {
	closed: "休市",
	pre_open: "盘前",
	opening_auction: "开盘集合竞价",
	continuous_trading: "交易中",
	mid_day_break: "午间休市",
	closing_auction: "收盘集合竞价",
	post_close: "盘后",
	halted: "停牌",
};

function formatCountdown(
	transitionAt: string | null,
	now: Date,
	stale: boolean,
) {
	if (stale) return "离线暂停";
	if (!transitionAt) return "暂无边界";
	const remaining = Date.parse(transitionAt) - now.getTime();
	if (remaining <= 0) return "边界已到";
	const totalSeconds = Math.floor(remaining / 1_000);
	const hours = Math.floor(totalSeconds / 3_600);
	const minutes = Math.floor((totalSeconds % 3_600) / 60);
	const seconds = totalSeconds % 60;
	return hours
		? `${hours}h ${String(minutes).padStart(2, "0")}m`
		: `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function notificationCopy(
	reminder: ReturnType<typeof transitionReminders>[number],
) {
	const boundary = reminder.kind === "opens" ? "开盘" : "收盘";
	return {
		title:
			reminder.notification === "approaching"
				? `${reminder.venueName} 临近${boundary}`
				: `${reminder.venueName} 已${boundary}`,
		body:
			reminder.notification === "approaching"
				? `${boundary}时间来自当前服务端规则。`
				: `服务端发布的${boundary}边界已到。`,
	};
}

function notifiedReminderKeys() {
	try {
		const stored: unknown = JSON.parse(
			localStorage.getItem(WIDGET_NOTIFIED_REMINDERS_KEY) ?? "[]",
		);
		return Array.isArray(stored)
			? stored.filter((value): value is string => typeof value === "string")
			: [];
	} catch {
		return [];
	}
}

function MarketCard({
	attention,
	now,
	stale,
	evidence,
}: {
	attention: VenueAttention;
	now: Date;
	stale: boolean;
	evidence: number | null;
}) {
	const segment = currentSegment(attention.venue);
	const unknown = !segment || segment.status === "unknown";
	const transition = attention.venue.next_trading_transition;
	const state = unknown
		? "UNKNOWN"
		: (phaseLabels[segment.phase ?? ""] ?? segment.phase ?? "已知状态");

	return (
		<article
			className="widget-market"
			data-state={unknown ? "unknown" : attention.kind}
		>
			<header>
				<div>
					<strong>{attention.venue.display_name}</strong>
					<small>{attention.venue.id}</small>
				</div>
				<time dateTime={now.toISOString()}>
					{formatZonedTime(now, attention.venue.home_zone)}
				</time>
			</header>
			<div className="widget-market-state">
				<span>{state}</span>
				<b>{formatCountdown(attention.transitionAt, now, stale)}</b>
			</div>
			<footer>
				<span>
					{transition
						? `下一次${transition.kind === "opens" ? "开盘" : "收盘"}`
						: unknown
							? "覆盖外，不推断"
							: "暂无下一交易边界"}
				</span>
				<span>{evidence === null ? "证据未读取" : `证据 ${evidence}`}</span>
			</footer>
		</article>
	);
}

export default function WidgetPage() {
	const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
	const [error, setError] = useState<ConnectionError | null>(null);
	const [evidenceError, setEvidenceError] = useState(false);
	const [reload, setReload] = useState(0);
	const [tick, setTick] = useState(0);
	const [settingsOpen, setSettingsOpen] = useState(false);
	const [desktop, setDesktop] = useState(false);
	const [permission, setPermission] =
		useState<SystemNotificationPermission>("unsupported");
	const [notice, setNotice] = useState<string | null>(null);
	const [reminders, setReminders] = useLocalStorage(
		REMINDER_PREFERENCES_KEY,
		defaultReminderPreferences,
	);

	useEffect(() => {
		setDesktop(isDesktopRuntime());
		void notificationPermission().then(setPermission, () =>
			setPermission("unsupported"),
		);
		const timer = window.setInterval(() => setTick(performance.now()), 1_000);
		const poll = window.setInterval(
			() => setReload((value) => value + 1),
			POLL_MS,
		);
		setTick(performance.now());
		return () => {
			window.clearInterval(timer);
			window.clearInterval(poll);
		};
	}, []);

	useEffect(() => {
		void reload;
		const controller = new AbortController();
		async function load() {
			try {
				const timelineResponse = await fetch(`${API_ORIGIN}/v1/timeline`, {
					cache: "no-store",
					headers: { Accept: "application/json" },
					signal: controller.signal,
				});
				const timelineBody: unknown = await timelineResponse
					.json()
					.catch(() => ({}));
				if (!timelineResponse.ok)
					throw new Error(`HTTP ${timelineResponse.status}`);
				if (!isTimelinePayload(timelineBody))
					throw new Error("时间线接口格式无效");
				const timelineReceivedAt = performance.now();
				setSnapshot({
					timeline: timelineBody,
					status: null,
					receivedAt: timelineReceivedAt,
				});
				setEvidenceError(true);
				setError(null);

				let status: StatusPayload | null = null;
				try {
					const statusResponse = await fetch(
						`${API_ORIGIN}/v1/status?at=${encodeURIComponent(timelineBody.at)}`,
						{
							cache: "no-store",
							headers: { Accept: "application/json" },
							signal: controller.signal,
						},
					);
					const statusBody: unknown = await statusResponse
						.json()
						.catch(() => ({}));
					if (
						statusResponse.ok &&
						isStatusPayload(statusBody) &&
						statusMatchesTimeline(statusBody, timelineBody)
					)
						status = statusBody;
				} catch {
					if (controller.signal.aborted) return;
				}

				setSnapshot({
					timeline: timelineBody,
					status,
					receivedAt: timelineReceivedAt,
				});
				setEvidenceError(status === null);
			} catch (loadError) {
				if (controller.signal.aborted) return;
				const observedAt = performance.now();
				setTick(observedAt);
				setError({
					message:
						loadError instanceof Error ? loadError.message : "服务不可用",
					observedAt,
				});
			}
		}
		void load();
		return () => controller.abort();
	}, [reload]);

	const age = snapshot ? Math.max(0, tick - snapshot.receivedAt) : 0;
	const staleReason = staleSnapshotReason(
		error?.message ?? null,
		snapshot !== null,
		age,
		STALE_AFTER_MS,
	);
	const stale = staleReason !== null;
	const frozenAt = snapshot
		? error
			? Math.min(error.observedAt, snapshot.receivedAt + STALE_AFTER_MS)
			: age > STALE_AFTER_MS
				? snapshot.receivedAt + STALE_AFTER_MS
				: null
		: null;
	const now = useMemo(() => {
		if (!snapshot) return new Date(Number.NaN);
		return serverAnchoredNow(
			snapshot.timeline.at,
			snapshot.receivedAt,
			tick,
			frozenAt,
		);
	}, [frozenAt, snapshot, tick]);
	const ranked = useMemo(
		() =>
			snapshot && Number.isFinite(now.getTime())
				? rankVenueAttention(snapshot.timeline.venues, now).slice(0, 4)
				: [],
		[now, snapshot],
	);
	const evidenceByVenue = useMemo(
		() =>
			new Map(
				(snapshot?.status?.venues ?? []).map((venue) => [
					venue.id.toLowerCase(),
					venue.evidence.length,
				]),
			),
		[snapshot?.status],
	);
	const revision =
		snapshot?.timeline.dataset_revisions.join(" · ") || "未连接规则集";
	const selected = new Set(
		reminders.venueIds?.map((id) => id.toLowerCase()) ?? [],
	);

	useEffect(() => {
		if (!snapshot || stale || !reminders.enabled || permission !== "granted")
			return;
		const serverNow = new Date(
			Date.parse(snapshot.timeline.at) +
				Math.max(0, performance.now() - snapshot.receivedAt),
		);
		const scheduled = transitionReminders(
			snapshot.timeline.venues,
			serverNow,
			reminders.leadMinutes,
			reminders.venueIds,
		);
		const timers = scheduled.flatMap((reminder) => {
			const delay = Date.parse(reminder.fireAt) - serverNow.getTime();
			if (
				delay > MAX_NOTIFICATION_DELAY_MS ||
				delay < -MAX_LATE_NOTIFICATION_MS
			)
				return [];
			return [
				window.setTimeout(
					() => {
						const lateBy =
							Date.parse(snapshot.timeline.at) +
							(performance.now() - snapshot.receivedAt) -
							Date.parse(reminder.fireAt);
						if (lateBy > MAX_LATE_NOTIFICATION_MS) return;
						const keys = notifiedReminderKeys();
						if (keys.includes(reminder.key)) return;
						const copy = notificationCopy(reminder);
						void sendSystemNotification(
							copy.title,
							`${copy.body} · ${revision}`,
						).then(
							() =>
								localStorage.setItem(
									WIDGET_NOTIFIED_REMINDERS_KEY,
									JSON.stringify([...keys.slice(-49), reminder.key]),
								),
							() => setNotice("系统通知发送失败。"),
						);
					},
					Math.max(0, delay),
				),
			];
		});
		return () => timers.forEach(window.clearTimeout);
	}, [permission, reminders, revision, snapshot, stale]);

	const toggleReminders = useCallback(
		async (enabled: boolean) => {
			setNotice(null);
			if (!enabled) {
				setReminders((current) => ({ ...current, enabled: false }));
				return;
			}
			try {
				const nextPermission = await requestNotificationPermission();
				setPermission(nextPermission);
				if (nextPermission !== "granted") {
					setNotice("系统通知未授权，提醒保持关闭。");
					return;
				}
				setReminders((current) => ({ ...current, enabled: true }));
			} catch {
				setPermission("unsupported");
				setNotice("系统通知不可用，提醒保持关闭。");
			}
		},
		[setReminders],
	);

	function toggleVenue(id: string) {
		setReminders((current) => {
			const venueIds = new Set(
				current.venueIds ?? snapshot?.timeline.venues.map((v) => v.id),
			);
			if (venueIds.has(id)) venueIds.delete(id);
			else venueIds.add(id);
			return { ...current, venueIds: [...venueIds] };
		});
	}

	async function hideWidget() {
		setNotice(null);
		try {
			if (!(await hideDesktopWidget())) setNotice("当前环境无法隐藏悬浮窗。");
		} catch {
			setNotice("悬浮窗隐藏失败，请从托盘退出。");
		}
	}

	return (
		<main className="widget-page" data-ui-framework="next-appica">
			<header className="widget-titlebar" data-tauri-drag-region>
				<div data-tauri-drag-region>
					<i aria-hidden="true" />
					<span>
						MARKET <b>TIME</b>
					</span>
				</div>
				<nav aria-label="悬浮窗操作">
					<button
						type="button"
						onClick={() => setReload((value) => value + 1)}
						aria-label="刷新"
					>
						↻
					</button>
					<button
						type="button"
						onClick={() => setSettingsOpen((open) => !open)}
						aria-label="设置"
					>
						⚙
					</button>
					<button
						type="button"
						onClick={() => void hideWidget()}
						aria-label="隐藏"
						disabled={!desktop}
					>
						—
					</button>
				</nav>
			</header>

			<section className="widget-clock" aria-live="polite">
				<div>
					<span>
						{snapshot
							? stale
								? "LAST KNOWN SNAPSHOT"
								: "SERVER-ANCHORED NOW"
							: "NO SNAPSHOT"}
					</span>
					<strong>
						{snapshot
							? formatZonedTime(
									now,
									Intl.DateTimeFormat().resolvedOptions().timeZone,
								)
							: "--:--:--"}
					</strong>
				</div>
				<p data-state={stale ? "offline" : snapshot ? "live" : "loading"}>
					<i aria-hidden="true" />
					{stale ? "服务离线" : snapshot ? "规则在线" : "正在连接"}
				</p>
			</section>

			{staleReason ? (
				<aside className="widget-notice">
					{snapshot
						? "保留最后快照，不继续推算状态或提醒："
						: "尚未取得可用快照："}
					{staleReason}
				</aside>
			) : null}
			{notice ? <aside className="widget-notice">{notice}</aside> : null}

			{settingsOpen ? (
				<section className="widget-settings" aria-label="悬浮窗设置">
					<label className="widget-setting-row">
						<span>
							<b>桌面提醒</b>
							<small>
								{permission === "granted" ? "系统通知已授权" : "开启时请求授权"}
							</small>
						</span>
						<input
							type="checkbox"
							checked={reminders.enabled}
							onChange={(event) => void toggleReminders(event.target.checked)}
						/>
					</label>
					<label className="widget-setting-row">
						<span>
							<b>临近边界</b>
							<small>到点仍会单独提醒</small>
						</span>
						<select
							value={reminders.leadMinutes}
							onChange={(event) =>
								setReminders((current) => ({
									...current,
									leadMinutes: Number(event.target.value),
								}))
							}
						>
							{reminderLeadOptions.map((minutes) => (
								<option key={minutes} value={minutes}>
									{minutes ? `提前 ${minutes} 分钟` : "仅到点"}
								</option>
							))}
						</select>
					</label>
					<div className="widget-scope">
						<header>
							<b>提醒市场</b>
							<button
								type="button"
								onClick={() =>
									setReminders((current) => ({
										...current,
										venueIds: undefined,
									}))
								}
							>
								{reminders.venueIds ? "全部市场" : "已选全部"}
							</button>
						</header>
						{reminders.venueIds ? (
							snapshot?.timeline.venues.map((venue) => (
								<label key={venue.id}>
									<input
										type="checkbox"
										checked={selected.has(venue.id.toLowerCase())}
										onChange={() => toggleVenue(venue.id)}
									/>
									<span>{venue.display_name}</span>
								</label>
							))
						) : (
							<button
								type="button"
								className="widget-customize"
								onClick={() =>
									setReminders((current) => ({
										...current,
										venueIds:
											snapshot?.timeline.venues.map((venue) => venue.id) ?? [],
									}))
								}
							>
								改为按市场选择
							</button>
						)}
					</div>
					<button
						className="widget-hide-action"
						type="button"
						onClick={() => void hideWidget()}
						disabled={!desktop}
					>
						隐藏悬浮窗（托盘可恢复）
					</button>
				</section>
			) : (
				<section className="widget-markets" aria-label="重要市场状态">
					{ranked.length ? (
						ranked.map((attention) => (
							<MarketCard
								key={attention.venue.id}
								attention={attention}
								now={now}
								stale={stale}
								evidence={
									evidenceByVenue.get(attention.venue.id.toLowerCase()) ?? null
								}
							/>
						))
					) : (
						<div className="widget-empty">
							<b>UNKNOWN</b>
							<span>尚未取得服务端市场状态。</span>
						</div>
					)}
				</section>
			)}

			<footer className="widget-footer">
				<span title={revision}>REV {revision}</span>
				<span>
					{snapshot?.timeline.tzdb_version
						? `TZDB ${snapshot.timeline.tzdb_version}`
						: "TZDB —"}
				</span>
				<span>
					{evidenceError
						? "证据接口不可用"
						: snapshot
							? `${snapshot.timeline.clock.discipline} · 精度未测量`
							: "等待时钟证据"}
				</span>
			</footer>
		</main>
	);
}
