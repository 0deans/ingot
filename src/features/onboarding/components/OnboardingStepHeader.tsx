import { Check, Cpu, Languages, Palette, type LucideIcon } from "lucide-react"
import { motion } from "motion/react"
import { useTranslation } from "react-i18next"
import {
	ONBOARDING_EASE,
	ONBOARDING_STEPS,
	type OnboardingStepId,
} from "@/features/onboarding/onboardingSteps"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

interface OnboardingStepHeaderProps {
	currentStep: OnboardingStepId
	stepIndex: number
	isMobile: boolean
	onSelectStep: (nextIndex: number) => void
	onSkip: () => void
}

interface StepPillMeta {
	id: Exclude<OnboardingStepId, "welcome">
	stepIndex: number
	icon: LucideIcon
	titleKey:
		| "onboarding.steps.language"
		| "onboarding.steps.appearance"
		| "onboarding.steps.gameplay"
}

const STEP_PILLS: readonly StepPillMeta[] = [
	{
		id: "language",
		stepIndex: ONBOARDING_STEPS.indexOf("language"),
		icon: Languages,
		titleKey: "onboarding.steps.language",
	},
	{
		id: "appearance",
		stepIndex: ONBOARDING_STEPS.indexOf("appearance"),
		icon: Palette,
		titleKey: "onboarding.steps.appearance",
	},
	{
		id: "gameplay",
		stepIndex: ONBOARDING_STEPS.indexOf("gameplay"),
		icon: Cpu,
		titleKey: "onboarding.steps.gameplay",
	},
]

const OnboardingStepHeader = ({
	currentStep,
	stepIndex,
	isMobile,
	onSelectStep,
	onSkip,
}: OnboardingStepHeaderProps) => {
	const { t } = useTranslation()

	const totalSetupSteps = ONBOARDING_STEPS.length - 1
	const activeSetupStepNumber = Math.max(1, stepIndex)
	const progressPercent = (activeSetupStepNumber / totalSetupSteps) * 100

	return (
		<div
			className={cn(
				"mx-auto flex w-full max-w-3xl shrink-0 items-center justify-between gap-3 px-4 py-3 sm:px-6",
				isMobile && "pt-[max(0.75rem,env(safe-area-inset-top))]",
			)}
		>
			{isMobile ? (
				<div className="flex min-w-0 flex-1 items-center gap-3">
					<span className="text-[11.5px] font-medium whitespace-nowrap text-muted-foreground">
						{t("onboarding.stepCounter", {
							current: activeSetupStepNumber,
							total: totalSetupSteps,
						})}
					</span>
					<div className="h-1.5 flex-1 overflow-hidden rounded-full bg-foreground/10">
						<motion.div
							animate={{ width: `${progressPercent}%` }}
							transition={{ duration: 0.28, ease: ONBOARDING_EASE }}
							className="h-full rounded-full bg-primary"
						/>
					</div>
				</div>
			) : (
				<nav
					aria-label={t("onboarding.stepsAria")}
					className="flex items-center gap-1 rounded-full border border-border/80 bg-card/55 p-1 backdrop-blur-md"
				>
					{STEP_PILLS.map((step) => {
						const Icon = step.icon
						const isCurrent = currentStep === step.id
						const isCompleted = stepIndex > step.stepIndex

						return (
							<button
								key={step.id}
								type="button"
								onClick={() => {
									onSelectStep(step.stepIndex)
								}}
								className={cn(
									"relative flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1 text-[12px] font-medium whitespace-nowrap outline-none",
									isCurrent ? "text-foreground" : "text-muted-foreground hover:text-foreground",
								)}
							>
								{isCurrent ? (
									<motion.span
										layoutId="onboarding-step-pill"
										transition={{ type: "spring", stiffness: 440, damping: 34 }}
										className="absolute inset-0 rounded-full bg-foreground/12 ring-1 ring-border"
									/>
								) : null}

								<span className="relative z-10 flex items-center justify-center">
									{isCompleted ? (
										<Check className="size-3.5 text-primary" />
									) : (
										<Icon className={cn("size-3.5", isCurrent && "text-primary")} />
									)}
								</span>
								<span className="relative z-10">{t(step.titleKey)}</span>
							</button>
						)
					})}
				</nav>
			)}

			<Button
				type="button"
				variant="outline"
				size="sm"
				onClick={onSkip}
				className="shrink-0 cursor-pointer rounded-full border-border/70 bg-card/50 px-3 text-[12px] text-muted-foreground backdrop-blur-md hover:text-foreground"
			>
				{t("onboarding.actions.skip")}
			</Button>
		</div>
	)
}

export default OnboardingStepHeader
