"use client";

import { Badge } from "@appica/ui-react/badge";
import { Button } from "@appica/ui-react/button";
import { useLocalStorage } from "@appica/ui-react/hooks/use-local-storage";
import { Progress } from "@appica/ui-react/progress";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@appica/ui-react/select";
import { Switch } from "@appica/ui-react/switch";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "@appica/ui-react/tabs";
import { useEffect, useMemo, useState } from "react";
import { marketCatalogGroups } from "../../lib/market-catalog";
import {
	isStatusPayload,
	type StatusPayload,
	summarizeSources,
} from "../../lib/market-time";
import {
	defaultReminderPreferences,
	PRIMARY_ZONE_KEY,
	primaryTimeZones,
	REMINDER_PREFERENCES_KEY,
	reminderLeadOptions,
} from "../../lib/preferences";
import { ThemeToggle } from "../theme-toggle";

const API_ORIGIN = process.env.NEXT_PUBLIC_MARK_TIME_API ?? "";
const marketIds = new Set(
	marketCatalogGroups.flatMap((group) =>
		group.entries.map((entry) => entry.id.toLowerCase()),
	),
);
function formatSourceStamp(value: string) {
	return `${new Intl.DateTimeFormat("zh-CN", {
		month: "2-digit",
		day: "2-digit",
		hour: "2-digit",
		minute: "2-digit",
		hour12: false,
		timeZone: "UTC",
	}).format(new Date(value))} UTC`;
}

