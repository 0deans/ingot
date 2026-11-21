import { AnimatePresence, motion } from "motion/react"
import {
	ISOMETRIC_CUBES,
	PIXEL_EMBERS,
	PIXEL_STARS,
	VOXEL_CLOUDS,
} from "@/features/onboarding/backdrop/voxelSceneConfig"
import { ONBOARDING_EASE } from "@/features/onboarding/onboardingSteps"
import { cn } from "@/shared/lib/utils"

interface VoxelAmbientLayerProps {
	prefersReducedMotion: boolean
	showFloatingCubes: boolean
}

const VoxelAmbientLayer = ({ prefersReducedMotion, showFloatingCubes }: VoxelAmbientLayerProps) => {
	return (
		<>
			{PIXEL_STARS.map((star) => {
				return (
					<motion.span
						key={star.id}
						style={{ left: star.leftPercent, top: star.topPercent }}
						initial={{ opacity: 0.2 }}
						animate={prefersReducedMotion ? { opacity: 0.35 } : { opacity: [0.15, 0.72, 0.15] }}
						transition={
							prefersReducedMotion
								? undefined
								: {
										duration: star.duration,
										repeat: Number.POSITIVE_INFINITY,
										ease: "easeInOut",
										delay: star.delay,
									}
						}
						className={cn("absolute rounded-[1px] bg-foreground/75", star.sizeClass)}
					/>
				)
			})}

			<div className="absolute top-[8%] right-[24%] hidden sm:block">
				<div className="relative flex size-8 items-center justify-center">
					<span className="absolute inset-0 rounded-sm bg-amber-200/18 blur-md" />
					<span className="relative block size-5 rounded-xs border border-foreground/20 bg-foreground/12 shadow-[inset_2px_2px_0_rgba(255,255,255,0.2)]" />
				</div>
			</div>

			{VOXEL_CLOUDS.map((cloud) => {
				return (
					<motion.svg
						key={cloud.id}
						viewBox="0 0 160 36"
						fill="none"
						style={{ left: cloud.startX }}
						animate={
							prefersReducedMotion
								? undefined
								: {
										x: [0, cloud.driftX, 0],
									}
						}
						transition={
							prefersReducedMotion
								? undefined
								: {
										duration: cloud.duration,
										repeat: Number.POSITIVE_INFINITY,
										ease: "easeInOut",
										delay: cloud.delay,
									}
						}
						className={cn("absolute h-auto", cloud.topClass, cloud.widthClass, cloud.opacityClass)}
					>
						<rect x="24" y="6" width="84" height="14" fill="var(--color-foreground)" />
						<rect x="44" y="0" width="44" height="8" fill="var(--color-foreground)" />
						<rect x="8" y="16" width="144" height="14" fill="var(--color-foreground)" />
						<rect
							x="32"
							y="26"
							width="96"
							height="8"
							fill="var(--color-foreground)"
							fillOpacity="0.6"
						/>
					</motion.svg>
				)
			})}

			<AnimatePresence>
				{showFloatingCubes
					? ISOMETRIC_CUBES.map((cube) => {
							return (
								<motion.svg
									key={cube.id}
									width={cube.size}
									height={cube.size}
									viewBox="0 0 40 40"
									fill="none"
									initial={{ opacity: 0, scale: 0.85 }}
									animate={
										prefersReducedMotion
											? { opacity: 0.7, scale: 1 }
											: {
													opacity: 0.75,
													scale: 1,
													y: [0, -cube.driftY, 0],
												}
									}
									exit={{
										opacity: 0,
										scale: 0.85,
										transition: { duration: 0.26, ease: ONBOARDING_EASE },
									}}
									transition={
										prefersReducedMotion
											? { duration: 0.25 }
											: {
													opacity: { duration: 0.45, ease: ONBOARDING_EASE },
													scale: { duration: 0.45, ease: ONBOARDING_EASE },
													y: {
														duration: cube.duration,
														repeat: Number.POSITIVE_INFINITY,
														ease: "easeInOut",
														delay: cube.delay,
													},
												}
									}
									className={cn("absolute", cube.positionClass, cube.visibilityClass)}
								>
									<polygon
										points="20,4 35,12.5 20,21 5,12.5"
										fill={cube.topFill}
										stroke={cube.strokeColor}
										strokeWidth="1"
										strokeLinejoin="round"
									/>
									<polygon
										points="5,12.5 20,21 20,36 5,27.5"
										fill={cube.leftFill}
										stroke={cube.strokeColor}
										strokeWidth="1"
										strokeLinejoin="round"
									/>
									<polygon
										points="35,12.5 20,21 20,36 35,27.5"
										fill={cube.rightFill}
										stroke={cube.strokeColor}
										strokeWidth="1"
										strokeLinejoin="round"
									/>
								</motion.svg>
							)
						})
					: null}
			</AnimatePresence>

			{!prefersReducedMotion
				? PIXEL_EMBERS.map((ember) => {
						return (
							<motion.span
								key={ember.id}
								style={{
									left: ember.leftPercent,
									bottom: ember.bottomPercent,
								}}
								initial={{ opacity: 0, y: 0 }}
								animate={{
									opacity: [0, 0.85, 0],
									y: [0, -ember.riseY],
								}}
								transition={{
									duration: ember.duration,
									repeat: Number.POSITIVE_INFINITY,
									ease: "easeOut",
									delay: ember.delay,
								}}
								className={cn("absolute rounded-[1px]", ember.sizeClass, ember.colorClass)}
							/>
						)
					})
				: null}

			<svg
				viewBox="0 0 1440 160"
				preserveAspectRatio="none"
				className="absolute inset-x-0 bottom-0 h-24 w-full opacity-35 sm:h-32"
			>
				<defs>
					<linearGradient id="voxel-horizon-back" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0%" stopColor="var(--color-primary)" stopOpacity="0.22" />
						<stop offset="100%" stopColor="var(--color-primary)" stopOpacity="0" />
					</linearGradient>
					<linearGradient id="voxel-horizon-front" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0%" stopColor="var(--color-foreground)" stopOpacity="0.14" />
						<stop offset="100%" stopColor="var(--color-foreground)" stopOpacity="0" />
					</linearGradient>
				</defs>

				<path
					d="M0,160 L0,96 L60,96 L60,72 L140,72 L140,88 L220,88 L220,56 L300,56 L300,80 L380,80 L380,104 L480,104 L480,120 L580,120 L580,108 L680,108 L680,128 L780,128 L780,112 L880,112 L880,92 L960,92 L960,104 L1060,104 L1060,64 L1160,64 L1160,84 L1260,84 L1260,52 L1340,52 L1340,76 L1440,76 L1440,160 Z"
					fill="url(#voxel-horizon-back)"
				/>
				<path
					d="M0,160 L0,116 L90,116 L90,100 L170,100 L170,124 L270,124 L270,108 L350,108 L350,132 L490,132 L490,140 L630,140 L630,134 L790,134 L790,142 L930,142 L930,124 L1030,124 L1030,108 L1130,108 L1130,120 L1240,120 L1240,96 L1350,96 L1350,112 L1440,112 L1440,160 Z"
					fill="url(#voxel-horizon-front)"
				/>
			</svg>
		</>
	)
}

export default VoxelAmbientLayer
