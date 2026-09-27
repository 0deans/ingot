import { Check, Globe } from "lucide-react"
import { memo } from "react"
import { useLanguage } from "@/i18n/use-language"
import { cn } from "@/lib/utils"

export const LanguageSettings = () => {
	const { language, locales, setLanguage, t } = useLanguage()

	return (
		<div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
			<div className="flex flex-col gap-1">
				<div className="flex items-center gap-2">
					<Globe className="size-4 text-emerald-400" />
					<h3 className="font-semibold text-foreground text-sm">{t("settings.language.title")}</h3>
				</div>
				<p className="text-muted-foreground text-xs">{t("settings.language.description")}</p>
			</div>

			<div className="grid grid-cols-1 gap-2.5 pt-1 sm:grid-cols-2 md:grid-cols-3">
				{locales.map((loc) => {
					const isSelected = language === loc.code
					return (
						<button
							key={loc.code}
							type="button"
							onClick={() => setLanguage(loc.code)}
							className={cn(
								"flex flex-col items-start gap-2 rounded-lg border p-3.5 text-left transition-all",
								isSelected
									? "border-primary/60 bg-primary/10 shadow-sm"
									: "border-border/30 bg-zinc-950/60 hover:border-border/60 hover:bg-zinc-900/60",
							)}
						>
							<div className="flex w-full items-center justify-between">
								<span
									className={cn(
										"inline-flex h-6 items-center justify-center rounded-md px-2 font-mono font-semibold text-[11px] uppercase transition-colors",
										isSelected
											? "bg-primary text-primary-foreground"
											: "bg-zinc-900 text-muted-foreground",
									)}
								>
									{loc.code}
								</span>
								{isSelected && (
									<span className="inline-flex items-center gap-1 rounded-full bg-primary/20 px-2 py-0.5 font-medium text-[10px] text-primary">
										<Check className="size-2.5" />
										{t("common.active")}
									</span>
								)}
							</div>

							<div>
								<div
									className={cn(
										"font-medium text-xs sm:text-sm",
										isSelected ? "font-semibold text-foreground" : "text-zinc-300",
									)}
								>
									{loc.nativeName}
								</div>
								<div className="mt-0.5 text-[11px] text-muted-foreground leading-tight">
									{loc.name}
								</div>
							</div>
						</button>
					)
				})}
			</div>
		</div>
	)
}

LanguageSettings.displayName = "LanguageSettings"

export default memo(LanguageSettings)
