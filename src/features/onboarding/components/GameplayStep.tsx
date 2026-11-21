import { Cpu, SlidersHorizontal } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import OnboardingStepIntro from "@/features/onboarding/components/OnboardingStepIntro"
import SelectionCheckBadge from "@/features/onboarding/components/SelectionCheckBadge"
import { ONBOARDING_BEHAVIORS, RAM_PRESETS_GB } from "@/features/onboarding/onboardingSteps"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"

interface GameplayStepProps {
	isMobile: boolean
}

const GameplayStep = ({ isMobile }: GameplayStepProps) => {
	const { t } = useTranslation()
	const { ramGb, setRamGb, launcherBehavior, setLauncherBehavior } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				ramGb: s.ramGb,
				setRamGb: s.setRamGb,
				launcherBehavior: s.launcherBehavior,
				setLauncherBehavior: s.setLauncherBehavior,
			}
		}),
	)

	return (
		<div className="flex flex-col gap-5">
			<OnboardingStepIntro
				icon={Cpu}
				title={t("onboarding.gameplay.title")}
				subtitle={t("onboarding.gameplay.subtitle")}
			/>

			<div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-card/55 p-3.5 backdrop-blur-md">
				<div className="flex items-center justify-between gap-2">
					<span className="text-[12.5px] font-medium text-foreground">
						{t("onboarding.gameplay.ramLabel")}
					</span>
					<span className="rounded-md bg-primary/15 px-2 py-0.5 font-mono text-[12px] font-semibold text-primary">
						{ramGb} GB
					</span>
				</div>

				<input
					type="range"
					min={1}
					max={32}
					step={1}
					value={ramGb}
					aria-label={t("onboarding.gameplay.ramLabel")}
					onChange={(event) => {
						setRamGb(Number(event.target.value))
					}}
					className="h-1.5 w-full cursor-pointer accent-primary"
				/>

				<div className="flex flex-wrap gap-1.5">
					{RAM_PRESETS_GB.map((preset) => {
						const isSelected = ramGb === preset

						return (
							<Button
								key={preset}
								type="button"
								variant={isSelected ? "default" : "outline"}
								size="xs"
								onClick={() => {
									setRamGb(preset)
								}}
								className={cn(
									"h-7 cursor-pointer rounded-lg px-2.5 font-mono text-[11.5px]",
									!isSelected && "bg-foreground/4 text-muted-foreground hover:text-foreground",
								)}
							>
								{preset} GB
							</Button>
						)
					})}
				</div>
			</div>

			{!isMobile ? (
				<div className="flex flex-col gap-2">
					<span className="inline-flex items-center gap-1.5 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
						<SlidersHorizontal className="size-3" />
						{t("settings.general.behavior.title")}
					</span>

					<div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
						{ONBOARDING_BEHAVIORS.map((option) => {
							const isSelected = launcherBehavior === option.id

							return (
								<button
									key={option.id}
									type="button"
									onClick={() => {
										setLauncherBehavior(option.id)
									}}
									className={cn(
										"flex cursor-pointer flex-col gap-1 rounded-xl border p-3 text-left backdrop-blur-md outline-none",
										isSelected
											? "border-primary/50 bg-primary/14 text-foreground"
											: "border-border/80 bg-card/55 text-muted-foreground hover:bg-card/80 hover:text-foreground",
									)}
								>
									<div className="flex items-center justify-between gap-2">
										<span className="text-[12.5px] font-medium text-foreground">
											{t(option.titleKey)}
										</span>
										{isSelected ? <SelectionCheckBadge size="sm" /> : null}
									</div>
									<p className="text-[11px] leading-snug text-muted-foreground">
										{t(option.descriptionKey)}
									</p>
								</button>
							)
						})}
					</div>
				</div>
			) : null}
		</div>
	)
}

export default GameplayStep
