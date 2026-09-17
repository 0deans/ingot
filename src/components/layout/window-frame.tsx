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

		window.addEventListener("mouseup", handleMouseUp)
		window.addEventListener("keydown", handleKeyDown)

		return () => {
			window.removeEventListener("mouseup", handleMouseUp)
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [router])

	return (
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-zinc-900/60 via-zinc-950 to-black text-foreground antialiased selection:bg-primary/20">
			<Titlebar title={title} />
			<div className="flex flex-1 overflow-hidden">{children}</div>
		</div>
	)
}

WindowFrame.displayName = "WindowFrame"

export default memo(WindowFrame)