export default function SettingsPage() {
	const [status, setStatus] = useState<StatusPayload | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [reload, setReload] = useState(0);
	const [primaryZone, setPrimaryZone] = useLocalStorage(
		PRIMARY_ZONE_KEY,
		"Asia/Shanghai",
	);
	const [reminderPreferences, setReminderPreferences] = useLocalStorage(
		REMINDER_PREFERENCES_KEY,
		defaultReminderPreferences,
	);
	const [notificationPermission, setNotificationPermission] = useState<
		NotificationPermission | "unsupported"
	>("unsupported");
	const [preferenceNotice, setPreferenceNotice] = useState<string | null>(null);

	useEffect(() => {
		setNotificationPermission(
			"Notification" in window ? Notification.permission : "unsupported",
		);
	}, []);

	useEffect(() => {
		void reload;
		const controller = new AbortController();

		async function loadStatus() {
			setError(null);
			try {
				const response = await fetch(`${API_ORIGIN}/v1/status`, {
					headers: { Accept: "application/json" },
					signal: controller.signal,
				});
				const body: unknown = await response.json().catch(() => ({}));
				if (!response.ok) throw new Error(`HTTP ${response.status}`);
				if (!isStatusPayload(body)) throw new Error("状态接口格式无效");
				setStatus(body);
			} catch (loadError) {
				if (controller.signal.aborted) return;
				setError(
					loadError instanceof Error ? loadError.message : "读取来源失败",
				);
			}
		}

		void loadStatus();
		return () => controller.abort();
	}, [reload]);

	const sources = useMemo(
		() => (status ? summarizeSources(status) : []),
		[status],
	);
	const connectedIds = new Set(
		(status?.venues ?? []).map((venue) => venue.id.toLowerCase()),
	);
	const connected = connectedIds.size;
	const marketCount = new Set([...marketIds, ...connectedIds]).size;
	const coverage = marketCount ? (connected / marketCount) * 100 : 0;
	const revision = status?.dataset_revisions.at(-1) ?? "未连接规则集";
	const browserZone =
		typeof Intl === "undefined"
			? "UTC"
			: Intl.DateTimeFormat().resolvedOptions().timeZone;
	const zoneOptions = primaryTimeZones.some((zone) => zone.id === browserZone)
		? primaryTimeZones
		: [
				...primaryTimeZones,
				{ id: browserZone, label: "本机时区", code: "LOCAL" },
			];

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
			setPreferenceNotice("浏览器未授权通知，提醒保持关闭。");
			return;
		}
		setReminderPreferences((current) => ({ ...current, enabled: true }));
	}

	return (
		<main className="page-shell settings-shell" data-ui-framework="next-appica">
			<h1 className="sr-only">MARK TIME 时间字典与设置</h1>

			<nav className="topbar" aria-label="主导航">
				<a className="wordmark" href="/" aria-label="MARK TIME 首页">
					MARKET <b>TIME</b>
				</a>
				<div className="topbar-links">
					<a href="/">全球交易时间表</a>
					<a href="/settings" aria-current="page">
						时间字典与设置
					</a>
				</div>
				<div className="connection-state" aria-live="polite">
					<i
						data-mode={error ? "ERROR" : status ? "LIVE" : "LOADING"}
						aria-hidden="true"
					/>
					<span>{error ? "来源离线" : status ? "证据在线" : "读取中"}</span>
					<Button
						size="icon-sm"
						variant="ghost"
						onClick={() => setReload((value) => value + 1)}
						aria-label="刷新数据源状态"
					>
						↻
					</Button>
				</div>
			</nav>

			<header className="settings-heading">
				<div>
					<p className="section-kicker">SETTINGS</p>
					<h2>时间字典与设置</h2>
					<p className="settings-description">
						集中管理显示时区、交易提醒、页面外观、数据证据与修订。
					</p>
				</div>
				<nav aria-label="设置类别">
					<a href="#time-preferences" aria-current="page">
						时间字典与提醒
					</a>
					<a href="#appearance">页面外观</a>
					<a href="#source-intelligence">数据源与修订</a>
				</nav>
			</header>

			{error ? (
				<aside className="status-notice" role="status">
					<span>读取失败：{error}</span>
					<Button
						size="sm"
						variant="outline"
						onClick={() => setReload((value) => value + 1)}
					>
						重新读取
					</Button>
				</aside>
			) : null}

			<section
				className="time-preferences"
				id="time-preferences"
				aria-labelledby="time-preferences-title"
			>
				<header>
					<div>
						<p className="section-kicker">TIME DICTIONARY</p>
						<h2 id="time-preferences-title">时间字典与提醒</h2>
						<p>主时钟只改变显示时区；交易边界继续读取服务端规则与 TZDB。</p>
					</div>
					<dl>
						<div>
							<dt>服务端 TZDB</dt>
							<dd>{status?.tzdb_version ?? "读取中"}</dd>
						</div>
						<div>
							<dt>本机时区</dt>
							<dd>{browserZone}</dd>
						</div>
					</dl>
				</header>

				<div className="preference-grid">
					<div className="preference-card">
						<span>
							<b>主时钟城市</b>
							<small>首页、地图和世界时钟共享这一选择。</small>
						</span>
						<Select
							value={primaryZone}
							onValueChange={(value) =>
								typeof value === "string" && setPrimaryZone(value)
							}
						>
							<SelectTrigger aria-label="主时钟城市">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{zoneOptions.map((zone) => (
									<SelectItem key={zone.id} value={zone.id}>
										{zone.label} · {zone.id}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</div>

					<div className="preference-card">
						<span>
							<b>开收盘提醒</b>
							<small>页面打开时，按服务端下一交易边界发送系统通知。</small>
						</span>
						<div className="reminder-settings">
							<Select
								value={String(reminderPreferences.leadMinutes)}
								onValueChange={(value) =>
									typeof value === "string" &&
									setReminderPreferences((current) => ({
										...current,
										leadMinutes: Number(value),
									}))
								}
							>
								<SelectTrigger aria-label="提前提醒时间">
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									{reminderLeadOptions.map((minutes) => (
										<SelectItem key={minutes} value={String(minutes)}>
											{minutes ? `提前 ${minutes} 分钟` : "到点提醒"}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
							<Switch
								aria-label="开收盘提醒"
								checked={reminderPreferences.enabled}
								onCheckedChange={(checked) => void toggleReminders(checked)}
							/>
						</div>
					</div>

					<div className="preference-card" id="appearance">
						<span>
							<b>页面外观</b>
							<small>全站共用亮色或暗色模式，只在这里调整。</small>
						</span>
						<ThemeToggle />
					</div>
				</div>
				<footer>
					<span>
						{notificationPermission === "granted"
							? "浏览器通知已授权"
							: notificationPermission === "denied"
								? "浏览器通知已拒绝"
								: notificationPermission === "unsupported"
									? "浏览器不支持通知"
									: "开启时请求通知权限"}
					</span>
					{preferenceNotice ? <strong>{preferenceNotice}</strong> : null}
				</footer>
			</section>

			<section
				className="source-intelligence"
				id="source-intelligence"
				aria-labelledby="source-intelligence-title"
			>
				<header className="source-intelligence-header">
					<div>
						<p className="section-kicker">SOURCE INTELLIGENCE</p>
						<h2 id="source-intelligence-title">真实数据，受控接入</h2>
						<p>
							Agent
							发现并解析候选规则；许可检查、黄金向量与人工批准决定它能否成为新修订。
						</p>
					</div>
					<div className="source-score">
						<strong>{connected}</strong>
						<span>/ {marketCount} 已接入</span>
						<Progress value={coverage} aria-label="真实市场规则覆盖率" />
					</div>
				</header>

				<div className="source-metrics">
					<div>
						<span>运行时来源</span>
						<strong>{sources.length}</strong>
						<small>按原始 URL 去重</small>
					</div>
					<div>
						<span>已接入市场</span>
						<strong>{connected}</strong>
						<small>状态由服务端规则计算</small>
					</div>
					<div>
						<span>等待来源</span>
						<strong>{marketCount - connected}</strong>
						<small>保留为明确未配置</small>
					</div>
					<div>
						<span>当前修订</span>
						<strong className="revision-metric" title={revision}>
							{revision}
						</strong>
						<small>
							{status ? `TZDB ${status.tzdb_version}` : "等待状态接口"}
						</small>
					</div>
				</div>

				<Tabs className="source-tabs" defaultValue="runtime" variant="line">
					<TabsList aria-label="数据接入视图">
						<TabsTrigger value="runtime">
							运行时来源 <span>{sources.length}</span>
						</TabsTrigger>
						<TabsTrigger value="intake">智能接入协议</TabsTrigger>
					</TabsList>

					<TabsContent value="runtime">
						<div className="runtime-source-panel">
							<header>
								<div>
									<strong>当前服务实际引用的证据</strong>
									<p>
										这里只展示 `/v1/status`
										返回的来源，不从市场目录或浏览器推断。
									</p>
								</div>
								<Badge
									variant={sources.length ? "success" : "warning"}
									size="sm"
								>
									{sources.length ? "证据在线" : "等待证据"}
								</Badge>
							</header>

							{sources.length ? (
								<ul className="runtime-source-list">
									{sources.map((source) => (
										<li key={source.url}>
											<i aria-hidden="true">↗</i>
											<div>
												<strong>{source.host}</strong>
												<small>{source.venues.join(" · ")}</small>
											</div>
											<dl>
												<div>
													<dt>最近抓取</dt>
													<dd>
														<time dateTime={source.fetchedAt}>
															{formatSourceStamp(source.fetchedAt)}
														</time>
													</dd>
												</div>
												<div>
													<dt>规则生效</dt>
													<dd>{source.effectiveFrom}</dd>
												</div>
											</dl>
											<a href={source.url} rel="noreferrer" target="_blank">
												原始来源
											</a>
										</li>
									))}
								</ul>
							) : (
								<div className="source-empty">
									<strong>状态接口尚未返回可验证来源</strong>
									<p>市场目录仍可使用，但不会把未配置市场解释为休市。</p>
								</div>
							)}
						</div>
					</TabsContent>

					<TabsContent value="intake">
						<div className="intake-panel">
							<ol className="intake-steps">
								<li>
									<i>01</i>
									<div>
										<strong>发现官方来源</strong>
										<p>Agent 搜索交易所规则、日历与公告，聚合站只作线索。</p>
									</div>
									<span>AGENT</span>
								</li>
								<li>
									<i>02</i>
									<div>
										<strong>登记条款与访问方式</strong>
										<p>运营方确认许可；不允许再分发的数据只在运行时抓取。</p>
									</div>
									<span>OPERATOR</span>
								</li>
								<li>
									<i>03</i>
									<div>
										<strong>提取候选规则</strong>
										<p>
											Agent 输出证据、覆盖范围、不确定性和可复现的候选修订。
										</p>
									</div>
									<span>AGENT</span>
								</li>
								<li>
									<i>04</i>
									<div>
										<strong>验证后发布</strong>
										<p>
											黄金向量、许可门和人工审阅全通过，才生成不可变 revision。
										</p>
									</div>
									<span>HUMAN + CI</span>
								</li>
							</ol>

							<aside className="agent-contract">
								<p>AGENT CONTRACT</p>
								<h3>机器先读事实，再决定动作</h3>
								<code>GET /v1/timeline</code>
								<code>GET /v1/status</code>
								<dl>
									<div>
										<dt>REVISION</dt>
										<dd>{revision}</dd>
									</div>
									<div>
										<dt>UNKNOWN</dt>
										<dd>独立值，不等于休市</dd>
									</div>
								</dl>
								<p>
									网页负责监督；抓取与 revision assembly 仍在 `market-time-data`
									边界执行。
								</p>
								<a href="#source-intelligence">检查证据与修订 →</a>
							</aside>
						</div>
					</TabsContent>
				</Tabs>
			</section>
		</main>
	);
}
