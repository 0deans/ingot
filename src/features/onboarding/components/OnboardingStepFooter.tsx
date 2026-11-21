import { ArrowLeft, ArrowRight, Check } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"

interface OnboardingStepFooterProps {
	isLastStep: boolean
	isMobile: boolean
	onBack: () => void
	onNext: () => void
}

const OnboardingStepFooter = ({
	isLastStep,
	isMobile,
	onBack,
	onNext,
}: OnboardingStepFooterProps) => {
	const { t } = useTranslation()

	return (
		<div
			className={cn(
				"mx-auto flex w-full max-w-3xl shrink-0 items-center justify-between gap-2 px-4 py-4 sm:px-6",
				isMobile && "pb-[max(0.85rem,env(safe-area-inset-bottom))]",
			)}
		>
			<Button
				type="button"
				variant="outline"
				size="lg"
				onClick={onBack}
				className="h-10 cursor-pointer rounded-xl border-border/80 bg-card/60 px-4 text-[13px] text-muted-foreground backdrop-blur-md hover:text-foreground"
			>
				<ArrowLeft className="size-3.5" />
				<span>{t("onboarding.actions.back")}</span>
			</Button>

			<Button
				type="button"
				size="lg"
				onClick={onNext}
				className="group h-10 cursor-pointer rounded-xl px-5 text-[13px] font-semibold shadow-[0_8px_24px_-6px_color-mix(in_oklab,var(--color-primary)_60%,transparent)]"
			>
				<span>
					{isLastStep ? t("onboarding.actions.finish") : t("onboarding.actions.continue")}
				</span>
				{isLastStep ? (
					<Check className="size-3.5" />
				) : (
					<ArrowRight className="size-3.5 transition-transform duration-150 group-hover:translate-x-0.5" />
				)}
			</Button>
		</div>
	)
}

export default OnboardingStepFooter
