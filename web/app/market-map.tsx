"use client";

import dynamic from "next/dynamic";
import { useEffect, useRef, useState } from "react";
import type { MarketHubState } from "../lib/market-time";

const WorldMap = dynamic(() => import("@react-map/world"), { ssr: false });

export interface MarketMapHub {
	id: string;
	label: string;
	code: string;
	zone: string;
	longitude: number;
	latitude: number;
	marketIds: readonly string[];
	state: MarketHubState;
	connected: number;
}

const hubStateLabels: Record<MarketHubState, string> = {
	trading: "交易中",
	idle: "已知休市",
	unknown: "状态未知",
	pending: "等待规则",
};

export function MarketMap({
	hubs,
	onSelectZone,
}: {
	hubs: MarketMapHub[];
	onSelectZone: (zone: string) => void;
}) {
	const mapRef = useRef<HTMLDivElement>(null);
	const [mapWidth, setMapWidth] = useState(544);

	useEffect(() => {
		const node = mapRef.current;
		if (!node) return;
		const update = () =>
			setMapWidth(Math.max(240, Math.floor(node.clientWidth)));
		update();
		const observer = new ResizeObserver(update);
		observer.observe(node);
		return () => observer.disconnect();
	}, []);

	return (
		<div className="market-map">
			<div className="map-canvas">
				<div className="map-base" ref={mapRef}>
					<div className="map-vector" aria-hidden="true">
						<WorldMap
							disableClick
							disableHover
							mapColor="var(--map-land)"
							size={mapWidth}
							strokeColor="var(--map-stroke)"
							strokeWidth={0.45}
							type="select-single"
						/>
					</div>
					{hubs.map((hub) => (
						<button
							aria-label={`${hub.label}，${hubStateLabels[hub.state]}，规则 ${hub.connected}/${hub.marketIds.length}`}
							className="market-hub"
							data-code={hub.code}
							data-state={hub.state}
							key={hub.id}
							onClick={() => onSelectZone(hub.zone)}
							style={{
								left: `${6 + ((hub.longitude + 180) / 360) * 88}%`,
								top: `${Math.min(90, Math.max(8, 68 - hub.latitude * 0.53))}%`,
							}}
							type="button"
						>
							<i aria-hidden="true" />
							<span>{hub.code}</span>
						</button>
					))}
				</div>
			</div>

			<ul className="map-legend" aria-label="地图状态图例">
				<li data-state="trading">交易中</li>
				<li data-state="idle">已知休市</li>
				<li data-state="unknown">Unknown</li>
				<li data-state="pending">等待规则</li>
			</ul>
		</div>
	);
}
