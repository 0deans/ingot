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
}: SkinViewer3DProps) => {
	const canvasRef = useRef<HTMLCanvasElement | null>(null)
	const viewerRef = useRef<SkinViewer | null>(null)

	const [animation, setAnimation] = useState<AnimationType>("idle")
	const [isSlim, setIsSlim] = useState(false)
	const [hasUserOverriddenModel, setHasUserOverriddenModel] = useState(false)
	const [showOuterLayer, setShowOuterLayer] = useState(true)
	const [isAutoRotate, setIsAutoRotate] = useState(false)
	const [resolvedSkin, setResolvedSkin] = useState<string>(DEFAULT_STEVE_SKIN)
	const [isLoadingSkin, setIsLoadingSkin] = useState(Boolean(skinUrl))

	useEffect(() => {
		let isMounted = true
		setHasUserOverriddenModel(false)

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
	}, [skinUrl])

	// Initialize SkinViewer
	useEffect(() => {
		const canvas = canvasRef.current
		if (!canvas) return

		const viewer = new SkinViewer({
			canvas,
			width,
			height,
			pixelRatio: "match-device",
		})

		// Configure camera & controls
		viewer.camera.position.z = 60
		viewer.camera.position.y = -5
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
	}, [width, height, enableControls])

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
		viewer.camera.position.z = 60
		viewer.camera.position.y = -5
	}

	return (
		<div className={`flex flex-col items-center gap-3 ${className}`}>
			{/* Canvas Container */}
			<div className="relative flex items-center justify-center overflow-hidden rounded-xl border border-border/50 bg-gradient-to-b from-zinc-900/90 to-zinc-950 p-2 shadow-inner">
				<canvas
					ref={canvasRef}
					className="cursor-grab active:cursor-grabbing"
					style={{ width, height }}
				/>

				{isLoadingSkin && (
					<div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-zinc-950/40 backdrop-blur-xs">
						<Loader2 className="size-6 animate-spin text-primary" />
					</div>
				)}

				{/* Floating Reset / Rotate Overlay Controls */}
				<div className="absolute top-2 right-2 flex flex-col gap-1">
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
				</div>
			</div>

			{/* Interactive Toolbar */}
			{showToolbar && (
				<div className="flex w-full flex-col gap-2">
					{/* Animations Selector */}
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
				</div>
			)}
		</div>
	)
}

SkinViewer3D.displayName = "SkinViewer3D"

export default memo(SkinViewer3D)
