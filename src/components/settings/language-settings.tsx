import { Globe } from "lucide-react"
import { memo, useMemo } from "react"
import SearchableSelect from "@/components/common/searchable-select"
import { SectionCardHeader } from "@/components/common/section-card"
import { Card, CardContent } from "@/components/ui/card"
import { useLanguage } from "@/i18n/use-language"

/** One searchable dropdown: there are too many languages for a grid of cards */
export const LanguageSettings = () => {
	const { language, locales, systemLanguage, setLanguage, t } = useLanguage()

	const options = useMemo(
		() =>
			locales.map((loc) => ({
				value: loc.code,
				// Both names, so it can be found by either ("Deutsch" or "German")
				label: loc.nativeName === loc.name ? loc.nativeName : `${loc.nativeName} · ${loc.name}`,
				badge: loc.code === systemLanguage ? t("onboarding.detectedBadge") : undefined,
			})),
		[locales, systemLanguage, t],
	)

	return (
		<Card>
			<SectionCardHeader
				icon={Globe}
				title={t("settings.language.title")}
				description={t("settings.language.description")}
			/>
			<CardContent>
				<SearchableSelect
					value={language}
					onValueChange={setLanguage}
					options={options}
					searchPlaceholder={t("common.search")}
					className="sm:w-72"
				/>
			</CardContent>
		</Card>
	)
}

LanguageSettings.displayName = "LanguageSettings"

export default memo(LanguageSettings)
