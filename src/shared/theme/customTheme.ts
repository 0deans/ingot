import { z } from "zod"

export type AppThemeId = "light" | "dark" | "purple" | "custom"
export type CustomThemeBaseMode = "dark" | "light"

export interface CustomThemeColors {
	background: string
	foreground: string
	card: string
	cardForeground: string
	popover: string
	popoverForeground: string
	sidebar: string
	sidebarForeground: string
	primary: string
	primaryForeground: string
	muted: string
	mutedForeground: string
	border: string
	destructive: string
}

export interface CustomThemeConfig {
	version: 1
	baseMode: CustomThemeBaseMode
	radiusRem: number
	colors: CustomThemeColors
}

export interface CustomThemeFieldDescriptor {
	key: keyof CustomThemeColors
	label: string
	token: string
}

export const CUSTOM_THEME_FIELDS: readonly CustomThemeFieldDescriptor[] = [
	{ key: "background", label: "App Background", token: "--background" },
	{ key: "foreground", label: "Primary Text", token: "--foreground" },
	{ key: "card", label: "Card Surface", token: "--card" },
	{ key: "cardForeground", label: "Card Text", token: "--card-foreground" },
	{ key: "sidebar", label: "Sidebar Surface", token: "--sidebar" },
	{ key: "sidebarForeground", label: "Sidebar Text", token: "--sidebar-foreground" },
	{ key: "popover", label: "Popover Surface", token: "--popover" },
	{ key: "popoverForeground", label: "Popover Text", token: "--popover-foreground" },
	{ key: "primary", label: "Primary Accent", token: "--primary" },
	{ key: "primaryForeground", label: "Accent Text", token: "--primary-foreground" },
	{ key: "muted", label: "Muted Surface", token: "--muted" },
	{ key: "mutedForeground", label: "Muted Text", token: "--muted-foreground" },
	{ key: "border", label: "Border Color", token: "--border" },
	{ key: "destructive", label: "Danger / Destructive", token: "--destructive" },
]

export const THEME_PRESETS: Record<Exclude<AppThemeId, "custom">, CustomThemeConfig> = {
	dark: {
		version: 1,
		baseMode: "dark",
		radiusRem: 0.75,
		colors: {
			background: "#09090b",
			foreground: "#f4f4f5",
			card: "#141418",
			cardForeground: "#f4f4f5",
			popover: "#18181f",
			popoverForeground: "#f4f4f5",
			sidebar: "#101014",
			sidebarForeground: "#f4f4f5",
			primary: "#10b981",
			primaryForeground: "#052e16",
			muted: "#202026",
			mutedForeground: "#a1a1aa",
			border: "#27272a",
			destructive: "#ef4444",
		},
	},
	light: {
		version: 1,
		baseMode: "light",
		radiusRem: 0.75,
		colors: {
			background: "#f4f6f9",
			foreground: "#0f172a",
			card: "#ffffff",
			cardForeground: "#0f172a",
			popover: "#ffffff",
			popoverForeground: "#0f172a",
			sidebar: "#eef1f6",
			sidebarForeground: "#0f172a",
			primary: "#059669",
			primaryForeground: "#ffffff",
			muted: "#e5e9f0",
			mutedForeground: "#475569",
			border: "#d8dee8",
			destructive: "#dc2626",
		},
	},
	purple: {
		version: 1,
		baseMode: "dark",
		radiusRem: 0.75,
		colors: {
			background: "#0d0917",
			foreground: "#f3efff",
			card: "#171127",
			cardForeground: "#f3efff",
			popover: "#1c1530",
			popoverForeground: "#f3efff",
			sidebar: "#130d21",
			sidebarForeground: "#f3efff",
			primary: "#a855f7",
			primaryForeground: "#faf5ff",
			muted: "#241b3b",
			mutedForeground: "#b2a5d6",
			border: "#342754",
			destructive: "#f43f5e",
		},
	},
}

export const DEFAULT_CUSTOM_THEME: CustomThemeConfig = THEME_PRESETS.dark

const HEX_COLOR_REGEX = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/
const NUMBER_REGEX = /^[+-]?(?:\d+\.?\d*|\.\d+)$/
const PERCENT_REGEX = /^([+-]?(?:\d+\.?\d*|\.\d+))%$/
const HUE_REGEX = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:deg|rad|grad|turn)?$/i

const parseFiniteNumber = (token: string): number | null => {
	if (!NUMBER_REGEX.test(token)) {
		return null
	}
	const num = Number(token)
	return Number.isFinite(num) ? num : null
}

