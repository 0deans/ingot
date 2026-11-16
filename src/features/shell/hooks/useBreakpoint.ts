import { useEffect, useState } from "react"

export type BreakpointTier = "mobile" | "tablet" | "desktop"

const resolveBreakpoint = (width: number): BreakpointTier => {
	if (width < 640) {
		return "mobile"
	}
	if (width < 960) {
		return "tablet"
	}
	return "desktop"
}

export const useBreakpoint = (): BreakpointTier => {
	const [tier, setTier] = useState<BreakpointTier>(() => {
		if (typeof window === "undefined") {
			return "desktop"
		}
		return resolveBreakpoint(window.innerWidth)
	})

	useEffect(() => {
		const handleResize = () => {
			setTier(resolveBreakpoint(window.innerWidth))
		}
		handleResize()
		window.addEventListener("resize", handleResize)
		return () => {
			window.removeEventListener("resize", handleResize)
		}
	}, [])

	return tier
}
