import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import AppearanceStep from "@/features/onboarding/components/AppearanceStep"
import GameplayStep from "@/features/onboarding/components/GameplayStep"
import LanguageStep from "@/features/onboarding/components/LanguageStep"
import OnboardingBackdrop from "@/features/onboarding/components/OnboardingBackdrop"
import OnboardingStepFooter from "@/features/onboarding/components/OnboardingStepFooter"
import OnboardingStepHeader from "@/features/onboarding/components/OnboardingStepHeader"
import WelcomeLogoStep from "@/features/onboarding/components/WelcomeLogoStep"
import {
	ONBOARDING_EASE,
	ONBOARDING_STEPS,
	type OnboardingStepId,
} from "@/features/onboarding/onboardingSteps"
import WindowControlButtons from "@/shared/components/window/WindowControlButtons"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

interface OnboardingFlowProps {
	isMobile: boolean
}

const stepSlideVariants = {
	enter: (direction: number) => {
		return {
			opacity: 0,
			x: direction * 28,
		}
	},
	center: {
		opacity: 1,
		x: 0,
	},
	exit: (direction: number) => {
		return {
			opacity: 0,
			x: direction * -28,
		}
	},
}

const OnboardingFlow = ({ isMobile }: OnboardingFlowProps) => {
	const { t } = useTranslation()
	const prefersReducedMotion = useReducedMotion()
	const setOnboardingCompleted = useLauncherPrefsStore((s) => {
		return s.setOnboardingCompleted
	})

	const [stepIndex, setStepIndex] = useState(0)
	const [direction, setDirection] = useState(1)

	const currentStep: OnboardingStepId = ONBOARDING_STEPS[stepIndex] ?? "welcome"
	const isWelcomeStep = currentStep === "welcome"
	const isLastStep = stepIndex === ONBOARDING_STEPS.length - 1

	const handleFinish = useCallback(() => {
		setOnboardingCompleted(true)
	}, [setOnboardingCompleted])

	const handleSelectStep = useCallback(
		(nextIndex: number) => {
			if (nextIndex === stepIndex || nextIndex < 0 || nextIndex >= ONBOARDING_STEPS.length) {
				return
			}
			setDirection(nextIndex > stepIndex ? 1 : -1)
			setStepIndex(nextIndex)
		},
		[stepIndex],
	)

	const handleNext = useCallback(() => {
		if (stepIndex >= ONBOARDING_STEPS.length - 1) {
			handleFinish()
			return
		}
		setDirection(1)
		setStepIndex((prev) => {
			return Math.min(ONBOARDING_STEPS.length - 1, prev + 1)
		})
	}, [handleFinish, stepIndex])

	const handleBack = useCallback(() => {
		if (stepIndex <= 0) {
			return
		}
		setDirection(-1)
		setStepIndex((prev) => {
			return Math.max(0, prev - 1)
		})
	}, [stepIndex])

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				handleFinish()
			}
		}

		window.addEventListener("keydown", handleKeyDown)
		return () => {
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [handleFinish])

	const renderSetupStepContent = (): React.ReactNode => {
		switch (currentStep) {
			case "language":
				return <LanguageStep />
			case "appearance":
				return <AppearanceStep isMobile={isMobile} />
			case "gameplay":
				return <GameplayStep isMobile={isMobile} />
			case "welcome":
				return null
		}
	}

	return (
		<motion.div
			initial={{ opacity: 0 }}
			animate={{ opacity: 1 }}
			exit={{ opacity: 0, scale: prefersReducedMotion ? 1 : 0.98 }}
			transition={{ duration: 0.24, ease: ONBOARDING_EASE }}
			role="dialog"
			aria-modal="true"
			aria-label={t("onboarding.dialogAria")}
			className="fixed inset-0 z-50 flex flex-col overflow-hidden bg-background text-foreground select-none"
		>
			<OnboardingBackdrop currentStep={currentStep} />

			{!isMobile ? (
				<header
					data-tauri-drag-region
					className="relative z-10 flex h-10 shrink-0 items-center justify-between pl-3"
				>
					<div data-tauri-drag-region className="flex items-center gap-2">
						<img
							src="/ingot.svg"
							alt={t("shell.logoAlt")}
							draggable={false}
							className="pointer-events-none size-4.5 object-contain"
						/>
						<span className="pointer-events-none text-[12px] font-semibold tracking-tight text-foreground/85">
							Ingot
						</span>
					</div>
					<WindowControlButtons />
				</header>
			) : null}

			<div className="relative z-10 flex min-h-0 flex-1 flex-col overflow-hidden">
				<AnimatePresence mode="wait">
					{isWelcomeStep ? (
						<motion.div
							key="onboarding-welcome-stage"
							initial={{ opacity: 0, scale: prefersReducedMotion ? 1 : 0.98 }}
							animate={{ opacity: 1, scale: 1 }}
							exit={{ opacity: 0, scale: prefersReducedMotion ? 1 : 0.97 }}
							transition={{ duration: 0.22, ease: ONBOARDING_EASE }}
							className="custom-scrollbar flex min-h-0 w-full flex-1 items-center justify-center overflow-x-hidden overflow-y-auto"
						>
							<WelcomeLogoStep onStart={handleNext} onSkip={handleFinish} />
						</motion.div>
					) : (
						<motion.div
							key="onboarding-setup-stage"
							initial={{ opacity: 0, y: prefersReducedMotion ? 0 : 10 }}
							animate={{ opacity: 1, y: 0 }}
							exit={{ opacity: 0, y: prefersReducedMotion ? 0 : -10 }}
							transition={{ duration: 0.22, ease: ONBOARDING_EASE }}
							className="flex min-h-0 w-full flex-1 flex-col"
						>
							<OnboardingStepHeader
								currentStep={currentStep}
								stepIndex={stepIndex}
								isMobile={isMobile}
								onSelectStep={handleSelectStep}
								onSkip={handleFinish}
							/>

							<div className="custom-scrollbar flex min-h-0 w-full flex-1 scroll-fade-y flex-col overflow-x-hidden overflow-y-auto px-4 py-3 sm:px-6 sm:py-4">
								<div className="mx-auto my-auto w-full max-w-3xl">
									<AnimatePresence mode="wait" initial={false} custom={direction}>
										<motion.div
											key={currentStep}
											custom={direction}
											variants={prefersReducedMotion ? undefined : stepSlideVariants}
											initial={prefersReducedMotion ? { opacity: 0 } : "enter"}
											animate={prefersReducedMotion ? { opacity: 1 } : "center"}
											exit={prefersReducedMotion ? { opacity: 0 } : "exit"}
											transition={{ duration: 0.2, ease: ONBOARDING_EASE }}
										>
											{renderSetupStepContent()}
										</motion.div>
									</AnimatePresence>
								</div>
							</div>

							<OnboardingStepFooter
								isLastStep={isLastStep}
								isMobile={isMobile}
								onBack={handleBack}
								onNext={handleNext}
							/>
						</motion.div>
					)}
				</AnimatePresence>
			</div>
		</motion.div>
	)
}

export default OnboardingFlow
