import i18n from "i18next"
import { initReactI18next } from "react-i18next"

export const LANGUAGE_STORAGE_KEY = "ingot:language"

export interface LocaleMeta {
	code: string
	name: string
	nativeName: string
}

interface I18nHmrPayload {
	type: "update" | "add" | "remove"
	lang: string
	resources?: Record<string, unknown>
	availableLocales: LocaleMeta[]
}

const localeModules = import.meta.glob<Record<string, unknown>>("../locales/*.json", {
	eager: true,
	import: "default",
})

function resolveDisplayName(code: string, targetLocale: string): string {
	try {
		const displayNames = new Intl.DisplayNames([targetLocale], { type: "language" })
		const resolved = displayNames.of(code)
		if (resolved) {
			return resolved.charAt(0).toUpperCase() + resolved.slice(1)
		}
	} catch {
		// Ignore invalid locale tag
	}
	return code.toUpperCase()
}

function extractLocaleMeta(code: string, data: Record<string, unknown>): LocaleMeta {
	const meta =
		typeof data._meta === "object" && data._meta !== null
			? (data._meta as Record<string, unknown>)
			: undefined

	const name =
		typeof meta?.name === "string" && meta.name.trim().length > 0
			? meta.name.trim()
			: resolveDisplayName(code, "en")

	const nativeName =
		typeof meta?.nativeName === "string" && meta.nativeName.trim().length > 0
			? meta.nativeName.trim()
			: resolveDisplayName(code, code)

	return { code, name, nativeName }
}

const initialResources: Record<string, { translation: Record<string, unknown> }> = {}
const discoveredLocales: LocaleMeta[] = []

for (const [filePath, content] of Object.entries(localeModules)) {
	const match = filePath.match(/\/([^/]+)\.json$/)
	if (!match) continue
	const code = match[1]
	initialResources[code] = { translation: content }
	discoveredLocales.push(extractLocaleMeta(code, content))
}

discoveredLocales.sort((a, b) => {
	if (a.code === "en") return -1
	if (b.code === "en") return 1
	return a.name.localeCompare(b.name)
})

let availableLocales: LocaleMeta[] = discoveredLocales
const stateListeners = new Set<() => void>()

function notifyStateListeners() {
	for (const listener of stateListeners) {
		listener()
	}
}

export function detectSystemLanguage(locales: LocaleMeta[] = availableLocales): string {
	if (typeof navigator === "undefined") return "en"
	const candidates = navigator.languages?.length ? navigator.languages : [navigator.language]

	for (const raw of candidates) {
		if (!raw) continue
		const lower = raw.toLowerCase()
		const exact = locales.find((l) => l.code.toLowerCase() === lower)
		if (exact) return exact.code

		const base = lower.split("-")[0]
		const baseMatch = locales.find((l) => l.code.toLowerCase() === base)
		if (baseMatch) return baseMatch.code
	}

	return "en"
}

function readSavedLanguage(): string | null {
	if (typeof window === "undefined") return null
	try {
		const saved = window.localStorage.getItem(LANGUAGE_STORAGE_KEY)
		if (saved && availableLocales.some((l) => l.code === saved)) {
			return saved
		}
	} catch {
		// Ignore storage access errors
	}
	return null
}

const savedLanguage = readSavedLanguage()
export const systemLanguage = detectSystemLanguage(availableLocales)
let firstLaunchPending = savedLanguage === null

const initialLanguage = savedLanguage ?? systemLanguage

i18n.use(initReactI18next).init({
	resources: initialResources,
	lng: initialLanguage,
	fallbackLng: "en",
	interpolation: {
		escapeValue: false,
	},
	react: {
		useSuspense: false,
	},
})

if (typeof document !== "undefined") {
	document.documentElement.lang = i18n.language || initialLanguage
}

i18n.on("languageChanged", (lng) => {
	if (typeof document !== "undefined") {
		document.documentElement.lang = lng
	}
	notifyStateListeners()
})

export function subscribeLanguageState(listener: () => void): () => void {
	stateListeners.add(listener)
	return () => {
		stateListeners.delete(listener)
	}
}

export function getAvailableLocales(): LocaleMeta[] {
	return availableLocales
}

export function getIsFirstLaunchPending(): boolean {
	return firstLaunchPending
}

/**
 * Changes the active display language immediately (e.g. for live preview in the first-launch modal)
 * without dismissing the first-launch prompt.
 */
export async function previewLanguage(code: string): Promise<void> {
	await i18n.changeLanguage(code)
}

/**
 * Changes the active display language, persists it to localStorage,
 * and marks the first-launch language prompt as completed.
 */
export async function confirmLanguage(code: string): Promise<void> {
	await i18n.changeLanguage(code)
	try {
		window.localStorage.setItem(LANGUAGE_STORAGE_KEY, code)
	} catch {
		// Ignore localStorage write errors
	}
	if (firstLaunchPending) {
		firstLaunchPending = false
		notifyStateListeners()
	}
}

/**
 * Changes and persists the application language.
 */
export async function setLanguage(code: string): Promise<void> {
	await confirmLanguage(code)
}

// Custom Vite HMR integration for instant locale hot-reloading without state loss
if (import.meta.hot) {
	import.meta.hot.on("i18n:locale-update", (payload: I18nHmrPayload) => {
		if (payload.type === "remove") {
			i18n.removeResourceBundle(payload.lang, "translation")
			if (i18n.language === payload.lang) {
				i18n.changeLanguage("en")
			}
		} else if (payload.resources) {
			i18n.addResourceBundle(payload.lang, "translation", payload.resources, true, true)
		}

		if (Array.isArray(payload.availableLocales)) {
			availableLocales = payload.availableLocales
		}

		// Trigger react-i18next components to re-render with updated strings in-place
		i18n.emit("languageChanged", i18n.language)
		notifyStateListeners()
	})
}

export default i18n
