import { memo, useEffect, useRef, useState } from "react"
import { accountService } from "@/services/account-service"

interface SkinAvatarProps {
	skinUrl?: string | null
	username: string
	size?: number
	className?: string
}

const SkinAvatar = ({ skinUrl, username, size = 36, className = "" }: SkinAvatarProps) => {
	const canvasRef = useRef<HTMLCanvasElement | null>(null)
	const [hasError, setHasError] = useState(false)
	const [isLoading, setIsLoading] = useState(Boolean(skinUrl))

	useEffect(() => {
		if (!skinUrl) {
			setIsLoading(false)
			setHasError(true)
			return
		}

		let isMounted = true
		setIsLoading(true)
		setHasError(false)

		accountService
			.getSkinDataUrl(skinUrl)
			.then((dataUrl) => {
				if (!isMounted) return
				const sourceUrl = dataUrl || skinUrl

				const img = new Image()
				if (!dataUrl) {
					img.crossOrigin = "anonymous"
				}
				img.src = sourceUrl

				img.onload = () => {
					if (!isMounted) return
					const canvas = canvasRef.current
					if (!canvas) return

					const ctx = canvas.getContext("2d")
					if (!ctx) return

					ctx.imageSmoothingEnabled = false
					ctx.clearRect(0, 0, size, size)

					// 1. Draw head base layer: (8, 8) 8x8 in skin texture
					ctx.drawImage(img, 8, 8, 8, 8, 0, 0, size, size)

					// 2. Draw head outer/accessory layer: (40, 8) 8x8 in skin texture
					ctx.drawImage(img, 40, 8, 8, 8, 0, 0, size, size)

					setIsLoading(false)
				}

				img.onerror = () => {
					if (!isMounted) return
					setIsLoading(false)
					setHasError(true)
				}
			})
			.catch(() => {
				if (!isMounted) return
				setIsLoading(false)
				setHasError(true)
			})

		return () => {
			isMounted = false
		}
	}, [skinUrl, size])

	const initials = (username || "P").slice(0, 2).toUpperCase()

	return (
		<div
			className={`relative flex select-none items-center justify-center overflow-hidden rounded-lg border border-border/40 bg-zinc-800 font-bold text-foreground text-xs shadow-inner ${className}`}
			style={{ width: size, height: size }}
		>
			<canvas
				ref={canvasRef}
				width={size}
				height={size}
				className={`size-full ${hasError || isLoading ? "hidden" : "block"}`}
				style={{ imageRendering: "pixelated" }}
			/>

			{(hasError || isLoading || !skinUrl) && (
				<span className="text-zinc-300 tracking-wider">{initials}</span>
			)}
		</div>
	)
}

SkinAvatar.displayName = "SkinAvatar"

export default memo(SkinAvatar)
