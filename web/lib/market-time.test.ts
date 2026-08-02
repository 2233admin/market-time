import { describe, expect, it } from "vitest";

import {
	catalogWithServerVenues,
	filterMarketCatalog,
	flagImageUrl,
	marketCatalogGroups,
} from "./market-catalog";
import {
	countSnapshot,
	currentSegment,
	cursorPercent,
	formatZonedTime,
	isStatusPayload,
	isTimelinePayload,
	marketHubState,
	nextTransitionReminder,
	rankVenueAttention,
	type StatusPayload,
	serverAnchoredNow,
	staleSnapshotReason,
	statusMatchesTimeline,
	summarizeSources,
	type TimelinePayload,
	transitionReminders,
	utcInstantAtPercent,
	type VenueTimeline,
	viewMode,
} from "./market-time";

const validPayload = {
	at: "2026-08-01T12:00:00Z",
	clock: { discipline: "supplied" },
	dataset_revisions: ["synthetic-2026-07-30"],
	interval: {
		axis_zone: "UTC",
		start: "2026-08-01T00:00:00Z",
		end: "2026-08-02T00:00:00Z",
	},
	tzdb_version: "2026c",
	venues: [
		{
			id: "SYNTH-AUCT",
			display_name: "Synthetic Auction Exchange",
			family: "equities",
			home_zone: "Asia/Shanghai",
			location: "Shanghai",
			next_trading_transition: {
				at: "2026-08-02T01:25:00Z",
				kind: "opens",
				phase: "opening_auction",
			},
			next_trading_window: {
				start: "2026-08-02T01:25:00Z",
				end: "2026-08-02T03:30:00Z",
			},
			trading_windows: [],
			segments: [
				{
					start: "2026-08-01T00:00:00Z",
					end: "2026-08-02T00:00:00Z",
					position: {
						start_millionths: 0,
						end_millionths: 1_000_000,
					},
					status: "known",
					phase: "closed",
					trading: false,
					current: true,
					calendar: { kind: "weekly_pattern", label: "Weekend" },
					boundary_uncertainty: { start: "exact", end: "exact" },
				},
			],
		},
	],
};

