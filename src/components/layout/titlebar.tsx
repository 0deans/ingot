import { getCurrentWindow } from "@tauri-apps/api/window"
import { Copy, Minus, Square, X } from "lucide-react"
import { memo, useEffect, useState } from "react"

interface TitlebarProps {
	title?: string
}

const Titlebar = ({ title = "Ingot" }: TitlebarProps) => {
	const [isMaximized, setIsMaximized] = useState(false)

	const isTauri =
		typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)

	useEffect(() => {
		if (!isTauri) return

		try {
			const appWindow = getCurrentWindow()
			appWindow.isMaximized().then(setIsMaximized)

			const unlistenPromise = appWindow.onResized(async () => {
				setIsMaximized(await appWindow.isMaximized())
			})

			return () => {
				unlistenPromise.then((unlisten) => unlisten())
			}
		} catch (error) {
			console.warn("Could not attach window listeners:", error)
		}
	}, [isTauri])

	const handleMinimize = async () => {
		if (!isTauri) return
		try {
			await getCurrentWindow().minimize()
		} catch (error) {
			console.error("Failed to minimize window:", error)
		}
	}

	const handleToggleMaximize = async () => {
		if (!isTauri) return
		try {
			const appWindow = getCurrentWindow()
			await appWindow.toggleMaximize()
			setIsMaximized(await appWindow.isMaximized())
		} catch (error) {
			console.error("Failed to toggle maximize window:", error)
		}
	}

	const handleClose = async () => {
		if (!isTauri) return
		try {
			await getCurrentWindow().close()
		} catch (error) {
			console.error("Failed to close window:", error)
		}
	}

	return (
		<header
			data-tauri-drag-region
			className="relative z-[100] flex h-10 w-full select-none items-center justify-between border-border/40 border-b bg-background/85 px-3 backdrop-blur-md"
		>
			{/* Left branding */}
			<div data-tauri-drag-region className="pointer-events-none flex items-center gap-2">
				<img src="/ingot.svg" alt="Ingot Logo" className="size-4.5 rounded-sm object-contain" />
				<span className="font-semibold text-foreground text-xs tracking-wide">{title}</span>
				<span className="rounded-full bg-primary/10 px-1.5 py-0.5 font-medium text-[10px] text-primary">
					v{APP_VERSION}
				</span>
			</div>

			{/* Draggable center region */}
			<div data-tauri-drag-region className="h-full flex-1" />

			{/* Window control buttons */}
			<div className="flex items-center gap-0.5">
				<button
					type="button"
					onClick={handleMinimize}
					aria-label="Minimize"
					className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				>
					<Minus className="size-3.5" />
				</button>

				<button
					type="button"
					onClick={handleToggleMaximize}
					aria-label={isMaximized ? "Restore" : "Maximize"}
					className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				>
					{isMaximized ? <Copy className="size-3" /> : <Square className="size-3" />}
				</button>

				<button
					type="button"
					onClick={handleClose}
					aria-label="Close"
					className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground"
				>
					<X className="size-3.5" />
				</button>
			</div>
		</header>
	)
}

Titlebar.displayName = "Titlebar"

export default memo(Titlebar)
