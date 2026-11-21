import { type LauncherBehavior, type NavVariant } from "@/shared/stores/useLauncherPrefsStore"
import { type AppThemeId } from "@/shared/theme/customTheme"

export type OnboardingStepId = "welcome" | "language" | "appearance" | "gameplay"

export const ONBOARDING_STEPS: readonly OnboardingStepId[] = [
	"welcome",
	"language",
	"appearance",
	"gameplay",
]

export const ONBOARDING_EASE = [0.22, 1, 0.36, 1] as const

export const RAM_PRESETS_GB = [2, 4, 6, 8, 12] as const

export interface OnboardingThemeOption {
	id: Exclude<AppThemeId, "custom">
	labelKey:
		| "settings.appearance.themes.dark.title"
		| "settings.appearance.themes.purple.title"
		| "settings.appearance.themes.light.title"
	surfaceClass: string
	accentClass: string
}

export const ONBOARDING_THEMES: readonly OnboardingThemeOption[] = [
	{
		id: "dark",
		labelKey: "settings.appearance.themes.dark.title",
		surfaceClass: "bg-zinc-950 border-zinc-800",
		accentClass: "bg-zinc-200",
	},
	{
		id: "purple",
		labelKey: "settings.appearance.themes.purple.title",
		surfaceClass: "bg-[#120d1d] border-[#2d2147]",
		accentClass: "bg-[#a855f7]",
	},
	{
		id: "light",
		labelKey: "settings.appearance.themes.light.title",
		surfaceClass: "bg-zinc-100 border-zinc-300",
		accentClass: "bg-zinc-900",
	},
]

export interface OnboardingNavOption {
	id: NavVariant
	labelKey:
		| "settings.appearance.navVariants.capsule.title"
		| "settings.appearance.navVariants.sonoma.title"
		| "settings.appearance.navVariants.rail.title"
		| "settings.appearance.navVariants.sequoia.title"
}

export const ONBOARDING_NAV_VARIANTS: readonly OnboardingNavOption[] = [
	{
		id: "capsule",
		labelKey: "settings.appearance.navVariants.capsule.title",
	},
	{
		id: "sonoma",
		labelKey: "settings.appearance.navVariants.sonoma.title",
	},
	{
		id: "rail",
		labelKey: "settings.appearance.navVariants.rail.title",
	},
	{
		id: "sequoia",
		labelKey: "settings.appearance.navVariants.sequoia.title",
	},
]

export interface OnboardingBehaviorOption {
	id: LauncherBehavior
	titleKey:
		| "settings.general.behavior.options.keepOpen.title"
		| "settings.general.behavior.options.hideToTray.title"
		| "settings.general.behavior.options.close.title"
	descriptionKey:
		| "settings.general.behavior.options.keepOpen.description"
		| "settings.general.behavior.options.hideToTray.description"
		| "settings.general.behavior.options.close.description"
}

export const ONBOARDING_BEHAVIORS: readonly OnboardingBehaviorOption[] = [
	{
		id: "keepOpen",
		titleKey: "settings.general.behavior.options.keepOpen.title",
		descriptionKey: "settings.general.behavior.options.keepOpen.description",
	},
	{
		id: "hideToTray",
		titleKey: "settings.general.behavior.options.hideToTray.title",
		descriptionKey: "settings.general.behavior.options.hideToTray.description",
	},
	{
		id: "close",
		titleKey: "settings.general.behavior.options.close.title",
		descriptionKey: "settings.general.behavior.options.close.description",
	},
]
