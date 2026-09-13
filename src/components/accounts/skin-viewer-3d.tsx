import {
	Eye,
	Footprints,
	Hand,
	Loader2,
	Pause,
	Play,
	RotateCcw,
	Sparkles,
	User,
	Zap,
} from "lucide-react"
import { memo, useEffect, useRef, useState } from "react"
import {
	IdleAnimation,
	RunningAnimation,
	SkinViewer,
	WalkingAnimation,
	WaveAnimation,
} from "skinview3d"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { accountService } from "@/services/account-service"

export const DEFAULT_STEVE_SKIN =
	"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAMAAACdt4HsAAAAdVBMVEUAAAD///+3g2sAzMyzeV4KvLyqclkAr68ApKSbY0mUYD4ElZWPXj6QWT8FiIiBUzkAf38Denp2SzNVVVV3QjVSPYlqQDBKSkpGOqUAaGhBNZs/Pz86MYk3Nzc/KhVJJRAoKCg0JRIzJBFCHQorHg0mGgokGAjoejraAAAAAXRSTlMAQObYZgAAAtFJREFUeNrtlm132jAMhelKFWuOHdr1hYYkZcbz//+Ju5LjsXRwEujHcSNsOefc50gmEK+KYgyIEJtRq0sVw4AQwBN0DQBmudyTyl0OgDvgcz0g5iZcbuFyQAggxGFoXAO5+nKA7AFUN/m6GAB3BCM29QfU1MuN4kwyDhAIWASZgmZxye7HlEJAwC9XEhxikPvDot6jDLz+tib+qeshBC0D1FmAVpxiZFqviYjVldvQRuYB+vhxorsX4pc7Suz0VkgKCku+/5AYInp9JWIoaf0pCSTOAlJMjuvmu6s/np8PSJqaHW4KIC4BNA5+FuPh44Dv32HBTh7HX9DCX2UH9f1+X9b3xMDSfTeq3K+qyjARnQTs933/B8AO4pMAAvxsBbOABwDmKzjfgpEWPlfQtl273W47TG2L2UKVIWYyVc4NRlgrZJg2kCyOgO4I2ELWe1tVlgij5pDa1Q/A+/sG6REwGtsC8I+P3sAiA3IlWAm1G6sALE4DEI9QMUluxIXPZtQ7JPMEIG0IoYPE5I0C/AiwAmFmJ5cKi2kFCLgVJGV7YxQwtqAFERETY4CboCkAKrOHrJSNbSi5LJkyQgGII6CdAiw8Xso2xkquACMAlTYyqaBTo2CQQdZbhNEiNEMCv2diBUBTwNtbN+pNBZcyYLQiccPvCYBCmAD6HVw/xLqTdJe/DNTjYZMEHSHZZgBnAP8N6PudGHc9hBEA7Wnbep/3BSyMTPDmLVDE6qb/Ug9ZlYEqJKubbvqk2ZetvJOIiCW/5sChAMdEX6jgoSLoKxUYx4taKOeFti0v3Q1UXu2ieUBxdgBB+jpXgo7LAJ0A2iPAZjNiaQUFgKycB4rG/4OiI/DcecGJWC6Vm68AZkgrkJxhzUeDrMtagASgdqUwLd9ESABOASqdFjxE5UlABmnvap8BnDsvFL/YZwCnzwsK4ALgGcCJ8wLnTUCQTP/8Hn4DsAh5tPm8HxQAAAAASUVORK5CYII="

export interface SkinViewer3DProps {
	skinUrl?: string | null
	username: string
	width?: number
	height?: number
	className?: string
	enableControls?: boolean
	showToolbar?: boolean
	model?: "default" | "slim"
	autoResize?: boolean
	borderless?: boolean
	floatingControlsClassName?: string
	floatingToolbar?: boolean
	footerActions?: React.ReactNode
	zoom?: number
}

type AnimationType = "idle" | "walk" | "run" | "wave" | "none"

