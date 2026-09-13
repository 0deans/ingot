import { ChevronLeft, ChevronRight, FolderOpen, HardDrive, Trash2, X } from "lucide-react"
import { memo, useCallback, useEffect } from "react"
import { createPortal } from "react-dom"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { type ScreenshotInfo, screenshotService } from "@/services/screenshot-service"

interface ScreenshotLightboxProps {
	screenshot: ScreenshotInfo | null
	allScreenshots: ScreenshotInfo[]
	onClose: () => void
	onSelect: (screenshot: ScreenshotInfo) => void
	onDelete: (screenshot: ScreenshotInfo) => void
	onReveal: (screenshot: ScreenshotInfo) => void
}

function formatBytes(bytes: number): string {
	if (!bytes || bytes === 0) return "0 B"
	const k = 1024
	const sizes = ["B", "KB", "MB", "GB"]
	const i = Math.floor(Math.log(bytes) / Math.log(k))
	return `${(bytes / k ** i).toFixed(1)} ${sizes[i]}`
}

function formatDate(timestampSec: number): string {
	if (!timestampSec) return ""
	const ms = timestampSec > 1e11 ? timestampSec : timestampSec * 1000
	const date = new Date(ms)
	return date.toLocaleDateString(undefined, {
		weekday: "short",
		month: "short",
		day: "numeric",
		year: "numeric",
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
	})
}

