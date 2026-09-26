import { Crosshair, Minus, Plus } from "lucide-react"
import { memo, useCallback, useEffect, useRef, useState } from "react"
import type { MapRegion, PlayerDetails } from "@/bindings"
import { avatarUrl } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import { useMapTile } from "@/services/server-data"

const REGION_BLOCKS = 512
const MIN_SCALE = 1 / 16
/** Same as the map's player polling interval, so movement looks continuous */
const MARKER_GLIDE_MS = 1500
const MAX_SCALE = 8

export interface MapView {
	/** World coordinates at the center of the viewport */
	x: number
	z: number
	/** Screen pixels per block */
	scale: number
}

const clampScale = (s: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s))

export function WorldMap({
	serverId,
	dimension,
	regions,
	revision,
	players,
	view,
	onViewChange,
	onSelectPlayer,
	voidColor,
}: {
	serverId: string
	dimension: string
	regions: MapRegion[]
	revision: number
	players: PlayerDetails[]
	view: MapView
	onViewChange: (view: MapView) => void
	onSelectPlayer: (name: string) => void
	voidColor: string
}) {
	const containerRef = useRef<HTMLDivElement>(null)
	const [size, setSize] = useState({ w: 0, h: 0 })
	const [cursor, setCursor] = useState<{ x: number; z: number } | null>(null)
	const pointers = useRef(new Map<number, { x: number; y: number }>())
	const gesture = useRef<{ view: MapView; x: number; y: number; dist: number } | null>(null)
	const viewRef = useRef(view)
	viewRef.current = view
	const lastScale = useRef(view.scale)
	const zooming = lastScale.current !== view.scale
	useEffect(() => {
		lastScale.current = view.scale
	})

	useEffect(() => {
		const el = containerRef.current
		if (!el) return
		const observer = new ResizeObserver(([entry]) =>
			setSize({ w: entry.contentRect.width, h: entry.contentRect.height }),
		)
		observer.observe(el)
		return () => observer.disconnect()
	}, [])

	/** Screen point (relative to the map) -> world coordinates */
	const toWorld = useCallback(
		(px: number, py: number, v: MapView = viewRef.current) => ({
			x: v.x + (px - size.w / 2) / v.scale,
			z: v.z + (py - size.h / 2) / v.scale,
		}),
		[size],
	)

	/** Zoom keeping the world point under (px, py) fixed on screen */
	const zoomAt = useCallback(
		(factor: number, px: number, py: number, base: MapView = viewRef.current) => {
			const scale = clampScale(base.scale * factor)
			const anchor = toWorld(px, py, base)
			onViewChange({
				scale,
				x: anchor.x - (px - size.w / 2) / scale,
				z: anchor.z - (py - size.h / 2) / scale,
			})
		},
		[onViewChange, size, toWorld],
	)

	// Wheel zoom (non-passive so the page doesn't scroll)
	useEffect(() => {
		const el = containerRef.current
		if (!el) return
		const onWheel = (e: WheelEvent) => {
			e.preventDefault()
			const rect = el.getBoundingClientRect()
			zoomAt(e.deltaY < 0 ? 1.2 : 1 / 1.2, e.clientX - rect.left, e.clientY - rect.top)
		}
		el.addEventListener("wheel", onWheel, { passive: false })
		return () => el.removeEventListener("wheel", onWheel)
	}, [zoomAt])

	const local = (e: React.PointerEvent) => {
		const rect = containerRef.current?.getBoundingClientRect()
		return { x: e.clientX - (rect?.left ?? 0), y: e.clientY - (rect?.top ?? 0) }
	}

	const startGesture = () => {
		const pts = [...pointers.current.values()]
		const cx = pts.reduce((s, p) => s + p.x, 0) / pts.length
		const cy = pts.reduce((s, p) => s + p.y, 0) / pts.length
		const dist = pts.length > 1 ? Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y) : 0
		gesture.current = { view: viewRef.current, x: cx, y: cy, dist }
	}

	const onPointerDown = (e: React.PointerEvent) => {
		;(e.target as Element).setPointerCapture?.(e.pointerId)
		pointers.current.set(e.pointerId, local(e))
		startGesture()
	}

	const onPointerMove = (e: React.PointerEvent) => {
		const p = local(e)
		setCursor(toWorld(p.x, p.y))
		if (!pointers.current.has(e.pointerId) || !gesture.current) return
		pointers.current.set(e.pointerId, p)
		const pts = [...pointers.current.values()]
		const cx = pts.reduce((s, q) => s + q.x, 0) / pts.length
		const cy = pts.reduce((s, q) => s + q.y, 0) / pts.length
		const g = gesture.current
		let scale = g.view.scale
		if (pts.length > 1 && g.dist > 0) {
			scale = clampScale(
				g.view.scale * (Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y) / g.dist),
			)
		}
		// Keep the world point that was under the gesture center under the fingers
		const anchor = toWorld(g.x, g.y, g.view)
		onViewChange({
			scale,
			x: anchor.x - (cx - size.w / 2) / scale,
			z: anchor.z - (cy - size.h / 2) / scale,
		})
	}

	const onPointerUp = (e: React.PointerEvent) => {
		pointers.current.delete(e.pointerId)
		if (pointers.current.size > 0) startGesture()
		else gesture.current = null
	}

	// Only mount tiles that intersect the viewport (plus a margin)
	const halfW = size.w / 2 / view.scale + REGION_BLOCKS / 2
	const halfH = size.h / 2 / view.scale + REGION_BLOCKS / 2
	const visible = regions.filter((r) => {
		const cx = r.x * REGION_BLOCKS + REGION_BLOCKS / 2
		const cz = r.z * REGION_BLOCKS + REGION_BLOCKS / 2
		return (
			Math.abs(cx - view.x) < halfW + REGION_BLOCKS / 2 &&
			Math.abs(cz - view.z) < halfH + REGION_BLOCKS / 2
		)
	})

	return (
		<div
			ref={containerRef}
			role="application"
			aria-label="World map. Drag to move, scroll or pinch to zoom."
			className="relative size-full touch-none select-none overflow-hidden"
			style={{ background: voidColor, cursor: gesture.current ? "grabbing" : "grab" }}
			onPointerDown={onPointerDown}
			onPointerMove={onPointerMove}
			onPointerUp={onPointerUp}
			onPointerCancel={onPointerUp}
			onPointerLeave={() => setCursor(null)}
			onDoubleClick={(e) => {
				const rect = containerRef.current?.getBoundingClientRect()
				zoomAt(2, e.clientX - (rect?.left ?? 0), e.clientY - (rect?.top ?? 0))
			}}
		>
			{/* Faint grid so empty areas still show movement */}
			<div
				className="pointer-events-none absolute inset-0 opacity-[0.07]"
				style={{
					backgroundImage:
						"linear-gradient(#fff 1px, transparent 1px), linear-gradient(90deg, #fff 1px, transparent 1px)",
					backgroundSize: `${REGION_BLOCKS * view.scale}px ${REGION_BLOCKS * view.scale}px`,
					backgroundPosition: `${size.w / 2 - view.x * view.scale}px ${size.h / 2 - view.z * view.scale}px`,
				}}
			/>

			<div
				className="absolute top-0 left-0 origin-top-left"
				style={{
					transform: `translate(${size.w / 2 - view.x * view.scale}px, ${size.h / 2 - view.z * view.scale}px) scale(${view.scale})`,
				}}
			>
				{visible.map((r) => (
					<RegionTile
						key={`${r.x}.${r.z}`}
						serverId={serverId}
						dimension={dimension}
						region={r}
						revision={revision}
						pixelated={view.scale >= 1}
					/>
				))}
			</div>

			{/* Pans with the map instantly; heads glide to new positions between polls */}
			<div
				className="pointer-events-none absolute top-0 left-0"
				style={{
					transform: `translate(${size.w / 2 - view.x * view.scale}px, ${size.h / 2 - view.z * view.scale}px)`,
				}}
			>
				{players.map((p) => (
					<button
						key={p.name}
						type="button"
						onPointerDown={(e) => e.stopPropagation()}
						onClick={() => onSelectPlayer(p.name)}
						className="group -translate-1/2 pointer-events-auto absolute flex flex-col items-center gap-1"
						style={{
							left: p.x * view.scale,
							top: p.z * view.scale,
							// No glide while zooming, or heads would trail behind the map
							transition: zooming
								? "none"
								: `left ${MARKER_GLIDE_MS}ms linear, top ${MARKER_GLIDE_MS}ms linear`,
						}}
					>
						<img
							src={avatarUrl(p.name, 48)}
							alt={p.name}
							className="size-7 rounded-md shadow-black/50 shadow-lg ring-2 ring-white transition-transform [image-rendering:pixelated] group-hover:scale-110"
						/>
						<span className="rounded-md bg-black/70 px-1.5 py-0.5 font-medium text-[10px] text-white backdrop-blur-sm">
							{p.name}
						</span>
					</button>
				))}
			</div>

			<div className="absolute right-3 bottom-3 flex flex-col overflow-hidden rounded-xl border border-white/10 bg-black/60 backdrop-blur-md">
				<MapButton label="Zoom in" onClick={() => zoomAt(1.5, size.w / 2, size.h / 2)}>
					<Plus className="size-4" />
				</MapButton>
				<div className="h-px bg-white/10" />
				<MapButton label="Zoom out" onClick={() => zoomAt(1 / 1.5, size.w / 2, size.h / 2)}>
					<Minus className="size-4" />
				</MapButton>
			</div>

			<div className="pointer-events-none absolute bottom-3 left-3 flex items-center gap-1.5 rounded-lg bg-black/60 px-2 py-1 font-mono text-[10px] text-zinc-300 backdrop-blur-md">
				<Crosshair className="size-3" />
				{Math.floor(cursor?.x ?? view.x)}, {Math.floor(cursor?.z ?? view.z)}
			</div>
		</div>
	)
}

