import { useRouterState } from "@tanstack/react-router"
import { useEffect, useLayoutEffect, useRef } from "react"
import { getTopLevelSection } from "./tab-history"

const scrollPositions = new Map<string, number>()

export function getScrollKey(pathname: string, search = ""): string {
	const section = getTopLevelSection(pathname)
	if (section === "/skins") {
		const params = new URLSearchParams(search)
		const tab = params.get("tab") ?? "catalog"
		return `/skins:${tab}`
	}
	return section
}

export function resetScroll(sectionOrKey: string): void {
	const key = getScrollKey(sectionOrKey)
	scrollPositions.delete(key)
	for (const k of scrollPositions.keys()) {
		if (k === key || k.startsWith(`${key}:`)) {
			scrollPositions.delete(k)
		}
	}
}

/**
 * Automatically captures and restores element scroll positions for pages using ScrollArea
 * across sidebar tab navigation and history navigation.
 */
export function useScrollRestoration() {
	const mainRef = useRef<HTMLElement | null>(null)
	const pathname = useRouterState({ select: (s) => s.location.pathname })
	const searchStr = useRouterState({ select: (s) => s.location.searchStr })
	const currentKey = getScrollKey(pathname, searchStr)
	const currentKeyRef = useRef(currentKey)
	const isRestoringRef = useRef(false)

	currentKeyRef.current = currentKey

	// Capture scroll position on user scroll
	useEffect(() => {
		const main = mainRef.current
		if (!main) return

		const handleScroll = (e: Event) => {
			if (isRestoringRef.current) return
			const target = e.target
			if (!(target instanceof HTMLElement)) return

			if (
				target.getAttribute("data-slot") === "scroll-area-viewport" ||
				target.classList.contains("overflow-y-auto")
			) {
				// Prevent recording 0 when an element collapses on unmount
				if (target.scrollTop === 0 && target.scrollHeight <= target.clientHeight) {
					return
				}
				scrollPositions.set(currentKeyRef.current, target.scrollTop)
			}
		}

		main.addEventListener("scroll", handleScroll, { capture: true, passive: true })
		return () => {
			main.removeEventListener("scroll", handleScroll, { capture: true })
		}
	}, [])

	// Restore scroll position when section / key changes
	useLayoutEffect(() => {
		const main = mainRef.current
		if (!main) return

		const targetScroll = scrollPositions.get(currentKey) ?? 0

		// If nothing was saved or scroll was at top, let page render naturally
		if (targetScroll <= 0) {
			return
		}

		let cancelled = false
		isRestoringRef.current = true

		let ro: ResizeObserver | null = null
		let observedChild: HTMLElement | null = null
		let timeoutId: number | null = null
		let rafId: number | null = null

		const cleanup = () => {
			cancelled = true
			isRestoringRef.current = false
			if (ro) {
				ro.disconnect()
				ro = null
			}
			observedChild = null
			if (timeoutId !== null) {
				clearTimeout(timeoutId)
				timeoutId = null
			}
			if (rafId !== null) {
				cancelAnimationFrame(rafId)
				rafId = null
			}
			main.removeEventListener("wheel", handleUserInteraction, { capture: true })
			main.removeEventListener("touchmove", handleUserInteraction, { capture: true })
		}

		const handleUserInteraction = () => {
			cleanup()
		}

		main.addEventListener("wheel", handleUserInteraction, { capture: true, passive: true })
		main.addEventListener("touchmove", handleUserInteraction, { capture: true, passive: true })

		let attempts = 0
		const maxAttempts = 90 // ~1.5s of frames to wait for data/query render

		const attemptRestore = () => {
			if (cancelled) return

			const found =
				main.querySelector<HTMLElement>('[data-slot="scroll-area-viewport"]') ??
				main.querySelector<HTMLElement>(".overflow-y-auto")

			if (!found) {
				attempts++
				if (attempts < maxAttempts) {
					rafId = requestAnimationFrame(attemptRestore)
				} else {
					cleanup()
				}
				return
			}

			const viewport = found

			viewport.scrollTop = targetScroll
			if (Math.abs(viewport.scrollTop - targetScroll) <= 2) {
				cleanup()
				return
			}

			// Track child growth via ResizeObserver in case data is still populating
			const contentChild = viewport.firstElementChild
			if (contentChild instanceof HTMLElement) {
				if (observedChild !== contentChild) {
					if (ro) ro.disconnect()
					observedChild = contentChild
					ro = new ResizeObserver(() => {
						if (cancelled) return
						viewport.scrollTop = targetScroll
						if (Math.abs(viewport.scrollTop - targetScroll) <= 2) {
							cleanup()
						}
					})
					ro.observe(contentChild)
				}
			}

			attempts++
			if (attempts < maxAttempts) {
				rafId = requestAnimationFrame(attemptRestore)
			} else {
				cleanup()
			}
		}

		rafId = requestAnimationFrame(attemptRestore)
		timeoutId = window.setTimeout(cleanup, 2000)

		return () => {
			cleanup()
		}
	}, [currentKey])

	return mainRef
}
