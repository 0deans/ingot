import { AppWindow, Layers, Minimize2, Power, type LucideIcon } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore, type LauncherBehavior } from "@/shared/stores/useLauncherPrefsStore"

interface BehaviorOptionDef {
	id: LauncherBehavior
	icon: LucideIcon
	titleKey:
		| "settings.general.behavior.options.keepOpen.title"
		| "settings.general.behavior.options.hideToTray.title"
		| "settings.general.behavior.options.close.title"
	descriptionKey:
		| "settings.general.behavior.options.keepOpen.description"
		| "settings.general.behavior.options.hideToTray.description"
		| "settings.general.behavior.options.close.description"
}

const BEHAVIOR_OPTIONS: readonly BehaviorOptionDef[] = [
	{
		id: "keepOpen",
		icon: AppWindow,
		titleKey: "settings.general.behavior.options.keepOpen.title",
		descriptionKey: "settings.general.behavior.options.keepOpen.description",
	},
	{
		id: "hideToTray",
		icon: Minimize2,
		titleKey: "settings.general.behavior.options.hideToTray.title",
		descriptionKey: "settings.general.behavior.options.hideToTray.description",
	},
	{
		id: "close",
		icon: Power,
		titleKey: "settings.general.behavior.options.close.title",
		descriptionKey: "settings.general.behavior.options.close.description",
	},
]

const LauncherBehaviorCard = () => {
	const { t } = useTranslation()
	const { launcherBehavior, setLauncherBehavior } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				launcherBehavior: s.launcherBehavior,
				setLauncherBehavior: s.setLauncherBehavior,
			}
		}),
	)

	return (
		<section className="@container flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-start gap-2.5">
				<div className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg border border-border bg-foreground/5 text-primary">
					<Layers className="size-4" />
				</div>
				<div className="min-w-0">
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.general.behavior.title")}
					</h2>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						{t("settings.general.behavior.description")}
					</p>
				</div>
			</div>

			<div
				role="radiogroup"
				aria-label={t("settings.general.behavior.title")}
				className="grid grid-cols-1 gap-2.5 @lg:grid-cols-3"
			>
				{BEHAVIOR_OPTIONS.map((option) => {
					const Icon = option.icon
					const isSelected = launcherBehavior === option.id

					return (
						<button
							key={option.id}
							type="button"
							role="radio"
							aria-checked={isSelected}
							onClick={() => {
								setLauncherBehavior(option.id)
							}}
							className={cn(
								"flex cursor-pointer flex-col gap-1.5 rounded-xl border p-3.5 text-left transition-colors",
								isSelected
									? "border-primary/50 bg-primary/10"
									: "border-border bg-foreground/3 hover:bg-foreground/6",
							)}
						>
							<div className="flex items-center justify-between gap-2">
								<div className="flex items-center gap-2">
									<Icon
										className={cn(
											"size-4 shrink-0",
											isSelected ? "text-primary" : "text-muted-foreground",
										)}
									/>
									<span className="text-[12.5px] font-semibold text-foreground">
										{t(option.titleKey)}
									</span>
								</div>

								<span
									aria-hidden="true"
									className={cn(
										"flex size-4 shrink-0 items-center justify-center rounded-full border transition-colors",
										isSelected ? "border-primary bg-primary" : "border-border bg-transparent",
									)}
								>
									{isSelected ? (
										<span className="size-1.5 rounded-full bg-primary-foreground" />
									) : null}
								</span>
							</div>

							<p className="text-[11px] leading-relaxed text-muted-foreground">
								{t(option.descriptionKey)}
							</p>
						</button>
					)
				})}
			</div>
		</section>
	)
}

export default LauncherBehaviorCard
