import { Check, Monitor, Moon, Palette, Sun } from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import { type ThemeMode, useTheme } from "@/lib/theme"
import { cn } from "@/lib/utils"

const THEME_OPTIONS: {
	id: ThemeMode
	icon: typeof Monitor
}[] = [
	{ id: "auto", icon: Monitor },
	{ id: "dark", icon: Moon },
	{ id: "light", icon: Sun },
]

export const ThemeSettings = () => {
	const { t } = useTranslation()
	const { themeMode, setThemeMode } = useTheme()

	return (
		<div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
			<div className="flex flex-col gap-1">
				<div className="flex items-center gap-2">
					<Palette className="size-4 text-emerald-400" />
					<h3 className="font-semibold text-foreground text-sm">{t("settings.theme.title")}</h3>
				</div>
				<p className="text-muted-foreground text-xs">{t("settings.theme.description")}</p>
			</div>

			<div className="grid grid-cols-1 gap-2.5 pt-1 sm:grid-cols-3">
				{THEME_OPTIONS.map((option) => {
					const Icon = option.icon
					const isSelected = themeMode === option.id
					return (
						<button
							key={option.id}
							type="button"
							onClick={() => setThemeMode(option.id)}
							className={cn(
								"flex flex-col items-start gap-2 rounded-lg border p-3.5 text-left transition-all",
								isSelected
									? "border-primary/60 bg-primary/10 shadow-sm"
									: "border-border/30 bg-zinc-950/60 hover:border-border/60 hover:bg-zinc-900/60",
							)}
						>
							<div className="flex w-full items-center justify-between">
								<div
									className={cn(
										"flex size-7 items-center justify-center rounded-md transition-colors",
										isSelected
											? "bg-primary text-primary-foreground"
											: "bg-zinc-900 text-muted-foreground",
									)}
								>
									<Icon className="size-4" />
								</div>
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
									{t(`settings.theme.${option.id}`)}
								</div>
								<div className="mt-0.5 text-[11px] text-muted-foreground leading-tight">
									{t(`settings.theme.${option.id}Desc`)}
								</div>
							</div>
						</button>
					)
				})}
			</div>
		</div>
	)
}

ThemeSettings.displayName = "ThemeSettings"

export default memo(ThemeSettings)
