import { z } from "zod"
import { create } from "zustand"
import { persist } from "zustand/middleware"
import { languagePreferenceSchema, type LauncherLanguagePreference } from "@/shared/i18n/languages"
import {
	DEFAULT_CUSTOM_THEME,
	THEME_PRESETS,
	customThemeBaseModeSchema,
	customThemeConfigSchema,
	customThemeRadiusSchema,
	validateCssColor,
	type AppThemeId,
	type CustomThemeBaseMode,
	type CustomThemeColors,
	type CustomThemeConfig,
} from "@/shared/theme/customTheme"

export type { LauncherLanguagePreference }
export type NavVariant = "sonoma" | "rail" | "sequoia" | "capsule"
export type ToastPosition =
	| "top-left"
	| "top-center"
	| "top-right"
	| "bottom-left"
	| "bottom-center"
	| "bottom-right"

const appThemeSchema = z.enum(["light", "dark", "purple", "custom"])
const navVariantSchema = z.enum(["sonoma", "rail", "sequoia", "capsule"])
const toastPositionSchema = z.enum([
	"top-left",
	"top-center",
	"top-right",
	"bottom-left",
	"bottom-center",
	"bottom-right",
])
const ramGbSchema = z.number().min(1).max(64)
const persistedObjectSchema = z.record(z.string(), z.unknown())

interface PersistedLauncherPrefs {
	theme: AppThemeId
	navVariant: NavVariant
	toastPosition: ToastPosition
	language: LauncherLanguagePreference
	showDeveloperPages: boolean
	enableDevContextMenu: boolean
	ramGb: number
	railExpanded: boolean
	customTheme: CustomThemeConfig
}

interface LauncherPrefsState extends PersistedLauncherPrefs {
	setTheme: (theme: AppThemeId) => void
	setNavVariant: (variant: NavVariant) => void
	setToastPosition: (position: ToastPosition) => void
	setLanguage: (language: LauncherLanguagePreference) => void
	setShowDeveloperPages: (enabled: boolean) => void
	setEnableDevContextMenu: (enabled: boolean) => void
	setRamGb: (ramGb: number) => void
	toggleRailExpanded: () => void
	setCustomTheme: (config: CustomThemeConfig) => void
	updateCustomThemeColor: (key: keyof CustomThemeColors, value: string) => void
	setCustomThemeBaseMode: (baseMode: CustomThemeBaseMode) => void
	setCustomThemeRadius: (radiusRem: number) => void
	resetCustomTheme: (fromPreset?: Exclude<AppThemeId, "custom">) => void
}

