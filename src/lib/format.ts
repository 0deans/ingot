import i18n from "i18next"

/**
 * Dates, times, numbers and sizes in the language the user picked (not the OS locale),
 * so a page is never half in one language. Components re-render on a language change
 * through useTranslation, and these read the current language each call.
 */

const locale = () => i18n.language || "en"

type DateInput = number | string | Date

/** Unix seconds, milliseconds, ISO strings and Dates alike */
function toDate(value: DateInput): Date {
	if (value instanceof Date) return value
	if (typeof value === "number") return new Date(value < 1e12 ? value * 1000 : value)
	return new Date(value)
}

const valid = (d: Date) => !Number.isNaN(d.getTime())

/** "12 Mar 2026" */
export function formatDate(value: DateInput, options?: Intl.DateTimeFormatOptions): string {
	const d = toDate(value)
	if (!valid(d)) return ""
	return d.toLocaleDateString(
		locale(),
		options ?? { year: "numeric", month: "short", day: "numeric" },
	)
}

/** "12 Mar 2026, 14:05" */
export function formatDateTime(value: DateInput, options?: Intl.DateTimeFormatOptions): string {
	const d = toDate(value)
	if (!valid(d)) return ""
	return d.toLocaleString(
		locale(),
		options ?? {
			year: "numeric",
			month: "short",
			day: "numeric",
			hour: "2-digit",
			minute: "2-digit",
		},
	)
}

/** "14:05" */
export function formatTime(value: DateInput, withSeconds = false): string {
	const d = toDate(value)
	if (!valid(d)) return ""
	return d.toLocaleTimeString(locale(), {
		hour: "2-digit",
		minute: "2-digit",
		...(withSeconds ? { second: "2-digit" } : {}),
	})
}

/** "5 minutes ago", "yesterday", "in 2 days"; older than a week falls back to the date */
export function formatRelative(value: DateInput, now = Date.now()): string {
	const d = toDate(value)
	if (!valid(d)) return ""
	const seconds = Math.round((d.getTime() - now) / 1000)
	const abs = Math.abs(seconds)
	const rtf = new Intl.RelativeTimeFormat(locale(), { numeric: "auto" })
	if (abs < 45) return rtf.format(0, "second")
	if (abs < 3600) return rtf.format(Math.round(seconds / 60), "minute")
	if (abs < 86400) return rtf.format(Math.round(seconds / 3600), "hour")
	if (abs < 7 * 86400) return rtf.format(Math.round(seconds / 86400), "day")
	return formatDate(d)
}

type DurationFormatCtor = new (
	locale: string,
	options: { style: string },
) => { format: (d: Record<string, number>) => string }

/** "2h 5m", "45s" in the user's language */
export function formatDuration(totalSeconds: number, withSeconds = true): string {
	const s = Math.max(0, Math.floor(totalSeconds))
	const hours = Math.floor(s / 3600)
	const minutes = Math.floor((s % 3600) / 60)
	const seconds = s % 60
	const parts: Record<string, number> = {}
	if (hours) parts.hours = hours
	if (minutes || (hours && !withSeconds)) parts.minutes = minutes
	if ((withSeconds && !hours) || (!hours && !minutes)) parts.seconds = seconds
	const DurationFormat = (Intl as unknown as { DurationFormat?: DurationFormatCtor }).DurationFormat
	if (DurationFormat) {
		try {
			return new DurationFormat(locale(), { style: "narrow" }).format(parts)
		} catch {
			// Fall through to units one by one
		}
	}
	const unit = (n: number, u: string) =>
		new Intl.NumberFormat(locale(), { style: "unit", unit: u, unitDisplay: "narrow" }).format(n)
	return [
		parts.hours !== undefined && unit(parts.hours, "hour"),
		parts.minutes !== undefined && unit(parts.minutes, "minute"),
		parts.seconds !== undefined && unit(parts.seconds, "second"),
	]
		.filter(Boolean)
		.join(" ")
}

/** "1,234" / "1 234" */
export function formatNumber(n: number, options?: Intl.NumberFormatOptions): string {
	return new Intl.NumberFormat(locale(), options).format(n)
}

/** "1.2M", "12K" */
export function formatCount(n: number): string {
	return new Intl.NumberFormat(locale(), { notation: "compact", maximumFractionDigits: 1 }).format(
		n,
	)
}

/** "12%" from 12 (not 0.12) */
export function formatPercent(value: number, fractionDigits = 0): string {
	return new Intl.NumberFormat(locale(), {
		style: "percent",
		maximumFractionDigits: fractionDigits,
	}).format(value / 100)
}

const BYTE_UNITS = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte"] as const

/** "1.5 GB" with the unit written the way the language writes it */
export function formatBytes(bytes: number, fractionDigits = 1): string {
	let i = 0
	let value = Math.max(0, bytes)
	while (value >= 1024 && i < BYTE_UNITS.length - 1) {
		value /= 1024
		i++
	}
	return new Intl.NumberFormat(locale(), {
		style: "unit",
		unit: BYTE_UNITS[i],
		unitDisplay: "short",
		maximumFractionDigits: i <= 1 ? 0 : fractionDigits,
	}).format(value)
}

/** RAM given in megabytes: "512 MB", "4 GB", "4.5 GB" */
export function formatMegabytes(mb: number, fractionDigits = 1): string {
	return formatBytes(mb * 1024 * 1024, fractionDigits)
}
