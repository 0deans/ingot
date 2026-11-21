import { motion, useMotionValue, useSpring } from "motion/react"
import { useEffect, useRef } from "react"
import {
	computeMagneticTarget,
	MAGNETIC_SPRING_CONFIG,
	type FloatingIconTile,
	type PointerListener,
} from "@/features/onboarding/backdrop/floatingTilesConfig"
import { ONBOARDING_EASE } from "@/features/onboarding/onboardingSteps"
import { cn } from "@/shared/lib/utils"

interface MagneticTileProps {
	tile: FloatingIconTile
	isHighlighted: boolean
	prefersReducedMotion: boolean
	registerPointerListener: (listener: PointerListener) => () => void
}

const MagneticTile = ({
	tile,
	isHighlighted,
	prefersReducedMotion,
	registerPointerListener,
}: MagneticTileProps) => {
	const tileRef = useRef<HTMLDivElement | null>(null)

	const rawX = useMotionValue(0)
	const rawY = useMotionValue(0)
	const rawScale = useMotionValue(1)
	const magneticX = useSpring(rawX, MAGNETIC_SPRING_CONFIG)
	const magneticY = useSpring(rawY, MAGNETIC_SPRING_CONFIG)
	const magneticScale = useSpring(rawScale, MAGNETIC_SPRING_CONFIG)

	useEffect(() => {
		if (prefersReducedMotion) {
			rawX.set(0)
			rawY.set(0)
			rawScale.set(1)
			return
		}

		return registerPointerListener((point) => {
			const el = tileRef.current
			if (!el) {
				return
			}

			const target = computeMagneticTarget(el.getBoundingClientRect(), point)
			rawX.set(target.x)
			rawY.set(target.y)
			rawScale.set(target.scale)
		})
	}, [prefersReducedMotion, rawScale, rawX, rawY, registerPointerListener])

	const Icon = tile.icon

	return (
		<motion.div
			ref={tileRef}
			initial={{ opacity: 0 }}
			animate={{ opacity: 1 }}
			exit={{
				opacity: 0,
				transition: { duration: 0.26, ease: ONBOARDING_EASE },
			}}
			transition={{ duration: 0.35, ease: ONBOARDING_EASE }}
			style={{ x: magneticX, y: magneticY, scale: magneticScale }}
			className={cn("absolute", tile.positionClass, tile.visibilityClass)}
		>
			<motion.div
				initial={{
					opacity: 0,
					scale: 0.82,
					rotate: tile.rotate,
				}}
				exit={{
					opacity: 0,
					scale: 0.82,
					transition: { duration: 0.26, ease: ONBOARDING_EASE },
				}}
				animate={
					prefersReducedMotion
						? {
								opacity: isHighlighted ? 0.85 : 0.42,
								scale: isHighlighted ? 1.04 : 0.96,
								rotate: tile.rotate,
							}
						: {
								opacity: isHighlighted ? 0.9 : 0.42,
								scale: isHighlighted ? 1.05 : 0.95,
								rotate: [tile.rotate, tile.rotate + 3, tile.rotate],
								y: [0, -tile.driftY, 0],
							}
				}
				transition={
					prefersReducedMotion
						? { duration: 0.25 }
						: {
								opacity: { duration: 0.45 },
								scale: { duration: 0.45 },
								y: {
									duration: tile.duration,
									repeat: Number.POSITIVE_INFINITY,
									ease: "easeInOut",
									delay: tile.delay,
								},
								rotate: {
									duration: tile.duration * 1.15,
									repeat: Number.POSITIVE_INFINITY,
									ease: "easeInOut",
									delay: tile.delay,
								},
							}
				}
				className={cn(
					"flex items-center justify-center border shadow-xl backdrop-blur-md",
					tile.sizeClass,
					isHighlighted
						? "border-primary/35 bg-card/75 ring-1 ring-primary/15"
						: "border-border/60 bg-card/45",
				)}
			>
				<Icon className={cn(tile.iconSizeClass, tile.tintClass)} />
			</motion.div>
		</motion.div>
	)
}

export default MagneticTile
