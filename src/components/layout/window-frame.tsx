import { useRouter } from "@tanstack/react-router"
import { memo, type ReactNode, useEffect } from "react"
import Titlebar from "@/components/layout/titlebar"

interface WindowFrameProps {
	title?: string
	children: ReactNode
}

const WindowFrame = ({ title = "Ingot", children }: WindowFrameProps) => {
	const router = useRouter()

	useEffect(() => {
		const handleMouseUp = (e: MouseEvent) => {
			// Mouse back button (button 3)
			if (e.button === 3) {
				e.preventDefault()
				router.history.back()
			}
			// Mouse forward button (button 4)
			else if (e.button === 4) {
				e.preventDefault()
				router.history.forward()
			}
		}

		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.altKey && e.key === "ArrowLeft") {
				e.preventDefault()
				router.history.back()
			} else if (e.altKey && e.key === "ArrowRight") {
				e.preventDefault()
				router.history.forward()
			}
		}

		// Long-pressing on a touchscreen opens the webview's own Back/Reload/Print menu; keep it
		// only where it's useful (text fields and selected text). Dev builds keep it for Inspect.
		const handleContextMenu = (e: MouseEvent) => {
			if (import.meta.env.DEV || e.defaultPrevented) return
			const target = e.target as HTMLElement | null
			if (target?.closest("input, textarea, [contenteditable]:not([contenteditable=false])")) return
			if (window.getSelection()?.toString()) return
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
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-zinc-900/60 via-zinc-950 to-background text-foreground antialiased selection:bg-primary/20 dark:to-black">
			<Titlebar title={title} />
			<div className="flex flex-1 overflow-hidden">{children}</div>
		</div>
	)
}

WindowFrame.displayName = "WindowFrame"

export default memo(WindowFrame)
