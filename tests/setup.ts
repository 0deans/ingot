// Vitest setup file for DOM tests
import "@testing-library/react"
import { initReactI18next } from "react-i18next"
import { i18n } from "@/shared/i18n/i18n"
import enTranslation from "../public/locales/en.json"

if (!i18n.isInitialized) {
	void i18n.use(initReactI18next).init({
		lng: "en",
		fallbackLng: "en",
		resources: {
			en: {
				translation: enTranslation,
			},
		},
		interpolation: {
			escapeValue: false,
		},
		react: {
			useSuspense: false,
		},
	})
}