const ScreenshotLightbox = ({
	screenshot,
	allScreenshots,
	onClose,
	onSelect,
	onDelete,
	onReveal,
}: ScreenshotLightboxProps) => {
	const currentIndex = screenshot
		? allScreenshots.findIndex(
				(s) => s.instanceId === screenshot.instanceId && s.fileName === screenshot.fileName,
			)
		: -1

	const hasPrev = currentIndex > 0
	const hasNext = currentIndex >= 0 && currentIndex < allScreenshots.length - 1

	const goToPrev = useCallback(() => {
		if (hasPrev) {
			onSelect(allScreenshots[currentIndex - 1])
		}
	}, [hasPrev, allScreenshots, currentIndex, onSelect])

	const goToNext = useCallback(() => {
		if (hasNext) {
			onSelect(allScreenshots[currentIndex + 1])
		}
	}, [hasNext, allScreenshots, currentIndex, onSelect])

	useEffect(() => {
		if (!screenshot) return

		const originalOverflow = document.body.style.overflow
		document.body.style.overflow = "hidden"

		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.key === "Escape") {
				onClose()
			} else if (e.key === "ArrowLeft") {
				goToPrev()
			} else if (e.key === "ArrowRight") {
				goToNext()
			} else if (e.key === "Delete") {
				onDelete(screenshot)
			}
		}

		window.addEventListener("keydown", handleKeyDown)
		return () => {
			document.body.style.overflow = originalOverflow
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [screenshot, onClose, goToPrev, goToNext, onDelete])

	if (!screenshot || typeof document === "undefined") return null

	const imageUrl = screenshotService.getImageUrl(screenshot.filePath)

	return createPortal(
		<div className="fixed inset-x-0 top-10 bottom-0 z-40 flex animate-in select-none items-center justify-center p-3 duration-200">
			{/* Backdrop overlay button - clicking outside image closes lightbox */}
			<button
				type="button"
				aria-label="Close preview"
				tabIndex={-1}
				onClick={onClose}
				className="absolute inset-0 size-full cursor-default border-none bg-black/92 outline-none backdrop-blur-2xl transition-all"
			/>

			{/* Floating Top Controls */}
			<div className="pointer-events-none absolute inset-x-0 top-0 z-20 flex items-center justify-between p-4">
				{/* Top-Left: Instance & Name pill */}
				<div className="pointer-events-auto flex items-center gap-2.5 rounded-full border border-zinc-800/80 bg-zinc-950/80 px-3.5 py-1.5 shadow-2xl backdrop-blur-md">
					<span className="flex items-center gap-1.5 font-medium text-xs text-zinc-300">
						<HardDrive className="size-3.5 text-primary" />
						<span className="max-w-[160px] truncate">{screenshot.instanceName}</span>
					</span>
					<span className="text-zinc-600">•</span>
					<span
						title={screenshot.fileName}
						className="max-w-[240px] truncate font-medium text-xs text-zinc-100"
					>
						{screenshot.fileName}
					</span>
				</div>

				{/* Top-Right: Counter & Action buttons pill */}
				<div className="pointer-events-auto flex items-center gap-1 rounded-full border border-zinc-800/80 bg-zinc-950/80 p-1 pl-3 shadow-2xl backdrop-blur-md">
					{currentIndex >= 0 && (
						<span className="mr-1.5 font-mono text-xs text-zinc-400">
							{currentIndex + 1} / {allScreenshots.length}
						</span>
					)}

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									size="icon"
									variant="ghost"
									className="size-8 rounded-full text-zinc-300 hover:bg-zinc-800 hover:text-white"
									onClick={() => onReveal(screenshot)}
								>
									<FolderOpen className="size-4" />
								</Button>
							}
						/>
						<TooltipContent side="bottom">Show in Folder</TooltipContent>
					</Tooltip>

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									size="icon"
									variant="ghost"
									className="size-8 rounded-full text-red-400 hover:bg-red-950/60 hover:text-red-300"
									onClick={() => onDelete(screenshot)}
								>
									<Trash2 className="size-4" />
								</Button>
							}
						/>
						<TooltipContent side="bottom">Delete Screenshot</TooltipContent>
					</Tooltip>

					<div className="mx-0.5 h-4 w-px bg-zinc-800" />

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									size="icon"
									variant="ghost"
									className="size-8 rounded-full text-zinc-400 hover:bg-zinc-800 hover:text-white"
									onClick={onClose}
								>
									<X className="size-4" />
								</Button>
							}
						/>
						<TooltipContent side="bottom">Close (Esc)</TooltipContent>
					</Tooltip>
				</div>
			</div>

			{/* Left Navigation Chevron */}
			{hasPrev && (
				<Button
					size="icon"
					variant="secondary"
					aria-label="Previous screenshot"
					className="absolute left-4 z-20 size-12 rounded-full border border-zinc-700/50 bg-zinc-900/80 text-zinc-200 shadow-2xl backdrop-blur-md transition-all hover:scale-110 hover:bg-zinc-800 hover:text-white active:scale-95"
					onClick={goToPrev}
				>
					<ChevronLeft className="size-6" />
				</Button>
			)}

			{/* Main Image Stage */}
			<div className="pointer-events-none relative z-10 flex max-h-[86vh] max-w-[92vw] items-center justify-center">
				<img
					src={imageUrl}
					alt={screenshot.fileName}
					className="pointer-events-auto max-h-[86vh] max-w-[92vw] select-none rounded-xl object-contain shadow-2xl shadow-black ring-1 ring-white/10 transition-all duration-200"
				/>
			</div>

			{/* Right Navigation Chevron */}
			{hasNext && (
				<Button
					size="icon"
					variant="secondary"
					aria-label="Next screenshot"
					className="absolute right-4 z-20 size-12 rounded-full border border-zinc-700/50 bg-zinc-900/80 text-zinc-200 shadow-2xl backdrop-blur-md transition-all hover:scale-110 hover:bg-zinc-800 hover:text-white active:scale-95"
					onClick={goToNext}
				>
					<ChevronRight className="size-6" />
				</Button>
			)}

			{/* Floating Bottom Metadata Pill */}
			<div className="pointer-events-none absolute inset-x-0 bottom-4 z-20 flex justify-center">
				<div className="pointer-events-auto flex items-center gap-3 rounded-full border border-zinc-800/80 bg-zinc-950/80 px-4 py-1.5 text-xs text-zinc-400 shadow-2xl backdrop-blur-md">
					<span className="font-mono text-[11px] text-zinc-300">
						{formatBytes(screenshot.fileSizeBytes)}
					</span>
					<span className="text-zinc-600">•</span>
					<span>{formatDate(screenshot.modifiedAt || screenshot.createdAt)}</span>
				</div>
			</div>
		</div>,
		document.body,
	)
}

ScreenshotLightbox.displayName = "ScreenshotLightbox"

export default memo(ScreenshotLightbox)
