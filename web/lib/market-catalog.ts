export interface MarketCatalogEntry {
	id: string;
	label: string;
	detail: string;
	timeZone: string;
	symbol: string;
	accent: string;
	symbolKind?: "asset";
}

export interface MarketCatalogGroup {
	id: string;
	label: string;
	entries: MarketCatalogEntry[];
}

export interface ServerVenueCatalogEntry {
	id: string;
	display_name: string;
	family: string | null;
	home_zone: string | null;
	location: string | null;
}

const TWEMOJI_FLAG_BASE =
	"https://cdnjs.cloudflare.com/ajax/libs/twemoji/14.0.2/svg";

export function flagImageUrl(symbol: string) {
	const codepoints = Array.from(symbol, (character) =>
		character.codePointAt(0),
	);
	if (
		codepoints.length !== 2 ||
		codepoints.some(
			(codepoint) =>
				codepoint === undefined || codepoint < 0x1f1e6 || codepoint > 0x1f1ff,
		)
	)
		return null;

	return `${TWEMOJI_FLAG_BASE}/${codepoints
		.map((codepoint) => codepoint?.toString(16))
		.join("-")}.svg`;
}

export const marketCatalogGroups: MarketCatalogGroup[] = [
	{
		id: "equities",
		label: "股票市场",
		entries: [
			{
				id: "xnys",
				label: "纽约证券交易所",
				detail: "纽约",
				timeZone: "America/New_York",
				symbol: "🇺🇸",
				accent: "#3978ff",
			},
			{
				id: "xnas",
				label: "纳斯达克交易所",
				detail: "纽约",
				timeZone: "America/New_York",
				symbol: "🇺🇸",
				accent: "#3978ff",
			},
			{
				id: "xtse",
				label: "多伦多证券交易所",
				detail: "多伦多",
				timeZone: "America/Toronto",
				symbol: "🇨🇦",
				accent: "#ff5565",
			},
			{
				id: "bvmf",
				label: "巴西 B3 交易所",
				detail: "圣保罗",
				timeZone: "America/Sao_Paulo",
				symbol: "🇧🇷",
				accent: "#46c982",
			},
			{
				id: "xlse",
				label: "伦敦证券交易所",
				detail: "伦敦",
				timeZone: "Europe/London",
				symbol: "🇬🇧",
				accent: "#41d2d0",
			},
			{
				id: "xpar",
				label: "泛欧交易所",
				detail: "巴黎 / 阿姆斯特丹",
				timeZone: "Europe/Paris",
				symbol: "🇪🇺",
				accent: "#31aec9",
			},
			{
				id: "xfra",
				label: "法兰克福交易所",
				detail: "法兰克福",
				timeZone: "Europe/Berlin",
				symbol: "🇩🇪",
				accent: "#29b6d5",
			},
			{
				id: "xswx",
				label: "瑞士证券交易所",
				detail: "苏黎世",
				timeZone: "Europe/Zurich",
				symbol: "🇨🇭",
				accent: "#26b9a9",
			},
			{
				id: "misx",
				label: "莫斯科交易所",
				detail: "莫斯科",
				timeZone: "Europe/Moscow",
				symbol: "🇷🇺",
				accent: "#27b9c6",
			},
			{
				id: "xjse",
				label: "约翰内斯堡交易所",
				detail: "约翰内斯堡",
				timeZone: "Africa/Johannesburg",
				symbol: "🇿🇦",
				accent: "#50c772",
			},
			{
				id: "dfm",
				label: "迪拜金融市场",
				detail: "迪拜",
				timeZone: "Asia/Dubai",
				symbol: "🇦🇪",
				accent: "#e96f32",
			},
			{
				id: "xtad",
				label: "沙特证券交易所",
				detail: "利雅得",
				timeZone: "Asia/Riyadh",
				symbol: "🇸🇦",
				accent: "#c58026",
			},
			{
				id: "xnse",
				label: "印度国家证券交易所",
				detail: "孟买",
				timeZone: "Asia/Kolkata",
				symbol: "🇮🇳",
				accent: "#9a58f5",
			},
			{
				id: "xshg",
				label: "上海证券交易所",
				detail: "上海",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff394e",
			},
			{
				id: "xshe",
				label: "深圳证券交易所",
				detail: "深圳",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff3868",
			},
			{
				id: "xhkg",
				label: "中国香港交易所",
				detail: "香港",
				timeZone: "Asia/Hong_Kong",
				symbol: "🇭🇰",
				accent: "#ff4e32",
			},
			{
				id: "xses",
				label: "新加坡交易所",
				detail: "新加坡",
				timeZone: "Asia/Singapore",
				symbol: "🇸🇬",
				accent: "#25b9c9",
			},
			{
				id: "xtks",
				label: "东京证券交易所",
				detail: "东京",
				timeZone: "Asia/Tokyo",
				symbol: "🇯🇵",
				accent: "#ed3472",
			},
			{
				id: "xkrx",
				label: "韩国交易所",
				detail: "首尔",
				timeZone: "Asia/Seoul",
				symbol: "🇰🇷",
				accent: "#5359df",
			},
			{
				id: "xasx",
				label: "澳大利亚证券交易所",
				detail: "悉尼",
				timeZone: "Australia/Sydney",
				symbol: "🇦🇺",
				accent: "#29b4b2",
			},
			{
				id: "xnze",
				label: "新西兰交易所",
				detail: "惠灵顿",
				timeZone: "Pacific/Auckland",
				symbol: "🇳🇿",
				accent: "#27bda9",
			},
		],
	},
	{
		id: "spot",
		label: "国际现货 / 外汇",
		entries: [
			{
				id: "xau",
				label: "现货黄金",
				detail: "全球",
				timeZone: "UTC",
				symbol: "■",
				symbolKind: "asset",
				accent: "#c79a37",
			},
			{
				id: "xag",
				label: "现货白银",
				detail: "全球",
				timeZone: "UTC",
				symbol: "■",
				symbolKind: "asset",
				accent: "#aab3c2",
			},
			{
				id: "wti-spot",
				label: "WTI 原油",
				detail: "全球",
				timeZone: "UTC",
				symbol: "■",
				symbolKind: "asset",
				accent: "#45423d",
			},
			{
				id: "copper-spot",
				label: "现货铜",
				detail: "全球",
				timeZone: "UTC",
				symbol: "■",
				symbolKind: "asset",
				accent: "#c56847",
			},
			{
				id: "xpt",
				label: "现货铂金",
				detail: "全球",
				timeZone: "UTC",
				symbol: "■",
				symbolKind: "asset",
				accent: "#d8dee8",
			},
			{
				id: "fx",
				label: "外汇",
				detail: "全球",
				timeZone: "UTC",
				symbol: "■",
				symbolKind: "asset",
				accent: "#2ab889",
			},
		],
	},
	{
		id: "futures",
		label: "大宗商品 / 期货",
		entries: [
			{
				id: "comex-gold",
				label: "COMEX 黄金",
				detail: "CME Group",
				timeZone: "America/Chicago",
				symbol: "🇺🇸",
				accent: "#d09a35",
			},
			{
				id: "comex-silver",
				label: "COMEX 白银",
				detail: "CME Group",
				timeZone: "America/Chicago",
				symbol: "🇺🇸",
				accent: "#aeb7c5",
			},
			{
				id: "comex-copper",
				label: "COMEX 铜",
				detail: "CME Group",
				timeZone: "America/Chicago",
				symbol: "🇺🇸",
				accent: "#d26034",
			},
			{
				id: "nymex-wti",
				label: "NYMEX WTI 原油",
				detail: "CME Group",
				timeZone: "America/Chicago",
				symbol: "🇺🇸",
				accent: "#55514c",
			},
			{
				id: "nymex-gas",
				label: "NYMEX 天然气",
				detail: "CME Group",
				timeZone: "America/Chicago",
				symbol: "🇺🇸",
				accent: "#747c8a",
			},
			{
				id: "ice-brent",
				label: "ICE 布伦特原油",
				detail: "伦敦",
				timeZone: "Europe/London",
				symbol: "🇬🇧",
				accent: "#db5c30",
			},
			{
				id: "cbot-grain",
				label: "CBOT 谷物（玉米/小麦/大豆）",
				detail: "CME Group",
				timeZone: "America/Chicago",
				symbol: "🇺🇸",
				accent: "#66bd45",
			},
			{
				id: "lme",
				label: "伦敦金属交易所",
				detail: "伦敦",
				timeZone: "Europe/London",
				symbol: "🇬🇧",
				accent: "#8da0b8",
			},
			{
				id: "shfe",
				label: "上海期货交易所",
				detail: "上海",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff394e",
			},
			{
				id: "dce",
				label: "大连商品交易所",
				detail: "大连",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff3868",
			},
			{
				id: "czce",
				label: "郑州商品交易所",
				detail: "郑州",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff387c",
			},
			{
				id: "ine",
				label: "上海国际能源交易中心",
				detail: "上海",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff5c2c",
			},
			{
				id: "gfex",
				label: "广州期货交易所",
				detail: "广州",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#ff2988",
			},
			{
				id: "cffex",
				label: "中国金融期货交易所",
				detail: "上海",
				timeZone: "Asia/Shanghai",
				symbol: "🇨🇳",
				accent: "#9154ee",
			},
			{
				id: "dgcx",
				label: "迪拜黄金商品交易所",
				detail: "迪拜",
				timeZone: "Asia/Dubai",
				symbol: "🇦🇪",
				accent: "#df6733",
			},
			{
				id: "mcx",
				label: "印度多种商品交易所",
				detail: "孟买",
				timeZone: "Asia/Kolkata",
				symbol: "🇮🇳",
				accent: "#8752df",
			},
			{
				id: "tocom",
				label: "东京商品交易所",
				detail: "东京",
				timeZone: "Asia/Tokyo",
				symbol: "🇯🇵",
				accent: "#d93073",
			},
		],
	},
	{
		id: "digital",
		label: "数字资产",
		entries: [
			{
				id: "binance",
				label: "Binance",
				detail: "全球 · 现货 / 衍生品",
				timeZone: "UTC",
				symbol: "◆",
				symbolKind: "asset",
				accent: "#e5ad18",
			},
			{
				id: "coinbase",
				label: "Coinbase Exchange",
				detail: "全球 · 现货",
				timeZone: "UTC",
				symbol: "●",
				symbolKind: "asset",
				accent: "#315dff",
			},
			{
				id: "okx",
				label: "OKX",
				detail: "全球 · 现货 / 衍生品",
				timeZone: "UTC",
				symbol: "✣",
				symbolKind: "asset",
				accent: "#111720",
			},
			{
				id: "bybit",
				label: "Bybit",
				detail: "全球 · 现货 / 衍生品",
				timeZone: "UTC",
				symbol: "▲",
				symbolKind: "asset",
				accent: "#ff9c2f",
			},
			{
				id: "kraken",
				label: "Kraken",
				detail: "全球 · 现货 / 衍生品",
				timeZone: "UTC",
				symbol: "◒",
				symbolKind: "asset",
				accent: "#6d52d9",
			},
			{
				id: "deribit",
				label: "Deribit",
				detail: "全球 · 数字资产衍生品",
				timeZone: "UTC",
				symbol: "D",
				symbolKind: "asset",
				accent: "#119b72",
			},
			{
				id: "bitget",
				label: "Bitget",
				detail: "全球 · 现货 / 衍生品",
				timeZone: "UTC",
				symbol: "↗",
				symbolKind: "asset",
				accent: "#18b9c8",
			},
			{
				id: "crypto-com",
				label: "Crypto.com Exchange",
				detail: "全球 · 现货 / 衍生品",
				timeZone: "UTC",
				symbol: "⬡",
				symbolKind: "asset",
				accent: "#1746a2",
			},
		],
	},
];