function MapButton({
	label,
	onClick,
	children,
}: {
	label: string
	onClick: () => void
	children: React.ReactNode
}) {
	return (
		<button
			type="button"
			aria-label={label}
			onPointerDown={(e) => e.stopPropagation()}
			onClick={onClick}
			className="flex size-9 items-center justify-center text-zinc-200 transition-colors hover:bg-white/10"
		>
			{children}
		</button>
	)
}

const RegionTile = memo(function RegionTile({
	serverId,
	dimension,
	region,
	revision,
	pixelated,
}: {
	serverId: string
	dimension: string
	region: MapRegion
	revision: number
	pixelated: boolean
}) {
	const { data, isLoading } = useMapTile(serverId, dimension, region.x, region.z, revision)
	return (
		<div
			className="absolute"
			style={{
				left: region.x * REGION_BLOCKS,
				top: region.z * REGION_BLOCKS,
				// One block of overlap hides sub-pixel seams between tiles when zoomed out
				width: REGION_BLOCKS + 1,
				height: REGION_BLOCKS + 1,
			}}
		>
			{data ? (
				<img
					src={data}
					alt=""
					draggable={false}
					className={cn("size-full", pixelated && "[image-rendering:pixelated]")}
				/>
			) : (
				isLoading && <div className="size-full animate-pulse bg-white/[0.03]" />
			)}
		</div>
	)
})
