import { Globe } from "lucide-react"
import { memo, useMemo } from "react"
import SearchableSelect from "@/components/common/searchable-select"
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
		<div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:flex-row sm:items-center sm:justify-between sm:gap-6 sm:p-5">
			<div className="flex min-w-0 flex-col gap-1">
				<div className="flex items-center gap-2">
					<Globe className="size-4 text-emerald-400" />
					<h3 className="font-semibold text-foreground text-sm">{t("settings.language.title")}</h3>
				</div>
				<p className="text-muted-foreground text-xs">{t("settings.language.description")}</p>
			</div>
			<SearchableSelect
				value={language}
				onValueChange={setLanguage}
				options={options}
				searchPlaceholder={t("common.search")}
				className="h-10 shrink-0 text-sm sm:w-72"
			/>
		</div>
	)
}

LanguageSettings.displayName = "LanguageSettings"

export default memo(LanguageSettings)
