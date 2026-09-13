import {
	ArrowUpDown,
	Camera,
	ChevronDown,
	Filter,
	FolderOpen,
	HardDrive,
	ImageOff,
	RefreshCw,
	Search,
	Trash2,
	X,
} from "lucide-react"
import { memo, useMemo, useState } from "react"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useInstances } from "@/services/instance-service"
import {
	type ScreenshotInfo,
	screenshotService,
	useScreenshots,
} from "@/services/screenshot-service"
import ScreenshotCard from "./screenshot-card"
import ScreenshotLightbox from "./screenshot-lightbox"

type SortOption = "date-desc" | "date-asc" | "name-asc" | "name-desc" | "size-desc" | "size-asc"

const SORT_LABELS: Record<SortOption, string> = {
	"date-desc": "Newest First",
	"date-asc": "Oldest First",
	"name-asc": "Name (A-Z)",
	"name-desc": "Name (Z-A)",
	"size-desc": "Largest Size",
	"size-asc": "Smallest Size",
}

const ScreenshotsView = () => {
	const {
		screenshots,
		isLoading,
		refresh,
		deleteScreenshot,
		openScreenshotsFolder,
		revealScreenshotFile,
	} = useScreenshots()
	const { instances } = useInstances()

	const [searchQuery, setSearchQuery] = useState("")
	const [selectedInstanceId, setSelectedInstanceId] = useState<string>("all")
	const [sortBy, setSortBy] = useState<SortOption>("date-desc")
	const [activeScreenshot, setActiveScreenshot] = useState<ScreenshotInfo | null>(null)
	const [screenshotToDelete, setScreenshotToDelete] = useState<ScreenshotInfo | null>(null)
	const [isDeleting, setIsDeleting] = useState(false)

	// Build a map of instances for filter dropdown
	const instanceOptions = useMemo(() => {
		const map = new Map<string, string>()
		for (const inst of instances) {
			map.set(inst.id, inst.name)
		}
		for (const s of screenshots) {
			if (!map.has(s.instanceId)) {
				map.set(s.instanceId, s.instanceName)
			}
		}
		return Array.from(map.entries()).map(([id, name]) => ({ id, name }))
	}, [instances, screenshots])

	// Filter and sort screenshots
	const filteredScreenshots = useMemo(() => {
		let list = [...screenshots]

		// Filter by instance
		if (selectedInstanceId !== "all") {
			list = list.filter((s) => s.instanceId === selectedInstanceId)
		}

		// Filter by search query
		if (searchQuery.trim()) {
			const query = searchQuery.trim().toLowerCase()
			list = list.filter(
				(s) =>
					s.fileName.toLowerCase().includes(query) || s.instanceName.toLowerCase().includes(query),
			)
		}

		// Sort
		list.sort((a, b) => {
			const timeA = a.modifiedAt || a.createdAt
			const timeB = b.modifiedAt || b.createdAt

			switch (sortBy) {
				case "date-desc":
					return timeB - timeA
				case "date-asc":
					return timeA - timeB
				case "name-asc":
					return a.fileName.localeCompare(b.fileName)
				case "name-desc":
					return b.fileName.localeCompare(a.fileName)
				case "size-desc":
					return b.fileSizeBytes - a.fileSizeBytes
				case "size-asc":
					return a.fileSizeBytes - b.fileSizeBytes
				default:
					return timeB - timeA
			}
		})

		return list
	}, [screenshots, selectedInstanceId, searchQuery, sortBy])

	const selectedInstanceLabel = useMemo(() => {
		if (selectedInstanceId === "all") return "All Instances"
		const found = instanceOptions.find((opt) => opt.id === selectedInstanceId)
		return found ? found.name : "Selected Instance"
	}, [selectedInstanceId, instanceOptions])

	const handleDeleteConfirm = async () => {
		if (!screenshotToDelete) return
		try {
			setIsDeleting(true)
			await deleteScreenshot(screenshotToDelete.instanceId, screenshotToDelete.fileName)
			if (
				activeScreenshot?.instanceId === screenshotToDelete.instanceId &&
				activeScreenshot?.fileName === screenshotToDelete.fileName
			) {
				setActiveScreenshot(null)
			}
			setScreenshotToDelete(null)
		} catch (error) {
			console.error("Failed to delete screenshot:", error)
		} finally {
			setIsDeleting(false)
		}
	}

	return (
		<div className="flex size-full flex-col overflow-hidden bg-background text-foreground">
			{/* Top Header */}
			<header className="flex shrink-0 items-center justify-between border-border/40 border-b bg-zinc-950/40 px-6 py-4 backdrop-blur-sm">
				<div className="flex items-center gap-3">
					<div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
						<Camera className="size-5" />
					</div>
					<div>
						<div className="flex items-center gap-2">
							<h1 className="font-semibold text-lg tracking-tight">Screenshots</h1>
							<span className="rounded-full bg-zinc-800/80 px-2 py-0.5 font-medium text-xs text-zinc-400">
								{screenshots.length}
							</span>
						</div>
						<p className="text-muted-foreground text-xs">
							Gallery of in-game screenshots captured across all your instances
						</p>
					</div>
				</div>

				{/* Header Actions */}
				<div className="flex items-center gap-2">
					<Button
						variant="outline"
						size="sm"
						className="h-9 gap-2 text-xs"
						onClick={() =>
							openScreenshotsFolder(selectedInstanceId !== "all" ? selectedInstanceId : null)
						}
					>
						<FolderOpen className="size-3.5" />
						<span>Open Folder</span>
					</Button>

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="outline"
									size="icon"
									className="size-9"
									onClick={() => refresh()}
									disabled={isLoading}
								>
									<RefreshCw className={`size-4 ${isLoading ? "animate-spin text-primary" : ""}`} />
								</Button>
							}
						/>
						<TooltipContent side="bottom">Refresh Screenshots</TooltipContent>
					</Tooltip>
				</div>
			</header>

			{/* Filter & Toolbar */}
			<div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-border/40 border-b bg-zinc-950/20 px-6 py-3">
				<div className="flex min-w-[200px] max-w-sm flex-1 items-center gap-2.5">
					<div className="relative w-full">
						<Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
						<Input
							type="text"
							placeholder="Search screenshots..."
							value={searchQuery}
							onChange={(e) => setSearchQuery(e.target.value)}
							className="h-9 w-full bg-zinc-900/60 px-8 text-xs placeholder:text-muted-foreground/60"
						/>
						{searchQuery && (
							<button
								type="button"
								onClick={() => setSearchQuery("")}
								className="absolute top-1/2 right-2.5 -translate-y-1/2 text-muted-foreground hover:text-foreground"
							>
								<X className="size-3.5" />
							</button>
						)}
					</div>
				</div>

				<div className="flex items-center gap-2">
					{/* Instance Filter Dropdown */}
					<DropdownMenu>
						<DropdownMenuTrigger
							render={
								<Button
									variant="outline"
									size="sm"
									className="h-9 max-w-[200px] gap-2 bg-zinc-900/60 text-xs"
								>
									<Filter className="size-3.5 shrink-0 text-muted-foreground" />
									<span className="truncate">{selectedInstanceLabel}</span>
									<ChevronDown className="ml-auto size-3.5 shrink-0 opacity-60" />
								</Button>
							}
						/>
						<DropdownMenuContent align="end" className="max-h-64 w-56 overflow-y-auto">
							<DropdownMenuItem
								className={`text-xs ${selectedInstanceId === "all" ? "font-semibold text-primary" : ""}`}
								onClick={() => setSelectedInstanceId("all")}
							>
								All Instances ({screenshots.length})
							</DropdownMenuItem>
							{instanceOptions.map((opt) => {
								const count = screenshots.filter((s) => s.instanceId === opt.id).length
								return (
									<DropdownMenuItem
										key={opt.id}
										className={`text-xs ${selectedInstanceId === opt.id ? "font-semibold text-primary" : ""}`}
										onClick={() => setSelectedInstanceId(opt.id)}
									>
										<span className="flex-1 truncate">{opt.name}</span>
										<span className="ml-2 text-[10px] text-muted-foreground">{count}</span>
									</DropdownMenuItem>
								)
							})}
						</DropdownMenuContent>
					</DropdownMenu>

					{/* Sort Dropdown */}
					<DropdownMenu>
						<DropdownMenuTrigger
							render={
								<Button variant="outline" size="sm" className="h-9 gap-2 bg-zinc-900/60 text-xs">
									<ArrowUpDown className="size-3.5 text-muted-foreground" />
									<span>{SORT_LABELS[sortBy]}</span>
									<ChevronDown className="size-3.5 opacity-60" />
								</Button>
							}
						/>
						<DropdownMenuContent align="end" className="w-44">
							{(Object.keys(SORT_LABELS) as SortOption[]).map((opt) => (
								<DropdownMenuItem
									key={opt}
									className={`text-xs ${sortBy === opt ? "font-semibold text-primary" : ""}`}
									onClick={() => setSortBy(opt)}
								>
									{SORT_LABELS[opt]}
								</DropdownMenuItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</div>

			{/* Main Gallery Area */}
			<ScrollArea scrollFade className="min-h-0 flex-1 [transform:translateZ(0)]">
				{filteredScreenshots.length > 0 ? (
					<div className="grid grid-cols-1 gap-4 p-6 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
						{filteredScreenshots.map((item) => (
							<ScreenshotCard
								key={`${item.instanceId}-${item.fileName}`}
								screenshot={item}
								onClick={(s) => setActiveScreenshot(s)}
								onDelete={(s) => setScreenshotToDelete(s)}
								onReveal={(s) => revealScreenshotFile(s.filePath)}
							/>
						))}
					</div>
				) : screenshots.length === 0 ? (
					/* No screenshots in any instance */
					<div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
						<div className="flex size-14 items-center justify-center rounded-2xl border border-border/40 bg-zinc-900 text-muted-foreground/60">
							<Camera className="size-7" />
						</div>
						<div className="space-y-1">
							<h3 className="font-semibold text-base text-zinc-200">No screenshots found</h3>
							<p className="max-w-md text-muted-foreground text-xs">
								Take screenshots while playing Minecraft by pressing{" "}
								<kbd className="rounded border border-zinc-700 bg-zinc-800 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300">
									F2
								</kbd>
								. They will automatically show up in this gallery.
							</p>
						</div>
						<Button variant="outline" size="sm" className="mt-2 text-xs" onClick={() => refresh()}>
							<RefreshCw className="mr-2 size-3.5" />
							Check Again
						</Button>
					</div>
				) : (
					/* Filter returned no results */
					<div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
						<div className="flex size-14 items-center justify-center rounded-2xl border border-border/40 bg-zinc-900 text-muted-foreground/60">
							<ImageOff className="size-7" />
						</div>
						<div className="space-y-1">
							<h3 className="font-semibold text-base text-zinc-200">No matching screenshots</h3>
							<p className="max-w-md text-muted-foreground text-xs">
								No screenshots match your search query or filter. Try clearing your filters to see
								more.
							</p>
						</div>
						<Button
							variant="outline"
							size="sm"
							className="mt-2 text-xs"
							onClick={() => {
								setSearchQuery("")
								setSelectedInstanceId("all")
							}}
						>
							Clear Filters
						</Button>
					</div>
				)}
			</ScrollArea>

			{/* Fullscreen Lightbox */}
			<ScreenshotLightbox
				screenshot={activeScreenshot}
				allScreenshots={filteredScreenshots}
				onClose={() => setActiveScreenshot(null)}
				onSelect={(s) => setActiveScreenshot(s)}
				onDelete={(s) => setScreenshotToDelete(s)}
				onReveal={(s) => revealScreenshotFile(s.filePath)}
			/>

			{/* Confirm Delete Dialog */}
			<Dialog
				open={!!screenshotToDelete}
				onOpenChange={(open) => {
					if (!open && !isDeleting) setScreenshotToDelete(null)
				}}
			>
				<DialogContent className="sm:max-w-md">
					<DialogHeader>
						<DialogTitle>Delete Screenshot</DialogTitle>
						<DialogDescription>
							Are you sure you want to permanently delete this screenshot? This action cannot be
							undone.
						</DialogDescription>
					</DialogHeader>

					{screenshotToDelete && (
						<div className="flex flex-col gap-3 py-2">
							<div className="aspect-video w-full overflow-hidden rounded-lg border border-border/40 bg-zinc-900">
								<img
									src={screenshotService.getImageUrl(screenshotToDelete.filePath)}
									alt={screenshotToDelete.fileName}
									className="size-full object-cover"
								/>
							</div>
							<div className="flex items-center justify-between text-xs text-zinc-400">
								<span className="truncate font-medium text-zinc-200">
									{screenshotToDelete.fileName}
								</span>
								<span className="flex shrink-0 items-center gap-1 rounded bg-zinc-800 px-1.5 py-0.5 text-[11px] text-zinc-300">
									<HardDrive className="size-3 text-primary" />
									{screenshotToDelete.instanceName}
								</span>
							</div>
						</div>
					)}

					<DialogFooter className="gap-2 sm:gap-0">
						<Button
							variant="outline"
							size="sm"
							onClick={() => setScreenshotToDelete(null)}
							disabled={isDeleting}
						>
							Cancel
						</Button>
						<Button
							variant="destructive"
							size="sm"
							onClick={handleDeleteConfirm}
							disabled={isDeleting}
							className="gap-1.5"
						>
							<Trash2 className="size-3.5" />
							<span>{isDeleting ? "Deleting..." : "Delete Permanently"}</span>
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	)
}

ScreenshotsView.displayName = "ScreenshotsView"

export default memo(ScreenshotsView)