describe("server timeline presentation", () => {
	it("keeps the requested market directory complete and identifier-safe", () => {
		const entries = marketCatalogGroups.flatMap((group) => group.entries);
		expect(entries).toHaveLength(52);
		expect(new Set(entries.map((entry) => entry.id)).size).toBe(entries.length);
		expect(marketCatalogGroups.map((group) => group.label)).toEqual([
			"股票市场",
			"国际现货 / 外汇",
			"大宗商品 / 期货",
			"数字资产",
		]);
		expect(entries.map((entry) => entry.id)).toEqual(
			expect.arrayContaining(["misx", "xhkg"]),
		);
		expect(entries.map((entry) => entry.id)).not.toEqual(
			expect.arrayContaining(["moex", "xhkx"]),
		);
	});

	it("maps regional-indicator flags to pinned Twemoji assets only", () => {
		expect(flagImageUrl("🇺🇸")).toBe(
			"https://cdnjs.cloudflare.com/ajax/libs/twemoji/14.0.2/svg/1f1fa-1f1f8.svg",
		);
		expect(flagImageUrl("■")).toBeNull();
		expect(flagImageUrl("🌐")).toBeNull();
	});

	it("filters the market directory without hiding the digital-asset family", () => {
		const result = filterMarketCatalog("digital", "coinbase");
		expect(result).toHaveLength(1);
		expect(result[0]?.id).toBe("digital");
		expect(result[0]?.entries.map((entry) => entry.id)).toEqual(["coinbase"]);
	});

	it("adds every service venue even when it is absent from the static directory", () => {
		const groups = catalogWithServerVenues([
			{
				id: "XTEST",
				display_name: "Test Exchange",
				family: "equities",
				home_zone: "Europe/London",
				location: "London",
			},
		]);
		const added = groups
			.flatMap((group) => group.entries)
			.find((entry) => entry.id === "xtest");

		expect(added).toMatchObject({
			label: "Test Exchange",
			detail: "London",
			timeZone: "Europe/London",
		});
	});

	it("keeps unknown distinct from a known closed venue", () => {
		const venues = [
			{ segments: [{ current: true, status: "known", trading: false }] },
			{ segments: [{ current: true, status: "known", trading: true }] },
			{ segments: [{ current: true, status: "unknown" }] },
		];

		expect(countSnapshot(venues)).toEqual({
			trading: 1,
			notTrading: 1,
			unknown: 1,
		});
		expect(currentSegment(venues[2])?.status).toBe("unknown");
	});

	it("puts active markets and imminent sessions at the front of the relay", () => {
		const venue = (
			id: string,
			phase: string,
			trading: boolean,
			kind: "opens" | "closes",
			at: string,
		) =>
			({
				...validPayload.venues[0],
				id,
				display_name: id,
				next_trading_transition: { at, kind, phase },
				segments: [
					{
						...validPayload.venues[0].segments[0],
						phase,
						trading,
					},
				],
			}) as VenueTimeline;
		const ranked = rankVenueAttention(
			[
				venue("xtks", "pre_open", false, "opens", "2026-08-01T12:30:00Z"),
				venue(
					"xnas",
					"continuous_trading",
					true,
					"closes",
					"2026-08-01T20:00:00Z",
				),
				venue(
					"xshg",
					"continuous_trading",
					true,
					"closes",
					"2026-08-01T13:00:00Z",
				),
			],
			validPayload.at,
		);

		expect(ranked.map(({ venue: item, kind }) => [item.id, kind])).toEqual([
			["xshg", "closing"],
			["xnas", "trading"],
			["xtks", "opening"],
		]);
	});

	it("schedules the earliest service transition without inventing a session", () => {
		const first = {
			...validPayload.venues[0],
			id: "XONE",
			display_name: "First Exchange",
			next_trading_transition: {
				at: "2026-08-01T12:30:00Z",
				kind: "opens" as const,
				phase: "continuous_trading",
			},
		} as VenueTimeline;
		const later = {
			...first,
			id: "XTWO",
			display_name: "Later Exchange",
			next_trading_transition: {
				...first.next_trading_transition,
				at: "2026-08-01T13:00:00Z",
			},
		} as VenueTimeline;

		expect(
			nextTransitionReminder([later, first], validPayload.at, 15),
		).toMatchObject({
			venueId: "XONE",
			kind: "opens",
			transitionAt: "2026-08-01T12:30:00Z",
			fireAt: "2026-08-01T12:15:00.000Z",
		});
		expect(
			nextTransitionReminder(
				[{ ...first, next_trading_transition: null }],
				validPayload.at,
				15,
			),
		).toBeNull();
	});

	it("scopes approaching and reached reminders to selected server venues", () => {
		const venue = (id: string, at: string) =>
			({
				...validPayload.venues[0],
				id,
				display_name: id,
				next_trading_transition: {
					at,
					kind: "opens" as const,
					phase: "continuous_trading",
				},
			}) as VenueTimeline;

		expect(
			transitionReminders(
				[
					venue("XONE", "2026-08-01T12:30:00Z"),
					venue("XTWO", "2026-08-01T13:00:00Z"),
				],
				validPayload.at,
				15,
				["xtwo"],
			),
		).toEqual([
			{
				key: "XTWO:opens:2026-08-01T13:00:00Z:approaching",
				venueId: "XTWO",
				venueName: "XTWO",
				kind: "opens",
				notification: "approaching",
				transitionAt: "2026-08-01T13:00:00Z",
				fireAt: "2026-08-01T12:45:00.000Z",
			},
			{
				key: "XTWO:opens:2026-08-01T13:00:00Z:reached",
				venueId: "XTWO",
				venueName: "XTWO",
				kind: "opens",
				notification: "reached",
				transitionAt: "2026-08-01T13:00:00Z",
				fireAt: "2026-08-01T13:00:00.000Z",
			},
		]);
		expect(
			transitionReminders(
				[venue("XONE", "2026-08-01T12:30:00Z")],
				validPayload.at,
				15,
				[],
			),
		).toEqual([]);
		expect(
			transitionReminders(
				[venue("XTWO", "2026-08-01T13:00:00Z")],
				validPayload.at,
				30,
			).map((reminder) => reminder.key),
		).toEqual([
			"XTWO:opens:2026-08-01T13:00:00Z:approaching",
			"XTWO:opens:2026-08-01T13:00:00Z:reached",
		]);
	});

	it("labels timezone fallbacks instead of presenting UTC as venue-local time", () => {
		const at = new Date("2026-08-01T12:00:00Z");
		expect(formatZonedTime(at, null)).toContain("UTC · 时区未知");
		expect(formatZonedTime(at, "Not/A_Zone")).toContain("UTC · 时区不可用");
	});

	it("anchors elapsed wall time to timeline receipt and expires old snapshots", () => {
		expect(
			serverAnchoredNow(validPayload.at, 1_000, 61_000, null).toISOString(),
		).toBe("2026-08-01T12:01:00.000Z");
		expect(
			serverAnchoredNow(validPayload.at, 1_000, 76_001, 76_000).toISOString(),
		).toBe("2026-08-01T12:01:15.000Z");
		expect(
			serverAnchoredNow(validPayload.at, 1_000, 90_000, 76_000).toISOString(),
		).toBe("2026-08-01T12:01:15.000Z");
		expect(staleSnapshotReason(null, true, 75_001, 75_000)).toBe(
			"快照超过 75 秒未更新",
		);
		expect(staleSnapshotReason(null, true, 75_000, 75_000)).toBeNull();
		expect(staleSnapshotReason(null, false, 100_000, 75_000)).toBeNull();
		expect(staleSnapshotReason("服务不可用", false, 0, 75_000)).toBe(
			"服务不可用",
		);
	});

	it("positions the cursor only inside the server-supplied interval", () => {
		const interval = {
			start: "2026-08-01T00:00:00Z",
			end: "2026-08-02T00:00:00Z",
		};

		expect(cursorPercent("2026-08-01T12:00:00Z", interval)).toBe(50);
		expect(cursorPercent("2026-08-02T00:00:00Z", interval)).toBeNull();
		expect(utcInstantAtPercent(0, interval)).toBe("2026-08-01T00:00:00.000Z");
		expect(utcInstantAtPercent(50, interval)).toBe("2026-08-01T12:00:00.000Z");
		expect(utcInstantAtPercent(100, interval)).toBe("2026-08-01T23:59:59.999Z");
	});

	it("does not relabel an accepted snapshot while another request is pending", () => {
		expect(
			viewMode({
				hasPayload: true,
				loading: true,
				stale: false,
				snapshotMode: "live",
			}),
		).toBe("REFRESHING");
		expect(
			viewMode({
				hasPayload: true,
				loading: true,
				stale: false,
				snapshotMode: "inspection",
			}),
		).toBe("REFRESHING");
		expect(
			viewMode({
				hasPayload: true,
				loading: false,
				stale: true,
				snapshotMode: "inspection",
			}),
		).toBe("STALE");
	});

	it("rejects malformed nested timeline data instead of guessing closed", () => {
		expect(isTimelinePayload(validPayload)).toBe(true);
		expect(isTimelinePayload({ ...validPayload, venues: [null] })).toBe(false);
		expect(
			isTimelinePayload({
				...validPayload,
				venues: [
					{
						...validPayload.venues[0],
						segments: [
							{
								...validPayload.venues[0].segments[0],
								status: "maybe",
							},
						],
					},
				],
			}),
		).toBe(false);
		expect(
			isTimelinePayload({
				...validPayload,
				venues: [
					{
						...validPayload.venues[0],
						segments: [
							{
								...validPayload.venues[0].segments[0],
								position: {
									start_millionths: -1,
									end_millionths: 1_000_001,
								},
							},
						],
					},
				],
			}),
		).toBe(false);
		expect(
			isTimelinePayload({
				...validPayload,
				venues: [
					{
						...validPayload.venues[0],
						next_trading_window: {
							start: "not-an-instant",
							end: "2026-08-02T03:30:00Z",
						},
					},
				],
			}),
		).toBe(false);
	});

	it("keeps map nodes pending, unknown, idle and trading", () => {
		expect(marketHubState([])).toBe("pending");
		expect(
			marketHubState([{ segments: [{ current: true, status: "unknown" }] }]),
		).toBe("unknown");
		expect(
			marketHubState([
				{
					segments: [{ current: true, status: "known", trading: false }],
				},
			]),
		).toBe("idle");
		expect(
			marketHubState([
				{ segments: [{ current: true, status: "known", trading: false }] },
				{ segments: [{ current: true, status: "known", trading: true }] },
			]),
		).toBe("trading");
	});

	it("validates and condenses runtime evidence for the source-intelligence view", () => {
		const statusPayload = {
			at: "2026-08-01T12:00:00Z",
			clock: { discipline: "supplied" },
			dataset_revisions: ["operator-2026-08-01"],
			tzdb_version: "2026c",
			venues: [
				{
					id: "XSHG",
					display_name: "Shanghai Stock Exchange",
					status: "known",
					phase: "closed",
					derived_reasoning: null,
					evidence: [
						{
							source_url: "https://example.test/rules",
							fetched_at: "2026-07-30T00:00:00Z",
							effective_from: "2026-01-01",
							publisher_last_changed: null,
						},
						{
							source_url: "https://example.test/rules",
							fetched_at: "2026-08-01T00:00:00Z",
							effective_from: "2026-07-01",
							publisher_last_changed: "2026-07-20T00:00:00Z",
						},
					],
				},
				{
					id: "XNYS",
					display_name: "New York Stock Exchange",
					status: "known",
					phase: "closed",
					derived_reasoning: null,
					evidence: [
						{
							source_url: "https://example.test/rules",
							fetched_at: "2026-07-31T00:00:00Z",
							effective_from: "2026-01-01",
							publisher_last_changed: null,
						},
					],
				},
			],
		};

		expect(isStatusPayload(statusPayload)).toBe(true);
		const matchingStatus = {
			...statusPayload,
			dataset_revisions: validPayload.dataset_revisions,
		};
		expect(
			statusMatchesTimeline(
				matchingStatus as StatusPayload,
				validPayload as TimelinePayload,
			),
		).toBe(true);
		expect(
			statusMatchesTimeline(
				{
					...matchingStatus,
					dataset_revisions: ["newer-rules"],
				} as StatusPayload,
				validPayload as TimelinePayload,
			),
		).toBe(false);
		expect(
			isStatusPayload({
				...statusPayload,
				venues: [
					{
						...statusPayload.venues[0],
						evidence: [
							{
								...statusPayload.venues[0]?.evidence[0],
								fetched_at: "yesterday",
							},
						],
					},
				],
			}),
		).toBe(false);
		expect(summarizeSources(statusPayload as StatusPayload)).toEqual([
			{
				effectiveFrom: "2026-01-01",
				fetchedAt: "2026-08-01T00:00:00Z",
				host: "example.test",
				publisherLastChanged: "2026-07-20T00:00:00Z",
				url: "https://example.test/rules",
				venues: ["New York Stock Exchange", "Shanghai Stock Exchange"],
			},
		]);
	});
});
