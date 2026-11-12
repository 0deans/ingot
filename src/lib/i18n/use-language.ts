import { useSyncExternalStore } from "react"
import { useTranslation } from "react-i18next"
import {
	availableLocales,
	confirmLanguage,
	getIsFirstLaunchPending,
	previewLanguage,
	subscribeLanguageState,
	systemLanguage,
} from "@/lib/i18n"

const findLocale = (code: string) => {
	return availableLocales.find((l) => {
		return l.code === code
	})
}

const resolveLocale = (code: string) => {
	return (
		findLocale(code) ??
		findLocale("en") ?? {
			code,
			name: code.toUpperCase(),
			nativeName: code.toUpperCase(),
		}
	)
}

export const useLanguage = () => {
	const { t, i18n } = useTranslation()
	const isFirstLaunchPending = useSyncExternalStore(
		subscribeLanguageState,
		getIsFirstLaunchPending,
		getIsFirstLaunchPending,
	)
	const language = i18n.resolvedLanguage || i18n.language || "en"

	return {
		language,
		currentLocale: resolveLocale(language),
		systemLanguage,
		locales: availableLocales,
		isFirstLaunchPending,
		previewLanguage,
		confirmLanguage,
		t,
	}
}
