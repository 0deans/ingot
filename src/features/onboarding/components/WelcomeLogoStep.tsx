import { ArrowRight, Layers, Package, Zap, type LucideIcon } from "lucide-react"
import { motion, useReducedMotion } from "motion/react"
import { useTranslation } from "react-i18next"
import { ONBOARDING_EASE } from "@/features/onboarding/onboardingSteps"
import { Button } from "@/shared/ui/button"

interface WelcomeLogoStepProps {
	onStart: () => void
	onSkip: () => void
}

interface FeaturePill {
	id: string
	icon: LucideIcon
	labelKey:
		| "onboarding.welcome.pillFast"
		| "onboarding.welcome.pillModpacks"
		| "onboarding.welcome.pillThemes"
}

const FEATURE_PILLS: readonly FeaturePill[] = [
	{
		id: "fast",
		icon: Zap,
		labelKey: "onboarding.welcome.pillFast",
	},
	{
		id: "modpacks",
		icon: Package,
		labelKey: "onboarding.welcome.pillModpacks",
	},
	{
		id: "themes",
		icon: Layers,
		labelKey: "onboarding.welcome.pillThemes",
	},
]

const WelcomeLogoStep = ({ onStart, onSkip }: WelcomeLogoStepProps) => {
	const { t } = useTranslation()
	const prefersReducedMotion = useReducedMotion()

	return (
		<div className="mx-auto flex max-w-xl flex-col items-center justify-center px-4 py-4 text-center sm:py-6">
			<div className="relative mb-6 flex items-center justify-center">
				<motion.div
					aria-hidden="true"
					initial={prefersReducedMotion ? { opacity: 0.45 } : { opacity: 0, scale: 0.65 }}
					animate={
						prefersReducedMotion
							? { opacity: 0.45 }
							: {
									opacity: [0, 0.55, 0.35],
									scale: [0.65, 1.15, 1],
								}
					}
					transition={{ duration: 0.95, ease: ONBOARDING_EASE }}
					className="pointer-events-none absolute size-36 rounded-full bg-amber-500/20 blur-3xl"
				/>

				<motion.div
					initial={
						prefersReducedMotion ? { opacity: 1 } : { opacity: 0, scale: 0.7, y: 8, rotate: -6 }
					}
					animate={{ opacity: 1, scale: 1, y: 0, rotate: 0 }}
					transition={
						prefersReducedMotion
							? { duration: 0.15 }
							: {
									type: "spring",
									stiffness: 240,
									damping: 20,
									mass: 0.9,
								}
					}
					className="relative flex items-center justify-center"
				>
					<motion.img
						src="/ingot.svg"
						alt={t("shell.logoAlt")}
						draggable={false}
						animate={
							prefersReducedMotion
								? undefined
								: {
										y: [0, -6, 0],
									}
						}
						transition={
							prefersReducedMotion
								? undefined
								: {
										duration: 3.6,
										repeat: Number.POSITIVE_INFINITY,
										ease: "easeInOut",
									}
						}
						className="pointer-events-none size-20 object-contain drop-shadow-[0_12px_28px_rgba(245,158,11,0.28)] sm:size-24"
					/>
				</motion.div>
			</div>

			<motion.div
				initial={prefersReducedMotion ? { opacity: 1 } : { opacity: 0, y: 12, filter: "blur(8px)" }}
				animate={prefersReducedMotion ? { opacity: 1 } : { opacity: 1, y: 0, filter: "blur(0px)" }}
				transition={{
					duration: 0.45,
					delay: prefersReducedMotion ? 0 : 0.14,
					ease: ONBOARDING_EASE,
				}}
				className="flex flex-col items-center gap-2.5"
			>
				<h1 className="text-3xl font-bold tracking-[-0.03em] text-foreground/85 sm:text-5xl">
					{t("onboarding.welcome.titlePrefix")}
					<span className="bg-linear-to-r from-amber-300 via-orange-400 to-amber-500 bg-clip-text text-transparent">
						Ingot
					</span>
				</h1>

				<p className="max-w-md text-[13.5px] leading-relaxed text-muted-foreground sm:text-[15px]">
					{t("onboarding.welcome.subtitle")}
				</p>
			</motion.div>

			<motion.div
				initial={prefersReducedMotion ? { opacity: 1 } : { opacity: 0, y: 10 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{
					duration: 0.36,
					delay: prefersReducedMotion ? 0 : 0.24,
					ease: ONBOARDING_EASE,
				}}
				className="mt-5 flex flex-wrap items-center justify-center gap-2"
			>
				{FEATURE_PILLS.map((pill) => {
					const PillIcon = pill.icon
					return (
						<span
							key={pill.id}
							className="inline-flex items-center gap-1.5 rounded-full border border-border/80 bg-card/65 px-3 py-1 text-[11.5px] font-medium text-muted-foreground backdrop-blur-md"
						>
							<PillIcon className="size-3.5 text-primary" />
							<span>{t(pill.labelKey)}</span>
						</span>
					)
				})}
			</motion.div>

			<motion.div
				initial={prefersReducedMotion ? { opacity: 1 } : { opacity: 0, y: 8 }}
				animate={{ opacity: 1, y: 0 }}
				transition={{
					duration: 0.35,
					delay: prefersReducedMotion ? 0 : 0.32,
					ease: ONBOARDING_EASE,
				}}
				className="mt-8 flex w-full max-w-xs flex-col gap-2.5 sm:w-auto sm:max-w-none sm:flex-row sm:items-center"
			>
				<motion.div
					whileHover={prefersReducedMotion ? undefined : { scale: 1.03 }}
					whileTap={prefersReducedMotion ? undefined : { scale: 0.97 }}
					transition={{ type: "spring", stiffness: 420, damping: 24 }}
					className="w-full sm:w-auto"
				>
					<Button
						type="button"
						size="lg"
						onClick={onStart}
						className="group relative h-11 w-full cursor-pointer gap-2 overflow-hidden rounded-xl px-6 text-[13.5px] font-semibold shadow-[0_8px_28px_-6px_color-mix(in_oklab,var(--color-primary)_65%,transparent)] sm:w-auto"
					>
						{!prefersReducedMotion ? (
							<motion.span
								aria-hidden="true"
								initial={{ x: "-130%" }}
								animate={{ x: "230%" }}
								transition={{
									duration: 1.15,
									ease: [0.22, 1, 0.36, 1],
									repeat: Number.POSITIVE_INFINITY,
									repeatDelay: 2.8,
								}}
								className="pointer-events-none absolute inset-y-0 left-0 w-1/2 -skew-x-20 bg-linear-to-r from-transparent via-white/30 to-transparent"
							/>
						) : null}
						<span className="relative z-10">{t("onboarding.welcome.getStarted")}</span>
						<ArrowRight className="relative z-10 size-4 transition-transform duration-150 group-hover:translate-x-1" />
					</Button>
				</motion.div>

				<Button
					type="button"
					variant="outline"
					size="lg"
					onClick={onSkip}
					className="h-11 cursor-pointer rounded-xl bg-card/60 px-5 text-[13.5px] text-muted-foreground backdrop-blur-md hover:text-foreground"
				>
					{t("onboarding.actions.skipSetup")}
				</Button>
			</motion.div>
		</div>
	)
}

export default WelcomeLogoStep
