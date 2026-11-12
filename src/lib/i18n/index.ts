import i18n, { type ParseKeys } from "i18next"
import HttpBackend from "i18next-http-backend"
import { initReactI18next } from "react-i18next"

/** Any key of en.json, for tables of texts kept outside components */
export type TranslationKey = ParseKeys<"translation">

const LANGUAGE_STORAGE_KEY = "ingot:language"

interface LocaleMeta {
	code: string
	name: string
	nativeName: string
}

const resolveDisplayName = (code: string, targetLocale: string): string => {
	try {
		const resolved = new Intl.DisplayNames([targetLocale], { type: "language" }).of(code)
		if (resolved) {
			return resolved.charAt(0).toUpperCase() + resolved.slice(1)
		}
	} catch {
		// Ignore invalid locale tag
	}
	return code.toUpperCase()
}

export const availableLocales: LocaleMeta[] = Object.keys(
	import.meta.glob("../../../public/locales/*.json"),
)
	.map((filePath) => {
		const code = filePath.match(/\/([^/]+)\.json$/)?.[1]
		return code
			? {
					code,
					name: resolveDisplayName(code, "en"),
					nativeName: resolveDisplayName(code, code),
				}
			: null
	})
	.filter((item): item is LocaleMeta => {
		return item !== null
	})
	.sort((a, b) => {
		if (a.code === "en") {
			return -1
		}
		if (b.code === "en") {
			return 1
		}
		return a.name.localeCompare(b.name)
	})

const stateListeners = new Set<() => void>()

const notifyStateListeners = () => {
	for (const listener of stateListeners) {
		listener()
	}
}

const detectSystemLanguage = (): string => {
	if (typeof navigator === "undefined") {
		return "en"
	}
	const candidates = navigator.languages?.length ? navigator.languages : [navigator.language]
	for (const raw of candidates) {
		const lower = raw?.toLowerCase()
		if (!lower) {
			continue
		}
		const match =
			availableLocales.find((l) => {
				return l.code.toLowerCase() === lower
			}) ??
			availableLocales.find((l) => {
				return l.code.toLowerCase() === lower.split("-")[0]
			})
		if (match) {
			return match.code
		}
	}
	return "en"
}

const readSavedLanguage = (): string | null => {
	try {
		const saved = window.localStorage.getItem(LANGUAGE_STORAGE_KEY)
		return saved &&
			availableLocales.some((l) => {
				return l.code === saved
			})
			? saved
			: null
	} catch {
		return null
	}
}

const savedLanguage = readSavedLanguage()
export const systemLanguage = detectSystemLanguage()
const initialLanguage = savedLanguage ?? systemLanguage
let firstLaunchPending = savedLanguage === null

void i18n
	.use(HttpBackend)
	.use(initReactI18next)
	.init({
		backend: {
			loadPath: "/locales/{{lng}}.json",
			requestOptions: import.meta.env.DEV ? { cache: "no-store" } : undefined,
		},
		lng: initialLanguage,
		fallbackLng: "en",
		supportedLngs: availableLocales.map((l) => {
			return l.code
		}),
		interpolation: { escapeValue: false },
		react: { useSuspense: false, bindI18nStore: "added" },
	})

const syncDocumentLang = (lng: string) => {
	if (typeof document !== "undefined") {
		document.documentElement.lang = lng
	}
}

syncDocumentLang(initialLanguage)
i18n.on("languageChanged", (lng) => {
	syncDocumentLang(lng)
})

export const subscribeLanguageState = (listener: () => void): (() => void) => {
	stateListeners.add(listener)
	return () => {
		stateListeners.delete(listener)
	}
}

export const getIsFirstLaunchPending = (): boolean => {
	return firstLaunchPending
}

export const previewLanguage = async (code: string): Promise<void> => {
	await i18n.changeLanguage(code)
}

export const confirmLanguage = async (code: string): Promise<void> => {
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

if (import.meta.hot) {
	import.meta.hot.on("i18n:update", async ({ lang }: { lang: string }) => {
		if (!i18n.languages?.includes(lang)) {
			return
		}
		await i18n.reloadResources(lang)
	})
}
