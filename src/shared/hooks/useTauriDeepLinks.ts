import { useEffect } from "react"
import {
	isTauriRuntime,
	shouldShowWindowOnStartup,
	showAndFocusMainWindow,
} from "@/shared/lib/tauriCommands"
import { subscribeDeepLinkUrls } from "@/shared/lib/tauriEvents"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

interface UseTauriDeepLinksOptions {
	onPreloadRoutes: () => Promise<void>
	onNavigatePath: (path: string) => void
}

export const parseIngotDeepLink = (rawUrl: string): string | null => {
	try {
		const parsed = new URL(rawUrl)
		const targetPath = `/${parsed.host}${parsed.pathname}`.replace(/\/+$/, "") || "/"
		if (parsed.search) {
			return `${targetPath}${parsed.search}`
		}

		return targetPath
	} catch (error: unknown) {
		console.error("[Ingot] Invalid deep-link URL:", rawUrl, error)
		return null
	}
}

export const useTauriDeepLinks = ({
	onPreloadRoutes,
	onNavigatePath,
}: UseTauriDeepLinksOptions): void => {
	useEffect(() => {
		let cancelled = false
		let revealFrameId: number | null = null
		let unlistenDeepLink: (() => void) | undefined

		const handleContextMenu = (event: MouseEvent) => {
			if (!useLauncherPrefsStore.getState().enableDevContextMenu) {
				event.preventDefault()
			}
		}

		document.addEventListener("contextmenu", handleContextMenu)

		const initDesktopRuntime = async () => {
			try {
				await onPreloadRoutes()
			} catch (error: unknown) {
				console.error("[Ingot] Initial route load failed:", error)
			}

			if (cancelled) {
				return
			}

			const shouldShow = await shouldShowWindowOnStartup()
			if (shouldShow && !cancelled) {
				revealFrameId = window.requestAnimationFrame(() => {
					if (cancelled) {
						return
					}
					void showAndFocusMainWindow().catch((error: unknown) => {
						console.error("[Ingot] Failed to show main window:", error)
					})
				})
			}

			try {
				const dispose = await subscribeDeepLinkUrls((urls) => {
					for (const rawUrl of urls) {
						const nextLocation = parseIngotDeepLink(rawUrl)
						if (nextLocation) {
							onNavigatePath(nextLocation)
						}
					}
				})
				if (cancelled) {
					dispose()
				} else {
					unlistenDeepLink = dispose
				}
			} catch (error: unknown) {
				console.error("[Ingot] Failed to bind deep-link listener:", error)
			}
		}

		if (isTauriRuntime()) {
			void initDesktopRuntime()
		}

		return () => {
			cancelled = true
			document.removeEventListener("contextmenu", handleContextMenu)
			if (revealFrameId !== null) {
				window.cancelAnimationFrame(revealFrameId)
			}
			if (unlistenDeepLink) {
				unlistenDeepLink()
			}
		}
	}, [onNavigatePath, onPreloadRoutes])
}
