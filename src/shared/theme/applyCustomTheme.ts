import { type CustomThemeConfig } from "@/shared/theme/customTheme"

const CUSTOM_CSS_VARIABLES = [
	"--radius",
	"--background",
	"--foreground",
	"--card",
	"--card-foreground",
	"--popover",
	"--popover-foreground",
	"--primary",
	"--primary-foreground",
	"--secondary",
	"--secondary-foreground",
	"--muted",
	"--muted-foreground",
	"--accent",
	"--accent-foreground",
	"--destructive",
	"--border",
	"--input",
	"--ring",
	"--sidebar",
	"--sidebar-foreground",
	"--sidebar-primary",
	"--sidebar-primary-foreground",
	"--sidebar-accent",
	"--sidebar-accent-foreground",
	"--sidebar-border",
	"--sidebar-ring",
	"--ingot-surface",
	"--ingot-surface-elevated",
	"--ingot-border",
	"--ingot-border-strong",
	"--ingot-scrollbar-thumb",
	"--ingot-scrollbar-thumb-hover",
] as const

export const resolveCustomCssProperties = (config: CustomThemeConfig): Record<string, string> => {
	const { colors, radiusRem } = config
	return {
		"--radius": `${radiusRem}rem`,
		"--background": colors.background,
		"--foreground": colors.foreground,
		"--card": `color-mix(in srgb, ${colors.card} 82%, transparent)`,
		"--card-foreground": colors.cardForeground,
		"--popover": `color-mix(in srgb, ${colors.popover} 94%, transparent)`,
		"--popover-foreground": colors.popoverForeground,
		"--primary": colors.primary,
		"--primary-foreground": colors.primaryForeground,
		"--secondary": colors.muted,
		"--secondary-foreground": colors.foreground,
		"--muted": colors.muted,
		"--muted-foreground": colors.mutedForeground,
		"--accent": `color-mix(in srgb, ${colors.foreground} 7%, transparent)`,
		"--accent-foreground": colors.foreground,
		"--destructive": colors.destructive,
		"--border": colors.border,
		"--input": colors.border,
		"--ring": `color-mix(in srgb, ${colors.primary} 50%, transparent)`,
		"--sidebar": `color-mix(in srgb, ${colors.sidebar} 82%, transparent)`,
		"--sidebar-foreground": colors.sidebarForeground,
		"--sidebar-primary": colors.primary,
		"--sidebar-primary-foreground": colors.primaryForeground,
		"--sidebar-accent": `color-mix(in srgb, ${colors.foreground} 7%, transparent)`,
		"--sidebar-accent-foreground": colors.sidebarForeground,
		"--sidebar-border": colors.border,
		"--sidebar-ring": `color-mix(in srgb, ${colors.primary} 50%, transparent)`,
		"--ingot-surface": `color-mix(in srgb, ${colors.card} 82%, transparent)`,
		"--ingot-surface-elevated": `color-mix(in srgb, ${colors.popover} 94%, transparent)`,
		"--ingot-border": colors.border,
		"--ingot-border-strong": `color-mix(in srgb, ${colors.foreground} 16%, transparent)`,
		"--ingot-scrollbar-thumb": `color-mix(in srgb, ${colors.foreground} 16%, transparent)`,
		"--ingot-scrollbar-thumb-hover": `color-mix(in srgb, ${colors.foreground} 28%, transparent)`,
	}
}

export const applyCustomThemeVariables = (
	config: CustomThemeConfig,
	target?: HTMLElement,
): void => {
	const element = target ?? (typeof document !== "undefined" ? document.documentElement : null)
	if (!element) {
		return
	}

	const properties = resolveCustomCssProperties(config)
	for (const [name, value] of Object.entries(properties)) {
		element.style.setProperty(name, value)
	}

	element.classList.remove("custom-dark", "custom-light")
	element.classList.add(config.baseMode === "dark" ? "custom-dark" : "custom-light")
	element.style.colorScheme = config.baseMode
}

export const clearCustomThemeVariables = (target?: HTMLElement): void => {
	const element = target ?? (typeof document !== "undefined" ? document.documentElement : null)
	if (!element) {
		return
	}

	for (const name of CUSTOM_CSS_VARIABLES) {
		element.style.removeProperty(name)
	}
	element.classList.remove("custom-dark", "custom-light")
	element.style.removeProperty("color-scheme")
}
