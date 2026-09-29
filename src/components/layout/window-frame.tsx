import { useRouter } from "@tanstack/react-router"
import { memo, type ReactNode, useEffect } from "react"
import Titlebar from "@/components/layout/titlebar"

interface WindowFrameProps {
	title?: string
	children: ReactNode
}

const MOUSE_BACK = 3
const MOUSE_FORWARD = 4

/** F5, Ctrl+F5, Ctrl+R, Ctrl+Shift+R (Cmd on macOS) */
const isReloadShortcut = (e: KeyboardEvent) => {
	if (e.key === "F5") return true

	const hasModifier = e.ctrlKey || e.metaKey
	return hasModifier && e.key.toLowerCase() === "r"
}

const TEXT_FIELD = "input, textarea, [contenteditable]:not([contenteditable=false])"

/** Places where the webview's own context menu is still useful: text fields and selected text */
const wantsNativeContextMenu = (target: EventTarget | null) => {
	if (target instanceof Element && target.closest(TEXT_FIELD)) return true

	return !!window.getSelection()?.toString()
}

const WindowFrame = ({ title = "Ingot", children }: WindowFrameProps) => {
	const router = useRouter()

	useEffect(() => {
		const handleMouseUp = (e: MouseEvent) => {
			if (e.button === MOUSE_BACK) {
				e.preventDefault()
				router.history.back()
			} else if (e.button === MOUSE_FORWARD) {
				e.preventDefault()
				router.history.forward()
			}
		}

		const handleKeyDown = (e: KeyboardEvent) => {
			// A reload drops all in-memory state; dev builds keep it for quick refreshes
			if (isReloadShortcut(e)) {
				if (!import.meta.env.DEV) e.preventDefault()
			} else if (e.altKey && e.key === "ArrowLeft") {
				e.preventDefault()
				router.history.back()
			} else if (e.altKey && e.key === "ArrowRight") {
				e.preventDefault()
				router.history.forward()
			}
		}

		// Long-pressing on a touchscreen opens the webview's Back/Reload/Print menu.
		// Dev builds keep it for Inspect.
		const handleContextMenu = (e: MouseEvent) => {
			if (import.meta.env.DEV) return
			if (e.defaultPrevented) return
			if (wantsNativeContextMenu(e.target)) return

			e.preventDefault()
		}

		window.addEventListener("mouseup", handleMouseUp)
		window.addEventListener("keydown", handleKeyDown)
		window.addEventListener("contextmenu", handleContextMenu)

		return () => {
			window.removeEventListener("mouseup", handleMouseUp)
			window.removeEventListener("keydown", handleKeyDown)
			window.removeEventListener("contextmenu", handleContextMenu)
		}
	}, [router])

	return (
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-card/60 via-background to-background text-foreground antialiased selection:bg-primary/20 dark:to-black">
			<Titlebar title={title} />
			<div className="flex flex-1 overflow-hidden">{children}</div>
		</div>
	)
}

WindowFrame.displayName = "WindowFrame"

export default memo(WindowFrame)
