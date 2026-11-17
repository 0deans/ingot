import { z } from "zod"

export const APP_LOCALES = [
	"en",
	"be",
	"zh",
	"cs",
	"nl",
	"fr",
	"de",
	"hu",
	"id",
	"it",
	"ja",
	"kk",
	"ko",
	"pl",
	"pt",
	"ro",
	"ru",
	"es",
	"sv",
	"tr",
	"uk",
	"vi",
] as const

export type AppLocaleCode = (typeof APP_LOCALES)[number]
export type LauncherLanguagePreference = "system" | AppLocaleCode

export const appLocaleSchema = z.enum(APP_LOCALES)
export const languagePreferenceSchema = z.union([z.literal("system"), appLocaleSchema])

export interface AppLanguageMeta {
	code: AppLocaleCode
	nativeName: string
	englishName: string
}

export const APP_LANGUAGES: readonly AppLanguageMeta[] = [
	{ code: "en", nativeName: "English", englishName: "English" },
	{ code: "be", nativeName: "Беларуская", englishName: "Belarusian" },
	{ code: "zh", nativeName: "简体中文", englishName: "Chinese (Simplified)" },
	{ code: "cs", nativeName: "Čeština", englishName: "Czech" },
	{ code: "nl", nativeName: "Nederlands", englishName: "Dutch" },
	{ code: "fr", nativeName: "Français", englishName: "French" },
	{ code: "de", nativeName: "Deutsch", englishName: "German" },
	{ code: "hu", nativeName: "Magyar", englishName: "Hungarian" },
	{ code: "id", nativeName: "Bahasa Indonesia", englishName: "Indonesian" },
	{ code: "it", nativeName: "Italiano", englishName: "Italian" },
	{ code: "ja", nativeName: "日本語", englishName: "Japanese" },
	{ code: "kk", nativeName: "Қазақша", englishName: "Kazakh" },
	{ code: "ko", nativeName: "한국어", englishName: "Korean" },
	{ code: "pl", nativeName: "Polski", englishName: "Polish" },
	{ code: "pt", nativeName: "Português", englishName: "Portuguese" },
	{ code: "ro", nativeName: "Română", englishName: "Romanian" },
	{ code: "ru", nativeName: "Русский", englishName: "Russian" },
	{ code: "es", nativeName: "Español", englishName: "Spanish" },
	{ code: "sv", nativeName: "Svenska", englishName: "Swedish" },
	{ code: "tr", nativeName: "Türkçe", englishName: "Turkish" },
	{ code: "uk", nativeName: "Українська", englishName: "Ukrainian" },
	{ code: "vi", nativeName: "Tiếng Việt", englishName: "Vietnamese" },
]

const SAMPLE_PREVIEW_DATE = new Date("2026-11-17T14:30:00Z")
const SAMPLE_SIZE_GB = 2.45
const SAMPLE_MOD_COUNT = 12480

const matchSupportedLocale = (rawLocale: string): AppLocaleCode | undefined => {
	const normalized = rawLocale.trim().toLowerCase()
	if (!normalized) {
		return undefined
	}

	const directMatch = appLocaleSchema.safeParse(normalized)
	if (directMatch.success) {
		return directMatch.data
	}

	const primarySubtag = normalized.split(/[-_]/)[0]
	const primaryMatch = appLocaleSchema.safeParse(primarySubtag)
	if (primaryMatch.success) {
		return primaryMatch.data
	}

	return undefined
}

export const detectSystemLocale = (
	navigatorLanguages?: readonly string[],
	navigatorLanguage?: string,
): AppLocaleCode => {
	const candidates: string[] = []
	if (navigatorLanguages) {
		for (const item of navigatorLanguages) {
			candidates.push(item)
		}
	} else if (typeof navigator !== "undefined" && Array.isArray(navigator.languages)) {
		for (const item of navigator.languages) {
			candidates.push(item)
		}
	}

	if (navigatorLanguage) {
		candidates.push(navigatorLanguage)
	} else if (typeof navigator !== "undefined" && typeof navigator.language === "string") {
		candidates.push(navigator.language)
	}

	for (const candidate of candidates) {
		const matched = matchSupportedLocale(candidate)
		if (matched) {
			return matched
		}
	}

	return "en"
}

export const resolveEffectiveLocale = (
	preference: LauncherLanguagePreference,
	systemLocale: AppLocaleCode = detectSystemLocale(),
): AppLocaleCode => {
	if (preference === "system") {
		return systemLocale
	}
	return preference
}

export const findLanguageMeta = (code: AppLocaleCode): AppLanguageMeta => {
	const found = APP_LANGUAGES.find((lang) => {
		return lang.code === code
	})
	return found ?? APP_LANGUAGES[0]
}

export const filterLanguages = (query: string): readonly AppLanguageMeta[] => {
	const normalized = query.trim().toLowerCase()
	if (!normalized) {
		return APP_LANGUAGES
	}

	return APP_LANGUAGES.filter((lang) => {
		return (
			lang.nativeName.toLowerCase().includes(normalized) ||
			lang.englishName.toLowerCase().includes(normalized) ||
			lang.code.toLowerCase().includes(normalized)
		)
	})
}

export interface LocaleFormatPreview {
	dateSample: string
	timeSample: string
	sizeSample: string
	numberSample: string
}

export const formatLocalePreview = (localeCode: AppLocaleCode): LocaleFormatPreview => {
	try {
		const dateSample = new Intl.DateTimeFormat(localeCode, {
			year: "numeric",
			month: "short",
			day: "numeric",
			timeZone: "UTC",
		}).format(SAMPLE_PREVIEW_DATE)

		const timeSample = new Intl.DateTimeFormat(localeCode, {
			hour: "2-digit",
			minute: "2-digit",
			timeZone: "UTC",
		}).format(SAMPLE_PREVIEW_DATE)

		const sizeNumber = new Intl.NumberFormat(localeCode, {
			minimumFractionDigits: 2,
			maximumFractionDigits: 2,
		}).format(SAMPLE_SIZE_GB)

		const numberSample = new Intl.NumberFormat(localeCode).format(SAMPLE_MOD_COUNT)

		return {
			dateSample,
			timeSample,
			sizeSample: `${sizeNumber} GB`,
			numberSample,
		}
	} catch {
		return {
			dateSample: "Nov 17, 2026",
			timeSample: "02:30 PM",
			sizeSample: "2.45 GB",
			numberSample: "12,480",
		}
	}
}