const SkinViewer3D = ({
	skinUrl,
	username: _username,
	width = 240,
	height = 300,
	className = "",
	enableControls = true,
	showToolbar = true,
	model,
	autoResize = false,
	borderless = false,
	floatingControlsClassName,
	floatingToolbar = false,
	footerActions,
	zoom,
}: SkinViewer3DProps) => {
	const canvasRef = useRef<HTMLCanvasElement | null>(null)
	const containerRef = useRef<HTMLDivElement | null>(null)
	const viewerRef = useRef<SkinViewer | null>(null)
	const initialWidthRef = useRef(width || 240)
	const initialHeightRef = useRef(height || 300)

	const [animation, setAnimation] = useState<AnimationType>("idle")
	const [isSlim, setIsSlim] = useState(model === "slim")
	const [hasUserOverriddenModel, setHasUserOverriddenModel] = useState(model !== undefined)
	const [showOuterLayer, setShowOuterLayer] = useState(true)
	const [isAutoRotate, setIsAutoRotate] = useState(false)
	const [resolvedSkin, setResolvedSkin] = useState<string>(DEFAULT_STEVE_SKIN)
	const [isLoadingSkin, setIsLoadingSkin] = useState(Boolean(skinUrl))

	useEffect(() => {
		if (model !== undefined) {
			setIsSlim(model === "slim")
			setHasUserOverriddenModel(true)
		}
	}, [model])

	useEffect(() => {
		let isMounted = true
		setHasUserOverriddenModel(model !== undefined)

		if (!skinUrl) {
			setResolvedSkin(DEFAULT_STEVE_SKIN)
			setIsLoadingSkin(false)
			return
		}

		setIsLoadingSkin(true)
		accountService
			.getSkinDataUrl(skinUrl)
			.then((dataUrl) => {
				if (!isMounted) return
				setResolvedSkin(dataUrl || skinUrl)
				setIsLoadingSkin(false)
			})
			.catch(() => {
				if (!isMounted) return
				setResolvedSkin(skinUrl)
				setIsLoadingSkin(false)
			})

		return () => {
			isMounted = false
		}
	}, [skinUrl, model])

	// Initialize SkinViewer
	useEffect(() => {
		const canvas = canvasRef.current
		if (!canvas) return

		const targetZoom = zoom ?? (floatingToolbar ? 0.55 : 0.9)
		const viewer = new SkinViewer({
			canvas,
			width: initialWidthRef.current,
			height: initialHeightRef.current,
			pixelRatio: "match-device",
			zoom: targetZoom,
		})

		// Configure camera & controls
		viewer.camera.position.z = 60
		viewer.camera.position.y = 0
		viewer.adjustCameraDistance()
		viewer.controls.enableRotate = enableControls
		viewer.controls.enableZoom = enableControls
		viewer.controls.enablePan = false
		viewer.autoRotateSpeed = 1.5

		// Initial animation
		viewer.animation = new IdleAnimation()

		viewerRef.current = viewer

		return () => {
			viewer.dispose()
			viewerRef.current = null
		}
	}, [enableControls, zoom, floatingToolbar])

	// Update zoom if prop changes
	useEffect(() => {
		const viewer = viewerRef.current
		if (!viewer) return
		const targetZoom = zoom ?? (floatingToolbar ? 0.55 : 0.9)
		viewer.zoom = targetZoom
		viewer.adjustCameraDistance()
		viewer.camera.position.y = 0
	}, [zoom, floatingToolbar])

	// Update size if fixed width/height changes and not autoResize
	useEffect(() => {
		if (autoResize) return
		const viewer = viewerRef.current
		if (viewer && width && height) {
			viewer.setSize(width, height)
			viewer.adjustCameraDistance()
			viewer.camera.position.y = 0
		}
	}, [width, height, autoResize])

	// Responsive AutoResize
	useEffect(() => {
		if (!autoResize) return
		const container = containerRef.current
		if (!container) return

		const updateViewerSize = () => {
			const viewer = viewerRef.current
			if (!viewer || !container) return
			// Use clientWidth/clientHeight because getBoundingClientRect() includes
			// CSS transform scales (e.g. data-open:zoom-in-95 during modal entrance)
			const w = container.clientWidth || Math.floor(container.getBoundingClientRect().width)
			const h = container.clientHeight || Math.floor(container.getBoundingClientRect().height)
			if (w > 20 && h > 20) {
				viewer.setSize(w, h)
				viewer.adjustCameraDistance()
				viewer.camera.position.y = 0
				if (canvasRef.current) {
					canvasRef.current.style.width = "100%"
					canvasRef.current.style.height = "100%"
				}
			}
		}

		updateViewerSize()

		const ro = new ResizeObserver((entries) => {
			const entry = entries[0]
			const viewer = viewerRef.current
			if (!viewer || !container) return

			let w = 0
			let h = 0
			if (entry) {
				if (entry.contentRect && entry.contentRect.width > 20) {
					w = Math.floor(entry.contentRect.width)
					h = Math.floor(entry.contentRect.height)
				} else if (entry.borderBoxSize?.[0]) {
					w = Math.floor(entry.borderBoxSize[0].inlineSize)
					h = Math.floor(entry.borderBoxSize[0].blockSize)
				}
			}

			if (!w || !h) {
				w = container.clientWidth
				h = container.clientHeight
			}

			if (w > 20 && h > 20) {
				viewer.setSize(w, h)
				viewer.adjustCameraDistance()
				viewer.camera.position.y = 0
				if (canvasRef.current) {
					canvasRef.current.style.width = "100%"
					canvasRef.current.style.height = "100%"
				}
			}
		})
		ro.observe(container)

		// Also handle modal entrance animations (e.g. zoom-in-95) and transitions
		const animEndHandler = () => updateViewerSize()
		container.addEventListener("animationend", animEndHandler)
		container.addEventListener("transitionend", animEndHandler)
		window.addEventListener("resize", updateViewerSize)

		const timer1 = setTimeout(updateViewerSize, 50)
		const timer2 = setTimeout(updateViewerSize, 120)
		const timer3 = setTimeout(updateViewerSize, 250)

		return () => {
			ro.disconnect()
			container.removeEventListener("animationend", animEndHandler)
			container.removeEventListener("transitionend", animEndHandler)
			window.removeEventListener("resize", updateViewerSize)
			clearTimeout(timer1)
			clearTimeout(timer2)
			clearTimeout(timer3)
		}
	}, [autoResize])

	// Update Animation
	useEffect(() => {
		const viewer = viewerRef.current
		if (!viewer) return

		switch (animation) {
			case "idle":
				viewer.animation = new IdleAnimation()
				break
			case "walk":
				viewer.animation = new WalkingAnimation()
				break
			case "run":
				viewer.animation = new RunningAnimation()
				break
			case "wave":
				viewer.animation = new WaveAnimation()
				break
			case "none":
				viewer.animation = null
				break
		}
	}, [animation])

	// Update Auto Rotate
	useEffect(() => {
		const viewer = viewerRef.current
		if (!viewer) return
		viewer.autoRotate = isAutoRotate
	}, [isAutoRotate])

	// Update Outer Layers Visibility
	useEffect(() => {
		const viewer = viewerRef.current
		if (!viewer) return
		if (viewer.playerObject?.skin) {
			viewer.playerObject.skin.setOuterLayerVisible(showOuterLayer)
		}
	}, [showOuterLayer])

	// Update Model (Slim vs Default) and Skin
	useEffect(() => {
		const viewer = viewerRef.current
		if (!viewer || !resolvedSkin) return

		const modelOption = hasUserOverriddenModel ? (isSlim ? "slim" : "default") : "auto-detect"

		viewer
			.loadSkin(resolvedSkin, {
				model: modelOption,
			})
			?.then(() => {
				if (!hasUserOverriddenModel && viewer.playerObject?.skin) {
					setIsSlim(viewer.playerObject.skin.modelType === "slim")
				}
			})
			?.catch((err: unknown) => {
				console.warn("Failed to load skin in 3D viewer:", err)
			})
	}, [isSlim, resolvedSkin, hasUserOverriddenModel])

	const handleResetView = () => {
		const viewer = viewerRef.current
		if (!viewer) return
		viewer.controls.reset()
		viewer.camera.position.x = 0
		viewer.camera.position.y = 0
		viewer.camera.position.z = 60
		const targetZoom = zoom ?? (floatingToolbar ? 0.55 : 0.9)
		viewer.zoom = targetZoom
		viewer.adjustCameraDistance()
		viewer.camera.position.y = 0
	}

	return (
		<div
			className={cn(
				"relative flex flex-col",
				!floatingToolbar && !autoResize && "items-center",
				floatingToolbar ? "size-full min-h-0 flex-1 justify-between overflow-hidden" : "",
				!floatingToolbar && showToolbar && "gap-3",
				!floatingToolbar && autoResize && "size-full min-h-0 flex-1",
				className,
			)}
		>
			{/* Canvas Container */}
			<div
				ref={containerRef}
				className={cn(
					"relative overflow-hidden",
					!floatingToolbar && !autoResize && "flex items-center justify-center",
					floatingToolbar
						? "absolute inset-0 z-0 size-full bg-transparent"
						: borderless
							? "size-full bg-transparent"
							: "rounded-xl border border-border/50 bg-gradient-to-b from-zinc-900/90 to-zinc-950 shadow-inner",
					!floatingToolbar && autoResize ? "size-full min-h-0 flex-1" : "",
				)}
			>
				<canvas
					ref={canvasRef}
					className={cn(
						"cursor-grab active:cursor-grabbing",
						autoResize && "!block !size-full !max-w-none !max-h-none",
					)}
					style={autoResize ? { width: "100%", height: "100%" } : { width, height }}
				/>

				{isLoadingSkin && (
					<div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-zinc-950/40 backdrop-blur-xs">
						<Loader2 className="size-6 animate-spin text-primary" />
					</div>
				)}

				{/* Floating Reset / Rotate / Animation Overlay Controls */}
				<div
					className={cn(
						"absolute z-10 flex flex-col gap-1",
						floatingControlsClassName || "top-2 right-2",
					)}
				>
					<Button
						variant="outline"
						size="icon-xs"
						onClick={handleResetView}
						title="Reset 3D camera"
						className="size-6 rounded-md border-border/50 bg-zinc-900/80 text-muted-foreground backdrop-blur-xs transition-colors hover:bg-zinc-800 hover:text-foreground active:scale-95"
					>
						<RotateCcw className="size-3" />
					</Button>
					<Button
						variant={isAutoRotate ? "default" : "outline"}
						size="icon-xs"
						onClick={() => setIsAutoRotate(!isAutoRotate)}
						title={isAutoRotate ? "Pause auto-rotation" : "Auto-rotate 3D model"}
						className={cn(
							"size-6 rounded-md backdrop-blur-xs transition-all active:scale-95",
							isAutoRotate
								? "border-primary bg-primary text-primary-foreground shadow-xs hover:bg-primary/90 hover:text-primary-foreground"
								: "border-border/50 bg-zinc-900/80 text-muted-foreground hover:bg-zinc-800 hover:text-foreground",
						)}
					>
						<Sparkles className="size-3" />
					</Button>
					<Button
						variant={animation === "walk" ? "default" : "outline"}
						size="icon-xs"
						onClick={() => setAnimation(animation === "walk" ? "idle" : "walk")}
						title={animation === "walk" ? "Switch to idle pose" : "Play walking animation"}
						className={cn(
							"size-6 rounded-md backdrop-blur-xs transition-all active:scale-95",
							animation === "walk"
								? "border-primary bg-primary text-primary-foreground shadow-xs hover:bg-primary/90 hover:text-primary-foreground"
								: "border-border/50 bg-zinc-900/80 text-muted-foreground hover:bg-zinc-800 hover:text-foreground",
						)}
					>
						<Footprints className="size-3" />
					</Button>
				</div>
			</div>

			{/* Middle interaction spacer for full bleed */}
			{floatingToolbar && <div className="pointer-events-none min-h-0 flex-1" />}

			{/* Interactive Toolbar */}
			{(showToolbar || footerActions) && (
				<div
					className={cn(
						"flex w-full flex-col gap-2",
						floatingToolbar &&
							"pointer-events-auto relative z-10 bg-gradient-to-t from-popover/95 via-popover/70 to-transparent p-3 sm:p-4",
					)}
				>
					{/* Animations Selector */}
					{showToolbar && (
						<>
							<div className="flex items-center justify-between gap-1 rounded-lg border border-border/40 bg-zinc-950/60 p-1">
								<Button
									variant={animation === "idle" ? "secondary" : "ghost"}
									size="xs"
									onClick={() => setAnimation("idle")}
									className="flex-1 text-[11px]"
									title="Idle breathing animation"
								>
									<Play className="mr-1 size-2.5" />
									Idle
								</Button>
								<Button
									variant={animation === "walk" ? "secondary" : "ghost"}
									size="xs"
									onClick={() => setAnimation("walk")}
									className="flex-1 text-[11px]"
									title="Walking animation"
								>
									<Footprints className="mr-1 size-2.5" />
									Walk
								</Button>
								<Button
									variant={animation === "run" ? "secondary" : "ghost"}
									size="xs"
									onClick={() => setAnimation("run")}
									className="flex-1 text-[11px]"
									title="Running animation"
								>
									<Zap className="mr-1 size-2.5" />
									Run
								</Button>
								<Button
									variant={animation === "wave" ? "secondary" : "ghost"}
									size="xs"
									onClick={() => setAnimation("wave")}
									className="flex-1 text-[11px]"
									title="Waving arm animation"
								>
									<Hand className="mr-1 size-2.5" />
									Wave
								</Button>
								<Button
									variant={animation === "none" ? "secondary" : "ghost"}
									size="xs"
									onClick={() => setAnimation("none")}
									className="size-6 px-0"
									title="Pause animations"
								>
									<Pause className="size-2.5" />
								</Button>
							</div>

							{/* Model Options (Layers & Slim Arm toggle) */}
							<div className="flex items-center justify-between gap-2 text-xs">
								<Button
									variant="outline"
									size="xs"
									onClick={() => setShowOuterLayer(!showOuterLayer)}
									className="flex-1 text-[11px]"
								>
									<Eye className="mr-1.5 size-3 text-muted-foreground" />
									{showOuterLayer ? "Outer Layer On" : "Outer Layer Off"}
								</Button>
								<Button
									variant="outline"
									size="xs"
									onClick={() => {
										setHasUserOverriddenModel(true)
										setIsSlim(!isSlim)
									}}
									className="flex-1 text-[11px]"
								>
									<User className="mr-1.5 size-3 text-muted-foreground" />
									{isSlim ? "Model: Slim (Alex)" : "Model: Classic (Steve)"}
								</Button>
							</div>
						</>
					)}

					{/* Custom Footer Actions (e.g. Change Skin, Copy URL, Download) */}
					{footerActions && (
						<div
							className={cn(
								"flex items-center justify-between gap-2",
								showToolbar && "border-border/30 border-t pt-2",
							)}
						>
							{footerActions}
						</div>
					)}
				</div>
			)}
		</div>
	)
}

SkinViewer3D.displayName = "SkinViewer3D"

export default memo(SkinViewer3D)
