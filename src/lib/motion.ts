import { useReducedMotion } from "motion/react"
import { useSyncExternalStore } from "react"

export type PageTransitionStyle = "apple" | "fade" | "instant"

const STORAGE_KEY = "ingot_transition_style"
let currentStyle: PageTransitionStyle =
	(typeof localStorage !== "undefined" &&
		(localStorage.getItem(STORAGE_KEY) as PageTransitionStyle)) ||
	"apple"

const listeners = new Set<() => void>()

export function getPageTransitionStyle(): PageTransitionStyle {
	return currentStyle
}

export function setPageTransitionStyle(style: PageTransitionStyle) {
	currentStyle = style
	if (typeof localStorage !== "undefined") {
		localStorage.setItem(STORAGE_KEY, style)
	}
	for (const listener of listeners) {
		listener()
	}
}

export function usePageTransitionStyle(): PageTransitionStyle {
	return useSyncExternalStore(
		(cb) => {
			listeners.add(cb)
			return () => listeners.delete(cb)
		},
		() => currentStyle,
		() => "apple",
	)
}

// Apple-inspired spring physics (SwiftUI .interactiveSpring & .spring)
export const appleSpring = {
	type: "spring",
	stiffness: 350,
	damping: 28,
	mass: 0.8,
} as const

export const springSnappy = {
	type: "spring",
	stiffness: 420,
	damping: 32,
	mass: 0.7,
} as const

export const springGentle = {
	type: "spring",
	stiffness: 280,
	damping: 26,
} as const

// Apple Fluid Easing (decelerates with silky smoothness)
export const appleEase = [0.16, 1, 0.3, 1] as const

/**
 * Variants for Apple macOS / iOS style depth dissolve:
 * - Entering: gentle scale-in from 0.995 to 1.0, opacity from 0 to 1 with fluid decelerate curve.
 * - Perfectly stable along the Y-axis (no jarring elevator bounces).
 */
export const applePageVariants = {
	initial: {
		opacity: 0,
		scale: 0.995,
	},
	animate: {
		opacity: 1,
		scale: 1,
		transition: {
			opacity: { duration: 0.16, ease: [0.25, 0.1, 0.25, 1] },
			scale: { duration: 0.2, ease: appleEase },
		},
	},
} as const

export const fadePageVariants = {
	initial: { opacity: 0 },
	animate: {
		opacity: 1,
		transition: { duration: 0.16, ease: [0.25, 0.1, 0.25, 1] },
	},
} as const

export function useAppMotion() {
	const systemReduced = useReducedMotion()
	const userStyle = usePageTransitionStyle()
	return {
		reducedMotion: Boolean(systemReduced) || userStyle === "instant",
		style: userStyle,
	}
}