export function catalogWithServerVenues(
	venues: ServerVenueCatalogEntry[],
): MarketCatalogGroup[] {
	const groups = marketCatalogGroups.map((group) => ({
		...group,
		entries: [...group.entries],
	}));
	const known = new Set(
		groups.flatMap((group) => group.entries.map((entry) => entry.id)),
	);
	const familyGroups: Record<string, string> = {
		equities: "equities",
		spot_and_fx: "spot",
		futures: "futures",
	};
	const uncategorized: MarketCatalogEntry[] = [];

	for (const venue of venues) {
		const id = venue.id.toLowerCase();
		if (known.has(id)) continue;
		const entry = {
			id,
			label: venue.display_name || venue.id,
			detail: venue.location || "服务端市场",
			timeZone: venue.home_zone || "UTC",
			symbol: "◎",
			symbolKind: "asset" as const,
			accent: "#7f8da3",
		};
		const target = venue.family ? familyGroups[venue.family] : undefined;
		const group = groups.find((candidate) => candidate.id === target);
		if (group) group.entries.push(entry);
		else uncategorized.push(entry);
		known.add(id);
	}

	if (uncategorized.length)
		groups.push({
			id: "connected",
			label: "其他已接入市场",
			entries: uncategorized,
		});
	return groups;
}

export function filterMarketCatalog(
	groupId: string,
	query: string,
	groups = marketCatalogGroups,
) {
	const normalizedQuery = query.trim().toLocaleLowerCase("zh-CN");

	return groups
		.filter((group) => groupId === "all" || group.id === groupId)
		.map((group) => ({
			...group,
			entries: normalizedQuery
				? group.entries.filter((entry) =>
						[entry.label, entry.detail, entry.id].some((value) =>
							value.toLocaleLowerCase("zh-CN").includes(normalizedQuery),
						),
					)
				: group.entries,
		}))
		.filter((group) => group.entries.length > 0);
}