export const useLauncherPrefsStore = create<LauncherPrefsState>()(
	persist(
		(set) => {
			return {
				theme: "dark",
				navVariant: "sonoma",
				toastPosition: "bottom-right",
				language: "system",
				showDeveloperPages: import.meta.env.DEV,
				enableDevContextMenu: import.meta.env.DEV,
				ramGb: 6,
				railExpanded: false,
				customTheme: DEFAULT_CUSTOM_THEME,
				setTheme: (theme) => {
					const parsed = appThemeSchema.safeParse(theme)
					if (!parsed.success) {
						return
					}
					set({ theme: parsed.data })
				},
				setNavVariant: (navVariant) => {
					const parsed = navVariantSchema.safeParse(navVariant)
					if (!parsed.success) {
						return
					}
					set({ navVariant: parsed.data })
				},
				setToastPosition: (toastPosition) => {
					const parsed = toastPositionSchema.safeParse(toastPosition)
					if (!parsed.success) {
						return
					}
					set({ toastPosition: parsed.data })
				},
				setLanguage: (language) => {
					const parsed = languagePreferenceSchema.safeParse(language)
					if (!parsed.success) {
						return
					}
					set({ language: parsed.data })
				},
				setShowDeveloperPages: (showDeveloperPages) => {
					set({ showDeveloperPages: Boolean(showDeveloperPages) })
				},
				setEnableDevContextMenu: (enableDevContextMenu) => {
					set({ enableDevContextMenu: Boolean(enableDevContextMenu) })
				},
				setRamGb: (ramGb) => {
					const parsed = ramGbSchema.safeParse(ramGb)
					if (!parsed.success) {
						return
					}
					set({ ramGb: parsed.data })
				},
				toggleRailExpanded: () => {
					set((state) => {
						return { railExpanded: !state.railExpanded }
					})
				},
				setCustomTheme: (customTheme) => {
					const parsed = customThemeConfigSchema.safeParse(customTheme)
					if (!parsed.success) {
						return
					}
					set({ customTheme: parsed.data })
				},
				updateCustomThemeColor: (key, value) => {
					const validation = validateCssColor(value)
					if (!validation.ok) {
						return
					}
					set((state) => {
						return {
							customTheme: {
								...state.customTheme,
								colors: {
									...state.customTheme.colors,
									[key]: validation.normalized,
								},
							},
						}
					})
				},
				setCustomThemeBaseMode: (baseMode) => {
					const parsed = customThemeBaseModeSchema.safeParse(baseMode)
					if (!parsed.success) {
						return
					}
					set((state) => {
						return {
							customTheme: {
								...state.customTheme,
								baseMode: parsed.data,
							},
						}
					})
				},
				setCustomThemeRadius: (radiusRem) => {
					if (!Number.isFinite(radiusRem)) {
						return
					}
					const clamped = Math.min(1.5, Math.max(0, radiusRem))
					const parsed = customThemeRadiusSchema.safeParse(clamped)
					if (!parsed.success) {
						return
					}
					set((state) => {
						return {
							customTheme: {
								...state.customTheme,
								radiusRem: parsed.data,
							},
						}
					})
				},
				resetCustomTheme: (fromPreset) => {
					const nextConfig = fromPreset ? THEME_PRESETS[fromPreset] : DEFAULT_CUSTOM_THEME
					set({
						customTheme: {
							...nextConfig,
							colors: { ...nextConfig.colors },
						},
					})
				},
			}
		},
		{
			name: "ingot-launcher-prefs",
			partialize: (state): PersistedLauncherPrefs => {
				return {
					theme: state.theme,
					navVariant: state.navVariant,
					toastPosition: state.toastPosition,
					language: state.language,
					showDeveloperPages: state.showDeveloperPages,
					enableDevContextMenu: state.enableDevContextMenu,
					ramGb: state.ramGb,
					railExpanded: state.railExpanded,
					customTheme: state.customTheme,
				}
			},
			merge: (persistedState, currentState) => {
				const parsedRecord = persistedObjectSchema.safeParse(persistedState)
				if (!parsedRecord.success) {
					return currentState
				}

				const raw = parsedRecord.data
				const parsedThemeId = appThemeSchema.safeParse(raw.theme)
				const parsedNav = navVariantSchema.safeParse(raw.navVariant)
				const parsedToastPos = toastPositionSchema.safeParse(raw.toastPosition)
				const parsedLanguage = languagePreferenceSchema.safeParse(raw.language)
				const parsedDevPages = z.boolean().safeParse(raw.showDeveloperPages)
				const parsedDevMenu = z.boolean().safeParse(raw.enableDevContextMenu)
				const parsedRam = ramGbSchema.safeParse(raw.ramGb)
				const parsedRail = z.boolean().safeParse(raw.railExpanded)
				const parsedTheme = customThemeConfigSchema.safeParse(raw.customTheme)

				return {
					...currentState,
					theme: parsedThemeId.success ? parsedThemeId.data : currentState.theme,
					navVariant: parsedNav.success ? parsedNav.data : currentState.navVariant,
					toastPosition: parsedToastPos.success ? parsedToastPos.data : currentState.toastPosition,
					language: parsedLanguage.success ? parsedLanguage.data : currentState.language,
					showDeveloperPages: parsedDevPages.success
						? parsedDevPages.data
						: currentState.showDeveloperPages,
					enableDevContextMenu: parsedDevMenu.success
						? parsedDevMenu.data
						: currentState.enableDevContextMenu,
					ramGb: parsedRam.success ? parsedRam.data : currentState.ramGb,
					railExpanded: parsedRail.success ? parsedRail.data : currentState.railExpanded,
					customTheme: parsedTheme.success ? parsedTheme.data : currentState.customTheme,
				}
			},
		},
	),
)
