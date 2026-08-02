"use client";

import { Button } from "@appica/ui-react/button";
import { useEffect, useState } from "react";

type Theme = "dark" | "light";

export function ThemeToggle() {
	const [theme, setTheme] = useState<Theme>("dark");

	useEffect(() => {
		setTheme(
			document.documentElement.dataset.theme === "light" ? "light" : "dark",
		);
	}, []);

	function toggleTheme() {
		const next = theme === "dark" ? "light" : "dark";
		document.documentElement.dataset.theme = next;
		try {
			localStorage.setItem("mark-time-theme", next);
		} catch {
			// The theme still changes for this page when storage is unavailable.
		}
		setTheme(next);
	}

	return (
		<Button
			className="theme-toggle"
			size="sm"
			variant="outline"
			onClick={toggleTheme}
			aria-label={theme === "dark" ? "切换到亮色模式" : "切换到暗色模式"}
			title={theme === "dark" ? "切换到亮色模式" : "切换到暗色模式"}
		>
			<span aria-hidden="true">{theme === "dark" ? "☼" : "◐"}</span>
			{theme === "dark" ? "切换亮色" : "切换暗色"}
		</Button>
	);
}
