import {
	Blocks,
	Camera,
	Compass,
	Cpu,
	Gamepad2,
	Globe2,
	Languages,
	Layers,
	Package,
	Palette,
	Server,
	Shirt,
	Sparkles,
	type LucideIcon,
} from "lucide-react"
import { type OnboardingStepId } from "@/features/onboarding/onboardingSteps"

export interface FloatingIconTile {
	id: string
	icon: LucideIcon
	positionClass: string
	sizeClass: string
	iconSizeClass: string
	rotate: number
	driftY: number
	duration: number
	delay: number
	tintClass: string
	highlightSteps: readonly OnboardingStepId[]
	visibilityClass: string
}

export interface PointerPoint {
	x: number
	y: number
}

export type PointerListener = (point: PointerPoint | null) => void

export interface MagneticTarget {
	x: number
	y: number
	scale: number
}

export const MAGNETIC_RADIUS_PX = 200
export const MAGNETIC_STRENGTH = 0.3
export const MAGNETIC_MAX_OFFSET_PX = 28

export const MAGNETIC_SPRING_CONFIG = {
	stiffness: 170,
	damping: 18,
	mass: 0.65,
}

export const RESTING_MAGNETIC_TARGET: MagneticTarget = {
	x: 0,
	y: 0,
	scale: 1,
}

const clampOffset = (value: number): number => {
	return Math.min(MAGNETIC_MAX_OFFSET_PX, Math.max(-MAGNETIC_MAX_OFFSET_PX, value))
}

export const computeMagneticTarget = (
	rect: DOMRect,
	point: PointerPoint | null,
): MagneticTarget => {
	if (!point || rect.width === 0 || rect.height === 0) {
		return RESTING_MAGNETIC_TARGET
	}

	const centerX = rect.left + rect.width / 2
	const centerY = rect.top + rect.height / 2
	const dx = point.x - centerX
	const dy = point.y - centerY
	const distance = Math.hypot(dx, dy)

	if (distance >= MAGNETIC_RADIUS_PX) {
		return RESTING_MAGNETIC_TARGET
	}

	const falloff = 1 - distance / MAGNETIC_RADIUS_PX
	const pull = falloff * MAGNETIC_STRENGTH

	return {
		x: clampOffset(dx * pull),
		y: clampOffset(dy * pull),
		scale: 1 + falloff * 0.14,
	}
}

