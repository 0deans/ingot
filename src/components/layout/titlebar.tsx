import { PhysicalPosition } from "@tauri-apps/api/dpi"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { Copy, Minus, Square, X } from "lucide-react"
import { memo, useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"

/** Finger travel (CSS px) before a touch on the titlebar turns into a window drag */
const DRAG_THRESHOLD = 6
const DOUBLE_TAP_MS = 300

interface TouchDrag {
	pointerId: number
	/** Where the finger touched the titlebar (client CSS px); the window keeps it under the finger */
	anchorX: number
	anchorY: number
	/** Window position (physical px) we last moved to; null until it has been read */
	position: { x: number; y: number } | null
	/** Newest finger position (client CSS px) not applied yet */
	latest: { x: number; y: number } | null
	dragging: boolean
	/** A setPosition call is in flight */
	moving: boolean
}

interface TitlebarProps {
	title?: string
}

const Titlebar = ({ title = "Ingot" }: TitlebarProps) => {
	const { t } = useTranslation()
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
	//
	// The finger should stay on the same spot of the titlebar, so each move shifts the window
	// by how far the finger has slid away from that spot (client coordinates). Screen
	// coordinates can't be used: WebView2 derives them from a window position that lags
	// behind while the window moves, which makes the window shake and trail the finger.
	const touchDrag = useRef<TouchDrag | null>(null)
	const lastTap = useRef(0)

	/** Moves the window after the finger, keeping at most one IPC call in flight */
	const pump = async (drag: TouchDrag) => {
		if (drag.moving) return
		drag.moving = true
		try {
			while (drag.latest && drag.position) {
				const ratio = window.devicePixelRatio
				const dx = Math.round((drag.latest.x - drag.anchorX) * ratio)
				const dy = Math.round((drag.latest.y - drag.anchorY) * ratio)
				drag.latest = null
				if (dx === 0 && dy === 0) continue
				drag.position = { x: drag.position.x + dx, y: drag.position.y + dy }
				await getCurrentWindow().setPosition(new PhysicalPosition(drag.position.x, drag.position.y))
			}
		} catch (error) {
			console.error("Failed to move window:", error)
		} finally {
			drag.moving = false
		}
	}

	const beginTouchDrag = async (drag: TouchDrag) => {
		const appWindow = getCurrentWindow()
		if (await appWindow.isMaximized()) {
			// Restore, keeping the finger at the same relative spot along the titlebar
			const ratio = window.devicePixelRatio
			const before = await appWindow.innerPosition()
			const fingerX = before.x + drag.anchorX * ratio
			const fingerY = before.y + drag.anchorY * ratio
			const relativeX = drag.anchorX / window.innerWidth
			await appWindow.unmaximize()
			const [outer, inner, size] = await Promise.all([
				appWindow.outerPosition(),
				appWindow.innerPosition(),
				appWindow.innerSize(),
			])
			drag.anchorX = (size.width * relativeX) / ratio
			drag.position = {
				x: Math.round(outer.x + fingerX - drag.anchorX * ratio - inner.x),
				y: Math.round(outer.y + fingerY - drag.anchorY * ratio - inner.y),
			}
			await appWindow.setPosition(new PhysicalPosition(drag.position.x, drag.position.y))
		} else {
			const pos = await appWindow.outerPosition()
			drag.position = { x: pos.x, y: pos.y }
		}
		pump(drag)
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
			anchorX: e.clientX,
			anchorY: e.clientY,
			position: null,
			latest: null,
			dragging: false,
			moving: false,
		}
	}

	const onTouchPointerMove = (e: React.PointerEvent<HTMLElement>) => {
		const drag = touchDrag.current
		if (!drag || drag.pointerId !== e.pointerId) return
		drag.latest = { x: e.clientX, y: e.clientY }
		if (!drag.dragging) {
			if (Math.hypot(e.clientX - drag.anchorX, e.clientY - drag.anchorY) < DRAG_THRESHOLD) return
			drag.dragging = true
			lastTap.current = 0
			beginTouchDrag(drag).catch((error) => console.error("Failed to start window drag:", error))
			return
		}
		pump(drag)
	}

	const onTouchPointerUp = (e: React.PointerEvent<HTMLElement>) => {
		const drag = touchDrag.current
		if (drag?.pointerId !== e.pointerId) return
		touchDrag.current = null
		// Land exactly where the finger was lifted
		if (drag.dragging && e.type === "pointerup") {
			drag.latest = { x: e.clientX, y: e.clientY }
			pump(drag)
		}
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
				<img src="/ingot.svg" alt="Ingot" className="size-4.5 rounded-sm object-contain" />
				<span className="font-semibold text-foreground text-xs tracking-wide">{title}</span>
				<Badge variant="secondary">v{APP_VERSION}</Badge>
			</div>

			{/* Draggable center region */}
			<div data-tauri-drag-region className="h-full flex-1" />

			{/* Window control buttons */}
			<div className="flex items-center gap-0.5">
				<button
					type="button"
					onClick={handleMinimize}
					aria-label={t("common.minimize")}
					className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				>
					<Minus className="size-3.5" />
				</button>

				<button
					type="button"
					onClick={handleToggleMaximize}
					aria-label={isMaximized ? t("common.restore") : t("common.maximize")}
					className="inline-flex size-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
				>
					{isMaximized ? <Copy className="size-3" /> : <Square className="size-3" />}
				</button>

				<button
					type="button"
					onClick={handleClose}
					aria-label={t("common.close")}
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
