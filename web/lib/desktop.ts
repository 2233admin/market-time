export type SystemNotificationPermission =
	| NotificationPermission
	| "unsupported";

export function isDesktopRuntime() {
	return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function hideDesktopWidget() {
	if (!isDesktopRuntime()) return false;
	const { getCurrentWindow } = await import("@tauri-apps/api/window");
	await getCurrentWindow().hide();
	return true;
}

export async function notificationPermission(): Promise<SystemNotificationPermission> {
	if (isDesktopRuntime()) {
		const { isPermissionGranted } = await import(
			"@tauri-apps/plugin-notification"
		);
		return (await isPermissionGranted()) ? "granted" : "default";
	}
	return "Notification" in window ? Notification.permission : "unsupported";
}

export async function requestNotificationPermission(): Promise<SystemNotificationPermission> {
	if (isDesktopRuntime()) {
		const { isPermissionGranted, requestPermission } = await import(
			"@tauri-apps/plugin-notification"
		);
		return (await isPermissionGranted()) ? "granted" : requestPermission();
	}
	return "Notification" in window
		? Notification.requestPermission()
		: "unsupported";
}

export async function sendSystemNotification(title: string, body: string) {
	if (isDesktopRuntime()) {
		const { sendNotification } = await import(
			"@tauri-apps/plugin-notification"
		);
		sendNotification({ title, body });
		return;
	}
	new Notification(title, { body });
}