const parsePercentValue = (token: string): number | null => {
	const match = PERCENT_REGEX.exec(token)
	if (!match?.[1]) {
		return null
	}
	const num = Number(match[1])
	return Number.isFinite(num) ? num : null
}

const isValidAlpha = (token: string): boolean => {
	if (token.toLowerCase() === "none") {
		return true
	}
	const num = parseFiniteNumber(token)
	if (num !== null) {
		return num >= 0 && num <= 1
	}
	const pct = parsePercentValue(token)
	return pct !== null && pct >= 0 && pct <= 100
}

const isValidRgbChannel = (token: string): boolean => {
	if (token.toLowerCase() === "none") {
		return true
	}
	const num = parseFiniteNumber(token)
	if (num !== null) {
		return num >= 0 && num <= 255
	}
	const pct = parsePercentValue(token)
	return pct !== null && pct >= 0 && pct <= 100
}

const isValidPercentInRange = (token: string, min: number, max: number): boolean => {
	if (token.toLowerCase() === "none") {
		return true
	}
	const pct = parsePercentValue(token)
	return pct !== null && pct >= min && pct <= max
}

const isValidHue = (token: string): boolean => {
	return token.toLowerCase() === "none" || HUE_REGEX.test(token)
}

const isValidOklchLightness = (token: string): boolean => {
	if (token.toLowerCase() === "none") {
		return true
	}
	const num = parseFiniteNumber(token)
	if (num !== null) {
		return num >= 0 && num <= 1
	}
	const pct = parsePercentValue(token)
	return pct !== null && pct >= 0 && pct <= 100
}

const isValidOklchChroma = (token: string): boolean => {
	if (token.toLowerCase() === "none") {
		return true
	}
	const num = parseFiniteNumber(token)
	if (num !== null) {
		return num >= 0
	}
	const pct = parsePercentValue(token)
	return pct !== null && pct >= 0
}

const isValidSignedNumberOrPercent = (token: string): boolean => {
	if (token.toLowerCase() === "none") {
		return true
	}
	return parseFiniteNumber(token) !== null || parsePercentValue(token) !== null
}

const splitColorArguments = (
	rawArgs: string,
	allowLegacyComma: boolean,
): { channels: string[]; alpha: string | null } | null => {
	const trimmedArgs = rawArgs.trim()
	if (!trimmedArgs) {
		return null
	}

	if (trimmedArgs.includes(",")) {
		if (!allowLegacyComma || trimmedArgs.includes("/")) {
			return null
		}
		const parts = trimmedArgs.split(",").map((part) => {
			return part.trim()
		})
		if (
			parts.some((part) => {
				return !part
			})
		) {
			return null
		}
		if (parts.length === 3) {
			return { channels: parts, alpha: null }
		}
		if (parts.length === 4) {
			return { channels: parts.slice(0, 3), alpha: parts[3] }
		}
		return null
	}

	const slashParts = trimmedArgs.split("/")
	if (slashParts.length > 2) {
		return null
	}

	const channels = slashParts[0].trim().split(/\s+/).filter(Boolean)
	if (channels.length !== 3) {
		return null
	}

	if (slashParts.length === 2) {
		const alpha = slashParts[1].trim()
		if (!alpha || /\s/.test(alpha)) {
			return null
		}
		return { channels, alpha }
	}

	return { channels, alpha: null }
}

export type ValidateCssColorResult = { ok: true; normalized: string } | { ok: false; error: string }

const validateCssColorSyntax = (trimmed: string): string | null => {
	if (trimmed.startsWith("#")) {
		if (!HEX_COLOR_REGEX.test(trimmed)) {
			return "Use a valid hex color (#RGB, #RGBA, #RRGGBB, or #RRGGBBAA)."
		}
		return null
	}

	const fnMatch = /^(rgb|rgba|hsl|hsla|oklch|oklab)\((.+)\)$/i.exec(trimmed)
	if (!fnMatch?.[1] || !fnMatch[2]) {
		return "Use hex (#rrggbb) or valid rgb(), hsl(), oklch(), or oklab() color syntax."
	}

	const fnName = fnMatch[1].toLowerCase()
	const isLegacyFn = fnName === "rgb" || fnName === "rgba" || fnName === "hsl" || fnName === "hsla"
	const parsedArgs = splitColorArguments(fnMatch[2], isLegacyFn)
	if (!parsedArgs) {
		return `Invalid ${fnName}() syntax. Check channels and optional alpha.`
	}

	const { channels, alpha } = parsedArgs
	if (alpha !== null && !isValidAlpha(alpha)) {
		return `Invalid alpha value "${alpha}" in ${fnName}(). Use 0–1 or 0%–100%.`
	}

	const [c1, c2, c3] = channels
	let channelsValid = false
	if (fnName === "rgb" || fnName === "rgba") {
		channelsValid = isValidRgbChannel(c1) && isValidRgbChannel(c2) && isValidRgbChannel(c3)
	} else if (fnName === "hsl" || fnName === "hsla") {
		channelsValid =
			isValidHue(c1) && isValidPercentInRange(c2, 0, 100) && isValidPercentInRange(c3, 0, 100)
	} else if (fnName === "oklch") {
		channelsValid = isValidOklchLightness(c1) && isValidOklchChroma(c2) && isValidHue(c3)
	} else if (fnName === "oklab") {
		channelsValid =
			isValidOklchLightness(c1) &&
			isValidSignedNumberOrPercent(c2) &&
			isValidSignedNumberOrPercent(c3)
	}

	if (!channelsValid) {
		return `Invalid ${fnName}() channel values or out-of-range components.`
	}

	return null
}

