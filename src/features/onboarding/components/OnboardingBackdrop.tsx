import { AnimatePresence, useReducedMotion } from "motion/react"
import { useCallback, useEffect, useRef } from "react"
import {
	FLOATING_TILES,
	type PointerListener,
	type PointerPoint,
} from "@/features/onboarding/backdrop/floatingTilesConfig"
import MagneticTile from "@/features/onboarding/backdrop/MagneticTile"
import VoxelAmbientLayer from "@/features/onboarding/components/VoxelAmbientLayer"
import { type OnboardingStepId } from "@/features/onboarding/onboardingSteps"

interface OnboardingBackdropProps {
	currentStep: OnboardingStepId
}

const OnboardingBackdrop = ({ currentStep }: OnboardingBackdropProps) => {
	const prefersReducedMotion = Boolean(useReducedMotion())
	const listenersRef = useRef<Set<PointerListener>>(new Set())

	const registerPointerListener = useCallback((listener: PointerListener) => {
		listenersRef.current.add(listener)
		return () => {
			listenersRef.current.delete(listener)
		}
	}, [])

	useEffect(() => {
		if (prefersReducedMotion) {
			return
		}

		const notifyAll = (point: PointerPoint | null) => {
			for (const listener of listenersRef.current) {
				listener(point)
			}
		}

		const handlePointerMove = (event: PointerEvent) => {
			if (event.pointerType === "touch") {
				return
			}
			notifyAll({ x: event.clientX, y: event.clientY })
		}

		const handleReset = () => {
			notifyAll(null)
		}

		window.addEventListener("pointermove", handlePointerMove, { passive: true })
		document.documentElement.addEventListener("pointerleave", handleReset)
		window.addEventListener("blur", handleReset)

		return () => {
			window.removeEventListener("pointermove", handlePointerMove)
			document.documentElement.removeEventListener("pointerleave", handleReset)
			window.removeEventListener("blur", handleReset)
		}
	}, [prefersReducedMotion])

	const isWelcomeStep = currentStep === "welcome"

	return (
		<div aria-hidden="true" className="pointer-events-none absolute inset-0 overflow-hidden">
			<div
				className="absolute inset-0 opacity-45"
				style={{
					backgroundImage:
						"radial-gradient(color-mix(in oklab, var(--color-foreground) 18%, transparent) 1px, transparent 1px)",
					backgroundSize: "28px 28px",
					maskImage: "radial-gradient(ellipse 75% 70% at 50% 45%, black 35%, transparent 100%)",
				}}
			/>

			<div className="absolute -top-24 left-1/2 h-96 w-176 -translate-x-1/2 rounded-full bg-primary/14 blur-3xl" />
			<div className="absolute -bottom-32 -left-24 size-96 rounded-full bg-amber-500/8 blur-3xl" />
			<div className="absolute -right-24 -bottom-28 size-96 rounded-full bg-primary/10 blur-3xl" />

			<VoxelAmbientLayer
				prefersReducedMotion={prefersReducedMotion}
				showFloatingCubes={isWelcomeStep}
			/>

			<AnimatePresence>
				{isWelcomeStep
					? FLOATING_TILES.map((tile) => {
							return (
								<MagneticTile
									key={tile.id}
									tile={tile}
									isHighlighted={tile.highlightSteps.includes(currentStep)}
									prefersReducedMotion={prefersReducedMotion}
									registerPointerListener={registerPointerListener}
								/>
							)
						})
					: null}
			</AnimatePresence>
		</div>
	)
}

export default OnboardingBackdrop
