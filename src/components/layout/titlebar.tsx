import { PhysicalPosition } from "@tauri-apps/api/dpi"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { Copy, Minus, Square, X } from "lucide-react"
import { memo, useEffect, useRef, useState } from "react"

/** Finger travel (CSS px) before a touch on the titlebar turns into a window drag */
const DRAG_THRESHOLD = 6
const DOUBLE_TAP_MS = 300

interface TouchDrag {
	pointerId: number
	startX: number
	startY: number
	/** Window position (physical px) when the drag started; null until it has been read */
	origin: { x: number; y: number } | null
	dragging: boolean
}

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

	// Tauri's data-tauri-drag-region only reacts to `mousedown`, which touch and pen input
	// don't fire until the finger is lifted, so move the window by hand for those.
	const touchDrag = useRef<TouchDrag | null>(null)
	const lastTap = useRef(0)
	const pendingPosition = useRef<{ x: number; y: number } | null>(null)
	const moving = useRef(false)

	/** Applies the latest requested position, never queueing more than one IPC call */
	const flushPosition = async () => {
		if (moving.current) return
		moving.current = true
		try {
			while (pendingPosition.current) {
				const { x, y } = pendingPosition.current
				pendingPosition.current = null
				await getCurrentWindow().setPosition(new PhysicalPosition(x, y))
			}
		} catch (error) {
			console.error("Failed to move window:", error)
		} finally {
			moving.current = false
		}
	}

	const beginTouchDrag = async (drag: TouchDrag, clientX: number, clientY: number) => {
		const appWindow = getCurrentWindow()
		const innerWidth = window.innerWidth
		if (await appWindow.isMaximized()) {
			// Restore, keeping the finger at the same relative spot along the titlebar
			await appWindow.unmaximize()
			const size = await appWindow.outerSize()
			const ratio = window.devicePixelRatio
			drag.origin = {
				x: Math.round(drag.startX * ratio - size.width * (clientX / innerWidth)),
				y: Math.round((drag.startY - clientY) * ratio),
			}
			pendingPosition.current = drag.origin
			flushPosition()
			return
		}
		const pos = await appWindow.outerPosition()
		drag.origin = { x: pos.x, y: pos.y }
	}

	const onTouchPointerDown = (e: React.PointerEvent<HTMLElement>) => {
		if (!isTauri || e.pointerType === "mouse" || !e.isPrimary) return
		if ((e.target as Element).closest("button, a, input, [role=button]")) return
		// Suppress the compatibility mousedown fired after a tap, which Tauri's drag script
		// would otherwise turn into a second maximize toggle on double-tap
		e.preventDefault()

		const now = performance.now()
		if (now - lastTap.current < DOUBLE_TAP_MS) {
			lastTap.current = 0
			touchDrag.current = null
			handleToggleMaximize()
			return
		}
		lastTap.current = now

		e.currentTarget.setPointerCapture(e.pointerId)
		touchDrag.current = {
			pointerId: e.pointerId,
			startX: e.screenX,
			startY: e.screenY,
			origin: null,
			dragging: false,
		}
	}

	const onTouchPointerMove = (e: React.PointerEvent<HTMLElement>) => {
		const drag = touchDrag.current
		if (!drag || drag.pointerId !== e.pointerId) return
		const dx = e.screenX - drag.startX
		const dy = e.screenY - drag.startY
		if (!drag.dragging) {
			if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return
			drag.dragging = true
			lastTap.current = 0
			beginTouchDrag(drag, e.clientX, e.clientY).catch((error) =>
				console.error("Failed to start window drag:", error),
			)
			return
		}
		if (!drag.origin) return
		const ratio = window.devicePixelRatio
		pendingPosition.current = {
			x: Math.round(drag.origin.x + dx * ratio),
			y: Math.round(drag.origin.y + dy * ratio),
		}
		flushPosition()
	}

	const onTouchPointerUp = (e: React.PointerEvent<HTMLElement>) => {
		if (touchDrag.current?.pointerId === e.pointerId) touchDrag.current = null
	}

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
			onPointerDown={onTouchPointerDown}
			onPointerMove={onTouchPointerMove}
			onPointerUp={onTouchPointerUp}
			onPointerCancel={onTouchPointerUp}
			className="relative z-[100] flex h-10 w-full touch-none select-none items-center justify-between border-border/40 border-b bg-background/85 px-3 backdrop-blur-md"
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
