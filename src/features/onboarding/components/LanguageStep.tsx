import { Globe, Languages } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import OnboardingStepIntro from "@/features/onboarding/components/OnboardingStepIntro"
import SelectionCheckBadge from "@/features/onboarding/components/SelectionCheckBadge"
import {
	APP_LANGUAGES,
	LANGUAGE_NAME_KEYS,
	type LauncherLanguagePreference,
} from "@/shared/i18n/languages"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

const LanguageStep = () => {
	const { t } = useTranslation()
	const { language, setLanguage } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				language: s.language,
				setLanguage: s.setLanguage,
			}
		}),
	)

	const handleSelect = (next: LauncherLanguagePreference) => {
		setLanguage(next)
	}

	const isSystemSelected = language === "system"

	return (
		<div className="flex flex-col gap-4">
			<OnboardingStepIntro
				icon={Languages}
				title={t("onboarding.language.title")}
				subtitle={t("onboarding.language.subtitle")}
			/>

			<button
				type="button"
				onClick={() => {
					handleSelect("system")
				}}
				className={cn(
					"flex w-full cursor-pointer items-center justify-between gap-3 rounded-xl border px-3.5 py-2.5 text-left backdrop-blur-md outline-none",
					isSystemSelected
						? "border-primary/45 bg-primary/14 text-foreground"
						: "border-border/80 bg-card/55 text-muted-foreground hover:bg-card/80 hover:text-foreground",
				)}
			>
				<div className="flex min-w-0 items-center gap-2.5">
					<Globe className={cn("size-4 shrink-0", isSystemSelected && "text-primary")} />
					<div className="min-w-0">
						<p className="text-[13px] font-medium text-foreground">
							{t("settings.language.followSystem")}
						</p>
						<p className="truncate text-[11px] text-muted-foreground">
							{t("onboarding.language.systemHint")}
						</p>
					</div>
				</div>

				{isSystemSelected ? <SelectionCheckBadge size="md" /> : null}
			</button>

			<div className="grid grid-cols-1 gap-1.5 sm:grid-cols-2 md:grid-cols-3">
				{APP_LANGUAGES.map((lang) => {
					const isSelected = language === lang.code

					return (
						<button
							key={lang.code}
							type="button"
							onClick={() => {
								handleSelect(lang.code)
							}}
							className={cn(
								"flex min-w-0 cursor-pointer items-center justify-between gap-2 rounded-xl border px-3 py-2 text-left backdrop-blur-md outline-none",
								isSelected
									? "border-primary/45 bg-primary/14 text-foreground"
									: "border-border/80 bg-card/55 text-muted-foreground hover:bg-card/80 hover:text-foreground",
							)}
						>
							<div className="min-w-0">
								<p className="truncate text-[12.5px] font-medium text-foreground">
									{lang.nativeName}
								</p>
								<p className="truncate text-[10.5px] text-muted-foreground">
									{t(LANGUAGE_NAME_KEYS[lang.code])}
								</p>
							</div>

							{isSelected ? <SelectionCheckBadge size="sm" /> : null}
						</button>
					)
				})}
			</div>
		</div>
	)
}

export default LanguageStep
