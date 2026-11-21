import { type LucideIcon } from "lucide-react"

interface OnboardingStepIntroProps {
	icon: LucideIcon
	title: string
	subtitle: string
}

const OnboardingStepIntro = ({ icon: Icon, title, subtitle }: OnboardingStepIntroProps) => {
	return (
		<div className="flex items-start gap-3">
			<div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-primary/25 bg-primary/10 text-primary">
				<Icon className="size-4.5" />
			</div>
			<div className="min-w-0">
				<h2 className="text-base font-semibold text-foreground sm:text-lg">{title}</h2>
				<p className="text-[12.5px] text-muted-foreground">{subtitle}</p>
			</div>
		</div>
	)
}

export default OnboardingStepIntro
