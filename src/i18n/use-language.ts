import { useSyncExternalStore } from "react"
import { useTranslation } from "react-i18next"
import {
	confirmLanguage,
	getAvailableLocales,
	getIsFirstLaunchPending,
	previewLanguage,
	setLanguage,
	subscribeLanguageState,
	systemLanguage,
} from "@/i18n"

export function useLanguage() {
	const { t, i18n } = useTranslation()

	const locales = useSyncExternalStore(
		subscribeLanguageState,
		getAvailableLocales,
		getAvailableLocales,
	)

	const isFirstLaunchPending = useSyncExternalStore(
		subscribeLanguageState,
		getIsFirstLaunchPending,
		getIsFirstLaunchPending,
	)

	const currentCode = i18n.resolvedLanguage || i18n.language || "en"
	const currentLocale = locales.find((l) => l.code === currentCode) ??
		locales.find((l) => l.code === "en") ?? {
			code: currentCode,
			name: currentCode.toUpperCase(),
			nativeName: currentCode.toUpperCase(),
		}

	return {
		language: currentCode,
		currentLocale,
		systemLanguage,
		locales,
		isFirstLaunchPending,
		previewLanguage,
		confirmLanguage,
		setLanguage,
		t,
		i18n,
	}
}