export const FLOATING_TILES: readonly FloatingIconTile[] = [
	{
		id: "gamepad",
		icon: Gamepad2,
		positionClass: "top-[14%] left-[7%]",
		sizeClass: "size-14 rounded-2xl",
		iconSizeClass: "size-6",
		rotate: -10,
		driftY: 10,
		duration: 7.2,
		delay: 0,
		tintClass: "text-primary",
		highlightSteps: ["welcome", "gameplay"],
		visibilityClass: "hidden md:flex",
	},
	{
		id: "package",
		icon: Package,
		positionClass: "top-[42%] left-[4%]",
		sizeClass: "size-12 rounded-2xl",
		iconSizeClass: "size-5",
		rotate: 8,
		driftY: 12,
		duration: 8.4,
		delay: 0.4,
		tintClass: "text-amber-400",
		highlightSteps: ["welcome", "gameplay"],
		visibilityClass: "hidden lg:flex",
	},
	{
		id: "globe",
		icon: Globe2,
		positionClass: "bottom-[20%] left-[9%]",
		sizeClass: "size-13 rounded-2xl",
		iconSizeClass: "size-5.5",
		rotate: -6,
		driftY: 9,
		duration: 7.8,
		delay: 0.9,
		tintClass: "text-sky-400",
		highlightSteps: ["welcome", "language"],
		visibilityClass: "hidden md:flex",
	},
	{
		id: "languages",
		icon: Languages,
		positionClass: "top-[27%] left-[16%]",
		sizeClass: "size-10 rounded-xl",
		iconSizeClass: "size-4.5",
		rotate: 12,
		driftY: 8,
		duration: 6.8,
		delay: 0.6,
		tintClass: "text-indigo-400",
		highlightSteps: ["language"],
		visibilityClass: "hidden xl:flex",
	},
	{
		id: "blocks",
		icon: Blocks,
		positionClass: "bottom-[11%] left-[20%]",
		sizeClass: "size-11 rounded-xl",
		iconSizeClass: "size-4.5",
		rotate: 9,
		driftY: 11,
		duration: 8.1,
		delay: 1.1,
		tintClass: "text-emerald-400",
		highlightSteps: ["welcome", "appearance"],
		visibilityClass: "hidden xl:flex",
	},
	{
		id: "palette",
		icon: Palette,
		positionClass: "top-[15%] right-[8%]",
		sizeClass: "size-14 rounded-2xl",
		iconSizeClass: "size-6",
		rotate: 10,
		driftY: 11,
		duration: 7.5,
		delay: 0.2,
		tintClass: "text-fuchsia-400",
		highlightSteps: ["welcome", "appearance"],
		visibilityClass: "hidden md:flex",
	},
	{
		id: "server",
		icon: Server,
		positionClass: "top-[44%] right-[4.5%]",
		sizeClass: "size-12 rounded-2xl",
		iconSizeClass: "size-5",
		rotate: -8,
		driftY: 10,
		duration: 8.6,
		delay: 0.7,
		tintClass: "text-cyan-400",
		highlightSteps: ["welcome", "gameplay"],
		visibilityClass: "hidden lg:flex",
	},
	{
		id: "cpu",
		icon: Cpu,
		positionClass: "bottom-[19%] right-[8.5%]",
		sizeClass: "size-13 rounded-2xl",
		iconSizeClass: "size-5.5",
		rotate: 7,
		driftY: 9,
		duration: 7.4,
		delay: 0.5,
		tintClass: "text-primary",
		highlightSteps: ["gameplay"],
		visibilityClass: "hidden md:flex",
	},
	{
		id: "sparkles",
		icon: Sparkles,
		positionClass: "top-[28%] right-[16.5%]",
		sizeClass: "size-10 rounded-xl",
		iconSizeClass: "size-4.5",
		rotate: -12,
		driftY: 8,
		duration: 6.9,
		delay: 1.0,
		tintClass: "text-amber-300",
		highlightSteps: ["welcome", "appearance"],
		visibilityClass: "hidden xl:flex",
	},
	{
		id: "shirt",
		icon: Shirt,
		positionClass: "bottom-[11%] right-[19.5%]",
		sizeClass: "size-11 rounded-xl",
		iconSizeClass: "size-4.5",
		rotate: -7,
		driftY: 10,
		duration: 8.0,
		delay: 0.3,
		tintClass: "text-rose-400",
		highlightSteps: ["appearance"],
		visibilityClass: "hidden xl:flex",
	},
	{
		id: "compass",
		icon: Compass,
		positionClass: "top-[8%] left-[25%]",
		sizeClass: "size-9 rounded-xl",
		iconSizeClass: "size-4",
		rotate: -14,
		driftY: 7,
		duration: 7.6,
		delay: 0.8,
		tintClass: "text-teal-400",
		highlightSteps: ["language"],
		visibilityClass: "hidden 2xl:flex",
	},
	{
		id: "camera",
		icon: Camera,
		positionClass: "top-[9%] right-[25%]",
		sizeClass: "size-9 rounded-xl",
		iconSizeClass: "size-4",
		rotate: 13,
		driftY: 7,
		duration: 7.9,
		delay: 1.2,
		tintClass: "text-violet-400",
		highlightSteps: ["appearance"],
		visibilityClass: "hidden 2xl:flex",
	},
	{
		id: "layers",
		icon: Layers,
		positionClass: "bottom-[7%] left-[44%]",
		sizeClass: "size-9 rounded-xl",
		iconSizeClass: "size-4",
		rotate: 6,
		driftY: 6,
		duration: 8.3,
		delay: 0.6,
		tintClass: "text-primary",
		highlightSteps: ["welcome", "appearance"],
		visibilityClass: "hidden lg:flex",
	},
]
