import "i18next"
import type enTranslation from "../../../public/locales/en.json"

declare module "i18next" {
	interface CustomTypeOptions {
		defaultNS: "translation"
		resources: {
			translation: typeof enTranslation
		}
	}
}
