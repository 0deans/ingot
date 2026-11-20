import {
	Calendar,
	Check,
	Clock,
	HardDrive,
	Hash,
	Languages,
	Monitor,
	Search,
	X,
} from "lucide-react"
import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import LanguageOptionCard from "@/features/settings/components/LanguageOptionCard"
import {
	APP_LANGUAGES,
	LANGUAGE_NAME_KEYS,
	detectSystemLocale,
	filterLanguages,
	findLanguageMeta,
	formatLocalePreview,
	resolveEffectiveLocale,
	type LauncherLanguagePreference,
} from "@/shared/i18n/languages"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { appleToast } from "@/shared/ui/sonner"

const LanguageSettingsPage = () => {
	const { t } = useTranslation()
	const { language, setLanguage } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				language: s.language,
				setLanguage: s.setLanguage,
			}
		}),
	)

	const [searchQuery, setSearchQuery] = useState("")

	const systemLocale = useMemo(() => {
		return detectSystemLocale()
	}, [])
	const systemLanguageMeta = useMemo(() => {
		return findLanguageMeta(systemLocale)
	}, [systemLocale])
	const effectiveLocale = useMemo(() => {
		return resolveEffectiveLocale(language, systemLocale)
	}, [language, systemLocale])
	const filteredLanguages = useMemo(() => {
		return filterLanguages(searchQuery)
	}, [searchQuery])
	const formatPreview = useMemo(() => {
		return formatLocalePreview(effectiveLocale)
	}, [effectiveLocale])
	const previewItems = useMemo(() => {
		return [
			{
				id: "date",
				label: t("settings.language.previewDate"),
				value: formatPreview.dateSample,
				icon: Calendar,
			},
			{
				id: "time",
				label: t("settings.language.previewTime"),
				value: formatPreview.timeSample,
				icon: Clock,
			},
			{
				id: "size",
				label: t("settings.language.previewFileSize"),
				value: formatPreview.sizeSample,
				icon: HardDrive,
			},
			{
				id: "number",
				label: t("settings.language.previewDownloads"),
				value: formatPreview.numberSample,
				icon: Hash,
			},
		]
	}, [formatPreview, t])

	const isSystemSelected = language === "system"
	const systemTranslatedName = t(LANGUAGE_NAME_KEYS[systemLanguageMeta.code])
	const systemLocaleLabel =
		systemLanguageMeta.nativeName === systemTranslatedName
			? systemLanguageMeta.nativeName
			: `${systemLanguageMeta.nativeName} (${systemTranslatedName})`

	const handleSelectLanguage = (nextPreference: LauncherLanguagePreference) => {
		setLanguage(nextPreference)
		if (nextPreference === "system") {
			appleToast.info(
				t("settings.language.savedToastSystem", { language: systemLanguageMeta.nativeName }),
				{
					description: t("settings.language.savedToastDescription"),
				},
			)
			return
		}
		const meta = findLanguageMeta(nextPreference)
		appleToast.info(t("settings.language.savedToastCustom", { language: meta.nativeName }), {
			description: t("settings.language.savedToastDescription"),
		})
	}

	return (
		<div className="flex flex-col gap-5">
			<section className="flex flex-col gap-4 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
				<div className="flex items-center gap-2">
					<Languages className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.language.title")}
					</h2>
				</div>

				<p className="text-[11.5px] leading-relaxed text-muted-foreground">
					{t("settings.language.description")}
				</p>

				<button
					type="button"
					aria-pressed={isSystemSelected}
					onClick={() => {
						handleSelectLanguage("system")
					}}
					className={cn(
						"flex cursor-pointer items-center justify-between gap-3 rounded-xl border p-3 text-left transition-colors",
						isSystemSelected
							? "border-primary/45 bg-primary/12 ring-1 ring-primary/25"
							: "border-border bg-foreground/3 hover:border-foreground/20 hover:bg-foreground/6",
					)}
				>
					<div className="flex min-w-0 items-center gap-3">
						<div
							className={cn(
								"flex size-9 shrink-0 items-center justify-center rounded-xl border",
								isSystemSelected
									? "border-primary/35 bg-primary/18 text-primary"
									: "border-border bg-foreground/5 text-muted-foreground",
							)}
						>
							<Monitor className="size-4" />
						</div>

						<div className="min-w-0">
							<span className="block text-[13px] font-semibold text-foreground">
								{t("settings.language.followSystem")}
							</span>
							<p className="mt-0.5 truncate text-[11px] text-muted-foreground">
								{t("settings.language.detectedOsLocale", { language: systemLocaleLabel })}
							</p>
						</div>
					</div>

					{isSystemSelected ? <Check className="size-4 shrink-0 text-primary" /> : null}
				</button>

				<div className="flex flex-col gap-2.5 rounded-xl border border-border bg-foreground/3 p-3">
					<div className="flex items-center justify-between gap-2">
						<span className="text-[11px] font-medium text-muted-foreground">
							{t("settings.language.formatPreviewTitle")}
						</span>
						<span className="rounded-md border border-border bg-foreground/5 px-1.5 py-0.5 font-mono text-[10px] font-semibold text-foreground/85 uppercase">
							{effectiveLocale}
						</span>
					</div>

					<div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
						{previewItems.map((item) => {
							const PreviewIcon = item.icon
							return (
								<div
									key={item.id}
									className="flex min-w-0 flex-col gap-1 rounded-lg border border-border/70 bg-background/55 px-2.5 py-2"
								>
									<div className="flex items-center gap-1 text-[10px] font-medium text-muted-foreground">
										<PreviewIcon className="size-3 shrink-0 text-primary" />
										<span className="truncate">{item.label}</span>
									</div>
									<span className="truncate font-mono text-[11.5px] font-medium text-foreground">
										{item.value}
									</span>
								</div>
							)
						})}
					</div>
				</div>

				<div className="relative min-w-0 sm:max-w-xs">
					<Search className="pointer-events-none absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted-foreground" />
					<input
						type="text"
						value={searchQuery}
						onChange={(event) => {
							setSearchQuery(event.target.value)
						}}
						placeholder={t("settings.language.searchPlaceholder", {
							count: APP_LANGUAGES.length,
						})}
						aria-label={t("settings.language.searchAriaLabel")}
						className="h-9 w-full rounded-xl border border-border bg-foreground/4 pr-8 pl-8.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/50 focus:outline-none"
					/>
					{searchQuery ? (
						<button
							type="button"
							onClick={() => {
								setSearchQuery("")
							}}
							aria-label={t("settings.language.clearSearchAriaLabel")}
							className="absolute top-1/2 right-2 flex size-5 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
						>
							<X className="size-3" />
						</button>
					) : null}
				</div>

				{filteredLanguages.length > 0 ? (
					<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
						{filteredLanguages.map((item) => {
							return (
								<LanguageOptionCard
									key={item.code}
									language={item}
									isSelected={language === item.code}
									isSystemDetected={item.code === systemLocale}
									onSelect={handleSelectLanguage}
								/>
							)
						})}
					</div>
				) : (
					<div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border bg-foreground/2 px-4 py-8 text-center">
						<p className="text-xs font-medium text-foreground">
							{t("settings.language.noMatches", { query: searchQuery.trim() })}
						</p>
						<button
							type="button"
							onClick={() => {
								setSearchQuery("")
							}}
							className="cursor-pointer rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/10"
						>
							{t("common.clearSearch")}
						</button>
					</div>
				)}
			</section>
		</div>
	)
}

export default LanguageSettingsPage
