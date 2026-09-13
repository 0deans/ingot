import { FolderOpen, HardDrive, ImageOff, Maximize2, MoreVertical, Trash2 } from "lucide-react"
import { memo, useState } from "react"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { type ScreenshotInfo, screenshotService } from "@/services/screenshot-service"

interface ScreenshotCardProps {
	screenshot: ScreenshotInfo
	onClick: (screenshot: ScreenshotInfo) => void
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
		month: "short",
		day: "numeric",
		year: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	})
}

const ScreenshotCard = ({ screenshot, onClick, onDelete, onReveal }: ScreenshotCardProps) => {
	const [imageError, setImageError] = useState(false)
	const [loaded, setLoaded] = useState(false)
	const [menuOpen, setMenuOpen] = useState(false)
	const imageUrl = screenshotService.getImageUrl(screenshot.filePath)

	return (
		<div
			style={{ contentVisibility: "auto", containIntrinsicSize: "220px" }}
			className="group relative flex flex-col overflow-hidden rounded-xl border border-border/40 bg-zinc-950/40 shadow-sm transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-black/40 hover:shadow-lg"
		>
			{/* Accessible full card click-to-fullscreen button */}
			<button
				type="button"
				aria-label={`View ${screenshot.fileName}`}
				onClick={() => onClick(screenshot)}
				className="absolute inset-0 z-10 size-full cursor-pointer border-none bg-transparent outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
			/>

			{/* Thumbnail container */}
			<div className="relative aspect-video w-full overflow-hidden bg-zinc-900/60">
				{!loaded && !imageError && (
					<div className="absolute inset-0 animate-pulse bg-zinc-800/40" />
				)}

				{imageError ? (
					<div className="flex size-full flex-col items-center justify-center gap-1.5 p-3 text-center text-muted-foreground">
						<ImageOff className="size-6 opacity-40" />
						<span className="text-[11px]">Image unavailable</span>
					</div>
				) : (
					<img
						src={imageUrl}
						alt={screenshot.fileName}
						loading="lazy"
						decoding="async"
						onLoad={() => setLoaded(true)}
						onError={() => setImageError(true)}
						className={`size-full object-cover transition-transform duration-300 group-hover:scale-105 ${
							loaded ? "opacity-100" : "opacity-0"
						}`}
					/>
				)}

				{/* Top gradient for readability */}
				<div className="pointer-events-none absolute inset-x-0 top-0 h-10 bg-gradient-to-b from-black/60 to-transparent" />

				{/* Top-left instance tag (clean opaque bg, no heavy backdrop-blur) */}
				<div className="pointer-events-none absolute top-2 left-2 z-10">
					<span className="inline-flex max-w-[140px] items-center gap-1 truncate rounded-md border border-zinc-800/90 bg-zinc-950/90 px-2 py-0.5 font-medium text-[10px] text-zinc-300 shadow-sm">
						<HardDrive className="size-2.5 shrink-0 text-primary" />
						<span className="truncate">{screenshot.instanceName}</span>
					</span>
				</div>

				{/* Top-right subtle 3-dots action menu (does not block click-to-fullscreen) */}
				<div
					className={`absolute top-1.5 right-1.5 z-20 transition-opacity duration-150 ${
						menuOpen ? "opacity-100" : "opacity-0 group-hover:opacity-100"
					}`}
				>
					<DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
						<DropdownMenuTrigger
							render={
								<Button
									size="icon"
									variant="secondary"
									className="size-7 rounded-lg border border-zinc-700/60 bg-zinc-950/90 text-zinc-300 shadow-md hover:bg-zinc-800 hover:text-white"
									onClick={(e) => e.stopPropagation()}
								>
									<MoreVertical className="size-3.5" />
								</Button>
							}
						/>
						<DropdownMenuContent align="end" className="w-44">
							<DropdownMenuItem
								className="gap-2 text-xs"
								onClick={() => {
									setMenuOpen(false)
									onClick(screenshot)
								}}
							>
								<Maximize2 className="size-3.5 text-zinc-400" />
								<span>Fullscreen</span>
							</DropdownMenuItem>
							<DropdownMenuItem
								className="gap-2 text-xs"
								onClick={() => {
									setMenuOpen(false)
									onReveal(screenshot)
								}}
							>
								<FolderOpen className="size-3.5 text-zinc-400" />
								<span>Show in Folder</span>
							</DropdownMenuItem>
							<DropdownMenuSeparator />
							<DropdownMenuItem
								className="gap-2 text-red-400 text-xs focus:text-red-300"
								onClick={() => {
									setMenuOpen(false)
									onDelete(screenshot)
								}}
							>
								<Trash2 className="size-3.5" />
								<span>Delete</span>
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</div>

			{/* Info strip */}
			<div className="pointer-events-none relative z-10 flex flex-col gap-1 p-2.5">
				<div className="flex items-center justify-between gap-2">
					<span
						title={screenshot.fileName}
						className="truncate font-medium text-xs text-zinc-200 transition-colors group-hover:text-primary"
					>
						{screenshot.fileName}
					</span>
					<span className="shrink-0 font-mono text-[10px] text-zinc-400">
						{formatBytes(screenshot.fileSizeBytes)}
					</span>
				</div>
				<div className="text-[10px] text-zinc-500">
					{formatDate(screenshot.modifiedAt || screenshot.createdAt)}
				</div>
			</div>
		</div>
	)
}

ScreenshotCard.displayName = "ScreenshotCard"

export default memo(ScreenshotCard)