export const cssColorSchema = z
	.string()
	.trim()
	.min(1, "Color value is required.")
	.max(64, "Color value must be 64 characters or fewer.")
	.superRefine((value, ctx) => {
		const syntaxError = validateCssColorSyntax(value)
		if (syntaxError !== null) {
			ctx.addIssue({
				code: "custom",
				message: syntaxError,
			})
		}
	})

export const validateCssColor = (value: string): ValidateCssColorResult => {
	const parsed = cssColorSchema.safeParse(value)
	if (!parsed.success) {
		const firstIssue = parsed.error.issues[0]
		return {
			ok: false,
			error: firstIssue ? firstIssue.message : "Invalid CSS color value.",
		}
	}

	return { ok: true, normalized: parsed.data }
}

export const isValidCssColor = (value: string): boolean => {
	return cssColorSchema.safeParse(value).success
}

export const appThemeIdSchema = z.enum(["light", "dark", "purple", "custom"])
export const customThemeBaseModeSchema = z.enum(["dark", "light"])
export const customThemeRadiusSchema = z.number().min(0).max(1.5)

const customThemeColorsSchema = z.object({
	background: cssColorSchema,
	foreground: cssColorSchema,
	card: cssColorSchema,
	cardForeground: cssColorSchema,
	popover: cssColorSchema,
	popoverForeground: cssColorSchema,
	sidebar: cssColorSchema,
	sidebarForeground: cssColorSchema,
	primary: cssColorSchema,
	primaryForeground: cssColorSchema,
	muted: cssColorSchema,
	mutedForeground: cssColorSchema,
	border: cssColorSchema,
	destructive: cssColorSchema,
})

export const customThemeConfigSchema = z.object({
	version: z.literal(1).default(1),
	baseMode: customThemeBaseModeSchema.default("dark"),
	radiusRem: customThemeRadiusSchema.default(0.75),
	colors: customThemeColorsSchema,
})

export type ParseCustomThemeResult =
	| { ok: true; value: CustomThemeConfig }
	| { ok: false; errors: string[] }

export const isAppThemeId = (value: string | undefined): value is AppThemeId => {
	return appThemeIdSchema.safeParse(value).success
}

export const parseCustomThemeJson = (rawJson: string): ParseCustomThemeResult => {
	const trimmed = rawJson.trim()
	if (!trimmed) {
		return { ok: false, errors: ["Theme JSON cannot be empty."] }
	}

	let parsedUnknown: unknown
	try {
		parsedUnknown = JSON.parse(trimmed)
	} catch (error: unknown) {
		const message = error instanceof Error ? error.message : "Invalid JSON syntax."
		return { ok: false, errors: [`Invalid JSON: ${message}`] }
	}

	const validation = customThemeConfigSchema.safeParse(parsedUnknown)
	if (!validation.success) {
		const errors = validation.error.issues.map((issue) => {
			const path = issue.path.length > 0 ? issue.path.join(".") : "root"
			return `${path}: ${issue.message}`
		})
		return { ok: false, errors }
	}

	return { ok: true, value: validation.data }
}

export const stringifyCustomThemeJson = (config: CustomThemeConfig): string => {
	return JSON.stringify(config, null, 2)
}

export const toColorPickerHex = (value: string, fallback = "#000000"): string => {
	const trimmed = value.trim()
	if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) {
		return trimmed.toLowerCase()
	}
	if (/^#[0-9a-fA-F]{3}$/.test(trimmed)) {
		const r = trimmed[1]
		const g = trimmed[2]
		const b = trimmed[3]
		return `#${r}${r}${g}${g}${b}${b}`.toLowerCase()
	}
	return fallback
}
