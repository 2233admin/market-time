export const PRIMARY_ZONE_KEY = "mark-time-primary-zone";
export const REMINDER_PREFERENCES_KEY = "mark-time-reminders";
export const LAST_REMINDER_KEY = "mark-time-last-reminder";
export const WIDGET_NOTIFIED_REMINDERS_KEY =
	"mark-time-widget-notified-reminders";

export const primaryTimeZones = [
	{ id: "America/New_York", label: "纽约", code: "NYC" },
	{ id: "Europe/London", label: "伦敦", code: "LON" },
	{ id: "Asia/Dubai", label: "迪拜", code: "DXB" },
	{ id: "Asia/Shanghai", label: "北京", code: "BJS" },
	{ id: "Asia/Tokyo", label: "东京", code: "TYO" },
] as const;

export interface ReminderPreferences {
	enabled: boolean;
	leadMinutes: number;
	venueIds?: string[];
}

export const defaultReminderPreferences: ReminderPreferences = {
	enabled: false,
	leadMinutes: 15,
};

export const reminderLeadOptions = [0, 5, 15, 30, 60] as const;
