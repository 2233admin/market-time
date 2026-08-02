import type { Metadata, Viewport } from "next";
import Script from "next/script";
import type { ReactNode } from "react";

import "./globals.css";

export const metadata: Metadata = {
	title: "MARK / TIME — 全球交易时间表",
	description:
		"用同一个绝对时刻查看全球交易场所的当地时间、交易状态与下一次开收盘。",
};

export const viewport: Viewport = {
	themeColor: [
		{ media: "(prefers-color-scheme: dark)", color: "#0b1018" },
		{ media: "(prefers-color-scheme: light)", color: "#eef1f4" },
	],
	colorScheme: "dark light",
};

export default function RootLayout({
	children,
}: Readonly<{ children: ReactNode }>) {
	return (
		<html lang="zh-CN" data-theme="dark" suppressHydrationWarning>
			<head>
				<Script id="theme-bootstrap" strategy="beforeInteractive">
					{
						'try{document.documentElement.dataset.theme=localStorage.getItem("mark-time-theme")==="light"?"light":"dark"}catch{document.documentElement.dataset.theme="dark"}'
					}
				</Script>
			</head>
			<body>{children}</body>
		</html>
	);
}
