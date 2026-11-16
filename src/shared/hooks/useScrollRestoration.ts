import { useLayoutEffect, type RefObject } from "react"

const scrollPositionCache = new Map<string, number>()

export const useScrollRestoration = (
	scrollRef: RefObject<HTMLElement | null>,
	routeKey: string,
): void => {
	useLayoutEffect(() => {
		const element = scrollRef.current
		if (!element) {
			return
		}

		const savedTop = scrollPositionCache.get(routeKey) ?? 0
		element.scrollTop = savedTop

		return () => {
			scrollPositionCache.set(routeKey, element.scrollTop)
		}
	}, [routeKey, scrollRef])
}
