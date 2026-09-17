import { memo, useEffect, useRef, useState } from "react"
import { accountService } from "@/services/account-service"

interface SkinPreviewCanvasProps {
	skinUrl?: string | null
	isSlim?: boolean
	width?: number
	height?: number
	className?: string
}

const previewDataUrlCache = new Map<string, string>()

function SkinPreviewCanvas({
	skinUrl,
	isSlim = false,
	width = 64,
	height = 128,
	className = "",
}: SkinPreviewCanvasProps) {
	const canvasRef = useRef<HTMLCanvasElement | null>(null)
	const cacheKey = `${skinUrl ?? ""}_${isSlim ? "slim" : "classic"}_${width}_${height}`

	const cachedUrl = skinUrl ? previewDataUrlCache.get(cacheKey) : undefined
	const [renderedUrl, setRenderedUrl] = useState<string | undefined>(cachedUrl)
	const [hasError, setHasError] = useState(false)
	const [isLoading, setIsLoading] = useState(!cachedUrl && Boolean(skinUrl))

	useEffect(() => {
		if (!skinUrl) {
			setIsLoading(false)
			setHasError(true)
			return
		}

		const key = `${skinUrl}_${isSlim ? "slim" : "classic"}_${width}_${height}`
		const existing = previewDataUrlCache.get(key)
		if (existing) {
			setRenderedUrl(existing)
			setIsLoading(false)
			setHasError(false)
			return
		}

		let isMounted = true
		setIsLoading(true)
		setHasError(false)

		const drawSkinToCanvas = (sourceUrl: string) => {
			const img = new Image()
			if (!sourceUrl.startsWith("data:")) {
				img.crossOrigin = "anonymous"
			}
			img.src = sourceUrl

			img.onload = () => {
				if (!isMounted) return

				const offscreen = document.createElement("canvas")
				offscreen.width = width
				offscreen.height = height
				const ctx = offscreen.getContext("2d")
				if (!ctx) {
					setIsLoading(false)
					setHasError(true)
					return
				}

				ctx.imageSmoothingEnabled = false
				ctx.clearRect(0, 0, width, height)

				// Standard Minecraft body is 16 units wide, 32 units tall
				const u = width / 16
				const is64x64 = img.naturalHeight >= 64

				// 1. Head base: (8, 8, 8, 8) -> (4, 0, 8, 8)
				ctx.drawImage(img, 8, 8, 8, 8, 4 * u, 0, 8 * u, 8 * u)
				// Head outer layer: (40, 8, 8, 8) -> (4, 0, 8, 8)
				ctx.drawImage(img, 40, 8, 8, 8, 4 * u, 0, 8 * u, 8 * u)

				// 2. Torso base: (20, 20, 8, 12) -> (4, 8, 8, 12)
				ctx.drawImage(img, 20, 20, 8, 12, 4 * u, 8 * u, 8 * u, 12 * u)
				// Torso outer layer (if 64x64)
				if (is64x64) {
					ctx.drawImage(img, 20, 36, 8, 12, 4 * u, 8 * u, 8 * u, 12 * u)
				}

				// 3. Right Arm
				if (isSlim) {
					ctx.drawImage(img, 44, 20, 3, 12, 1 * u, 8 * u, 3 * u, 12 * u)
					if (is64x64) {
						ctx.drawImage(img, 44, 36, 3, 12, 1 * u, 8 * u, 3 * u, 12 * u)
					}
				} else {
					ctx.drawImage(img, 44, 20, 4, 12, 0, 8 * u, 4 * u, 12 * u)
					if (is64x64) {
						ctx.drawImage(img, 44, 36, 4, 12, 0, 8 * u, 4 * u, 12 * u)
					}
				}

				// 4. Left Arm
				if (is64x64) {
					if (isSlim) {
						ctx.drawImage(img, 36, 52, 3, 12, 12 * u, 8 * u, 3 * u, 12 * u)
						ctx.drawImage(img, 52, 52, 3, 12, 12 * u, 8 * u, 3 * u, 12 * u)
					} else {
						ctx.drawImage(img, 36, 52, 4, 12, 12 * u, 8 * u, 4 * u, 12 * u)
						ctx.drawImage(img, 52, 52, 4, 12, 12 * u, 8 * u, 4 * u, 12 * u)
					}
				} else {
					// In old 64x32 skins, left arm is mirrored right arm
					ctx.save()
					ctx.translate(16 * u, 0)
					ctx.scale(-1, 1)
					ctx.drawImage(img, 44, 20, 4, 12, 0, 8 * u, 4 * u, 12 * u)
					ctx.restore()
				}

				// 5. Right Leg: (4, 20, 4, 12) -> (4, 20, 4, 12)
				ctx.drawImage(img, 4, 20, 4, 12, 4 * u, 20 * u, 4 * u, 12 * u)
				if (is64x64) {
					ctx.drawImage(img, 4, 36, 4, 12, 4 * u, 20 * u, 4 * u, 12 * u)
				}

				// 6. Left Leg
				if (is64x64) {
					ctx.drawImage(img, 20, 52, 4, 12, 8 * u, 20 * u, 4 * u, 12 * u)
					ctx.drawImage(img, 4, 52, 4, 12, 8 * u, 20 * u, 4 * u, 12 * u)
				} else {
					ctx.save()
					ctx.translate(16 * u, 0)
					ctx.scale(-1, 1)
					ctx.drawImage(img, 4, 20, 4, 12, 4 * u, 20 * u, 4 * u, 12 * u)
					ctx.restore()
				}

				try {
					const dataUrl = offscreen.toDataURL("image/png")
					previewDataUrlCache.set(key, dataUrl)
					setRenderedUrl(dataUrl)
				} catch {
					// In case of any browser security error with toDataURL, render to live canvas
					const liveCanvas = canvasRef.current
					if (liveCanvas) {
						const liveCtx = liveCanvas.getContext("2d")
						if (liveCtx) {
							liveCtx.imageSmoothingEnabled = false
							liveCtx.clearRect(0, 0, width, height)
							liveCtx.drawImage(offscreen, 0, 0)
						}
					}
				}

				setIsLoading(false)
			}

			img.onerror = () => {
				if (!isMounted) return
				setIsLoading(false)
				setHasError(true)
			}
		}

		const cachedSkinData = accountService.getCachedSkinDataUrl(skinUrl)
		if (cachedSkinData) {
			drawSkinToCanvas(cachedSkinData)
		} else {
			accountService
				.getSkinDataUrl(skinUrl)
				.then((dataUrl) => {
					if (!isMounted) return
					drawSkinToCanvas(dataUrl || skinUrl)
				})
				.catch(() => {
					if (!isMounted) return
					drawSkinToCanvas(skinUrl)
				})
		}

		return () => {
			isMounted = false
		}
	}, [skinUrl, isSlim, width, height])

	return (
		<div
			className={`relative flex select-none items-center justify-center overflow-hidden ${className}`}
			style={{ width, height }}
		>
			{renderedUrl ? (
				<img
					src={renderedUrl}
					alt="Minecraft skin preview"
					width={width}
					height={height}
					className="size-full select-none object-contain"
					style={{ imageRendering: "pixelated" }}
				/>
			) : (
				<canvas
					ref={canvasRef}
					width={width}
					height={height}
					className={`size-full ${hasError || isLoading ? "hidden" : "block"}`}
					style={{ imageRendering: "pixelated" }}
				/>
			)}
			{isLoading && !renderedUrl && (
				<div className="size-full animate-pulse rounded bg-zinc-800/60" />
			)}
		</div>
	)
}

SkinPreviewCanvas.displayName = "SkinPreviewCanvas"

export default memo(SkinPreviewCanvas)
