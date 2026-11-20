import i18next, { type i18n as I18nInstance } from "i18next"
import HttpBackend, { type HttpBackendOptions } from "i18next-http-backend"
import { initReactI18next } from "react-i18next"
import {
	detectSystemLocale,
	resolveEffectiveLocale,
	type AppLocaleCode,
} from "@/shared/i18n/languages"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

export const AVAILABLE_TRANSLATION_LOCALES: readonly AppLocaleCode[] = ["en"]

export const isAvailableTranslationLocale = (code: string): code is AppLocaleCode => {
	return AVAILABLE_TRANSLATION_LOCALES.some((locale) => {
		return locale === code
	})
}

export const resolveTranslationLocale = (localeCode: AppLocaleCode): AppLocaleCode => {
	return isAvailableTranslationLocale(localeCode) ? localeCode : "en"
}

export const i18n: I18nInstance = i18next.createInstance()

let initPromise: Promise<I18nInstance> | null = null

export const syncI18nLanguage = async (effectiveLocale: AppLocaleCode): Promise<void> => {
	if (typeof document !== "undefined") {
		document.documentElement.lang = effectiveLocale
	}

	const translationLocale = resolveTranslationLocale(effectiveLocale)
	if (!i18n.isInitialized) {
		return
	}
	if (i18n.language === translationLocale) {
		return
	}
	await i18n.changeLanguage(translationLocale)
}

export const initI18n = (): Promise<I18nInstance> => {
	if (initPromise) {
		return initPromise
	}

	const initialPreference = useLauncherPrefsStore.getState().language
	const initialEffectiveLocale = resolveEffectiveLocale(initialPreference, detectSystemLocale())
	const initialTranslationLocale = resolveTranslationLocale(initialEffectiveLocale)

	if (typeof document !== "undefined") {
		document.documentElement.lang = initialEffectiveLocale
	}

	const backendOptions: HttpBackendOptions = {
		loadPath: "/locales/{{lng}}.json",
		requestOptions: import.meta.env.DEV ? { cache: "no-store" } : undefined,
	}

	initPromise = i18n
		.use(HttpBackend)
		.use(initReactI18next)
		.init({
			lng: initialTranslationLocale,
			fallbackLng: "en",
			supportedLngs: [...AVAILABLE_TRANSLATION_LOCALES],
			load: "languageOnly",
			nonExplicitSupportedLngs: true,
			interpolation: {
				escapeValue: false,
			},
			react: {
				useSuspense: false,
			},
			backend: backendOptions,
		})
		.then(() => {
			return i18n
		})

	return initPromise
}

interface I18nHmrPayload {
	lang?: string
}

if (import.meta.hot) {
	import.meta.hot.on("i18n:update", (payload: I18nHmrPayload) => {
		if (!i18n.isInitialized) {
			return
		}
		const targetLang = payload.lang ?? i18n.resolvedLanguage ?? "en"
		if (!isAvailableTranslationLocale(targetLang)) {
			return
		}
		void i18n
			.reloadResources(targetLang)
			.then(() => {
				return i18n.changeLanguage(i18n.language)
			})
			.catch((error: unknown) => {
				console.error("[Ingot] Failed to hot-reload i18n resources:", error)
			})
	})
}
