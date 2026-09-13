import { createFileRoute } from "@tanstack/react-router"
import {
	Box,
	Check,
	ChevronLeft,
	ChevronRight,
	Download,
	Flame,
	Info,
	Package,
	Paintbrush,
	RefreshCw,
	Search,
	Sparkles,
	SunMedium,
	X,
} from "lucide-react"
import { memo, useCallback, useEffect, useRef, useState } from "react"
import { ContentDetailsDialog } from "@/components/content/content-details-dialog"
import { InstallDialog } from "@/components/content/install-dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import {
	type ContentSort,
	type ContentSource,
	type ContentType,
	contentService,
	type UnifiedContentItem,
	type UnifiedContentVersion,
} from "@/services/content-service"

const CATEGORIES: { id: ContentType; label: string; icon: typeof Sparkles }[] = [
	{ id: "all", label: "All Types", icon: Sparkles },
	{ id: "modpack", label: "Modpacks", icon: Package },
	{ id: "mod", label: "Mods", icon: Box },
	{ id: "resourcepack", label: "Resource Packs", icon: Paintbrush },
	{ id: "shader", label: "Shaders", icon: SunMedium },
]

const SOURCES: { id: ContentSource; label: string }[] = [
	{ id: "all", label: "All Sources" },
	{ id: "modrinth", label: "Modrinth" },
	{ id: "curseforge", label: "CurseForge" },
]

const LOADERS = [
	{ id: "", label: "All Loaders" },
	{ id: "fabric", label: "Fabric" },
	{ id: "forge", label: "Forge" },
	{ id: "neoforge", label: "NeoForge" },
	{ id: "quilt", label: "Quilt" },
]

const POPULAR_VERSIONS = [
	{ id: "", label: "All Versions" },
	{ id: "1.21.4", label: "1.21.4" },
	{ id: "1.21.1", label: "1.21.1" },
	{ id: "1.20.4", label: "1.20.4" },
	{ id: "1.20.1", label: "1.20.1" },
	{ id: "1.19.2", label: "1.19.2" },
	{ id: "1.16.5", label: "1.16.5" },
	{ id: "1.12.2", label: "1.12.2" },
	{ id: "1.7.10", label: "1.7.10" },
]

const SORTS: { id: ContentSort; label: string }[] = [
	{ id: "downloads", label: "Most Downloads" },
	{ id: "relevance", label: "Relevance" },
	{ id: "updated", label: "Recently Updated" },
	{ id: "newest", label: "Newest" },
]

function formatDownloads(count: number): string {
	if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`
	if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
	if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`
	return count.toString()
}

const PAGE_SIZE = 24

const ModpacksPage = () => {
	const [searchQuery, setSearchQuery] = useState("")
	const [activeCategory, setActiveCategory] = useState<ContentType>("all")
	const [activeSource, setActiveSource] = useState<ContentSource>("all")
	const [activeLoader, setActiveLoader] = useState("")
	const [activeVersion, setActiveVersion] = useState("")
	const [activeSort, setActiveSort] = useState<ContentSort>("downloads")
	const [page, setPage] = useState(0)

	const [items, setItems] = useState<UnifiedContentItem[]>([])
	const [totalHits, setTotalHits] = useState(0)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<string | null>(null)

	// Modal States
	const [detailsItem, setDetailsItem] = useState<UnifiedContentItem | null>(null)
	const [installItem, setInstallItem] = useState<UnifiedContentItem | null>(null)
	const [installVersion, setInstallVersion] = useState<UnifiedContentVersion | null>(null)
	const [successNotification, setSuccessNotification] = useState<string | null>(null)

	const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
	const searchCountRef = useRef(0)

	const fetchContent = useCallback(
		async (
			query: string,
			category: ContentType,
			source: ContentSource,
			loader: string,
			version: string,
			sort: ContentSort,
			pageNum: number,
		) => {
			const reqId = ++searchCountRef.current
			setIsLoading(true)
			setError(null)

			try {
				const result = await contentService.searchContent({
					source,
					projectType: category,
					query: query || null,
					gameVersion: version || null,
					loader: loader || null,
					sort,
					page: pageNum,
					pageSize: PAGE_SIZE,
				})

				if (reqId === searchCountRef.current) {
					setItems(result.items)
					setTotalHits(result.totalHits)
				}
			} catch (err) {
				if (reqId === searchCountRef.current) {
					setError(err instanceof Error ? err.message : "Failed to load content")
				}
			} finally {
				if (reqId === searchCountRef.current) {
					setIsLoading(false)
				}
			}
		},
		[],
	)

	// Perform fetch with debounce on query change
	useEffect(() => {
		if (debounceTimerRef.current) {
			clearTimeout(debounceTimerRef.current)
		}

		debounceTimerRef.current = setTimeout(() => {
			fetchContent(
				searchQuery,
				activeCategory,
				activeSource,
				activeLoader,
				activeVersion,
				activeSort,
				page,
			)
		}, 300)

		return () => {
			if (debounceTimerRef.current) {
				clearTimeout(debounceTimerRef.current)
			}
		}
	}, [
		searchQuery,
		activeCategory,
		activeSource,
		activeLoader,
		activeVersion,
		activeSort,
		page,
		fetchContent,
	])

	const handleCategoryChange = (cat: ContentType) => {
		setActiveCategory(cat)
		setPage(0)
	}

	const handleSourceChange = (src: ContentSource) => {
		setActiveSource(src)
		setPage(0)
	}

	const handleLoaderChange = (ldr: string) => {
		setActiveLoader(ldr)
		setPage(0)
	}

	const handleVersionChange = (ver: string) => {
		setActiveVersion(ver)
		setPage(0)
	}

	const handleSortChange = (srt: ContentSort) => {
		setActiveSort(srt)
		setPage(0)
	}

	const handleClearFilters = () => {
		setSearchQuery("")
		setActiveCategory("all")
		setActiveSource("all")
		setActiveLoader("")
		setActiveVersion("")
		setActiveSort("downloads")
		setPage(0)
	}

	const handleCardClick = (item: UnifiedContentItem) => {
		setDetailsItem(item)
	}

	const handleStartInstall = (item: UnifiedContentItem, specificVer?: UnifiedContentVersion) => {
		setInstallItem(item)
		setInstallVersion(specificVer ?? null)
	}

	const handleInstallSuccess = (message: string) => {
		setSuccessNotification(message)
		setTimeout(() => setSuccessNotification(null), 5000)
	}

	const totalPages = Math.ceil(totalHits / PAGE_SIZE)

	return (
		<ScrollArea className="flex-1" scrollFade>
			<div className="flex flex-col gap-6 p-6">
				{/* Notification Banner */}
				{successNotification && (
					<div className="flex items-center justify-between rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-3 text-emerald-300 text-xs shadow-sm">
						<div className="flex items-center gap-2">
							<Check className="size-4 text-emerald-400" />
							<span className="font-medium">{successNotification}</span>
						</div>
						<button
							type="button"
							onClick={() => setSuccessNotification(null)}
							className="text-muted-foreground hover:text-foreground"
						>
							<X className="size-3.5" />
						</button>
					</div>
				)}

				{/* Header */}
				<div className="flex flex-col gap-2">
					<div className="flex items-center gap-2">
						<Sparkles className="size-6 text-primary" />
						<h1 className="font-bold text-2xl text-foreground tracking-tight">Discover Content</h1>
					</div>
					<p className="text-muted-foreground text-sm">
						Search and install mods, modpacks, resource packs, and shaders directly into your
						instances from Modrinth and CurseForge.
					</p>
				</div>

				{/* Search & Category Tabs */}
				<div className="flex flex-col gap-4">
					<div className="flex flex-wrap items-center gap-3">
						{/* Search Input */}
						<div className="relative min-w-[280px] flex-1">
							<Search className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
							<Input
								type="text"
								value={searchQuery}
								onChange={(e) => {
									setSearchQuery(e.target.value)
									setPage(0)
								}}
								placeholder="Search mods, modpacks, resource packs, shaders..."
								className="h-10 px-9 text-sm"
							/>
							{searchQuery && (
								<button
									type="button"
									onClick={() => {
										setSearchQuery("")
										setPage(0)
									}}
									aria-label="Clear search"
									className="absolute top-1/2 right-3 -translate-y-1/2 text-muted-foreground hover:text-foreground"
								>
									<X className="size-4" />
								</button>
							)}
						</div>

						{/* Source Selector */}
						<div className="flex items-center rounded-lg border border-border/40 bg-zinc-900/40 p-1 backdrop-blur-xs">
							{SOURCES.map((s) => (
								<button
									key={s.id}
									type="button"
									onClick={() => handleSourceChange(s.id)}
									className={`flex items-center gap-1.5 rounded-md px-3 py-1.5 font-medium text-xs transition-colors ${
										activeSource === s.id
											? "bg-primary text-primary-foreground shadow-xs"
											: "text-muted-foreground hover:bg-zinc-800/60 hover:text-foreground"
									}`}
								>
									{s.id === "curseforge" && <Flame className="size-3.5 text-orange-400" />}
									{s.label}
								</button>
							))}
						</div>
					</div>

					{/* Category Tabs */}
					<div className="flex flex-wrap items-center gap-2 border-border/40 border-b pb-3">
						{CATEGORIES.map((cat) => {
							const IconComponent = cat.icon
							const isSelected = activeCategory === cat.id
							return (
								<button
									key={cat.id}
									type="button"
									onClick={() => handleCategoryChange(cat.id)}
									className={`flex items-center gap-2 rounded-lg px-3.5 py-1.5 font-medium text-xs transition-all ${
										isSelected
											? "border border-primary/40 bg-primary/15 text-primary"
											: "border border-transparent bg-zinc-900/30 text-muted-foreground hover:bg-zinc-900/60 hover:text-foreground"
									}`}
								>
									<IconComponent className="size-3.5" />
									{cat.label}
								</button>
							)
						})}
					</div>

					{/* Filter Dropdowns / Options */}
					<div className="flex flex-wrap items-center gap-3">
						{/* Loader Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">Loader:</span>
							<Select
								value={activeLoader || "all"}
								onValueChange={(val) => handleLoaderChange(!val || val === "all" ? "" : val)}
							>
								<SelectTrigger className="h-8 w-28 text-xs">
									<SelectValue placeholder="All Loaders" />
								</SelectTrigger>
								<SelectContent>
									{LOADERS.map((ldr) => (
										<SelectItem key={ldr.id || "all"} value={ldr.id || "all"}>
											{ldr.label}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>

						{/* Version Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">Version:</span>
							<Select
								value={activeVersion || "all"}
								onValueChange={(val) => handleVersionChange(!val || val === "all" ? "" : val)}
							>
								<SelectTrigger className="h-8 w-32 text-xs">
									<SelectValue placeholder="All Versions" />
								</SelectTrigger>
								<SelectContent>
									{POPULAR_VERSIONS.map((v) => (
										<SelectItem key={v.id || "all"} value={v.id || "all"}>
											{v.label}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>

						{/* Sort Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">Sort:</span>
							<Select
								value={activeSort}
								onValueChange={(val) => val && handleSortChange(val as ContentSort)}
							>
								<SelectTrigger className="h-8 w-36 text-xs">
									<SelectValue placeholder="Sort by" />
								</SelectTrigger>
								<SelectContent>
									{SORTS.map((s) => (
										<SelectItem key={s.id} value={s.id}>
											{s.label}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>

						{/* Active Filters Clear */}
						{(searchQuery ||
							activeCategory !== "all" ||
							activeSource !== "all" ||
							activeLoader ||
							activeVersion ||
							activeSort !== "downloads") && (
							<Button
								size="sm"
								variant="ghost"
								onClick={handleClearFilters}
								className="h-8 gap-1 px-2.5 text-muted-foreground text-xs hover:text-foreground"
							>
								<X className="size-3" />
								Reset filters
							</Button>
						)}

						{/* Refresh */}
						<Button
							size="sm"
							variant="ghost"
							disabled={isLoading}
							onClick={() =>
								fetchContent(
									searchQuery,
									activeCategory,
									activeSource,
									activeLoader,
									activeVersion,
									activeSort,
									page,
								)
							}
							className="ml-auto h-8 gap-1 text-muted-foreground text-xs hover:text-foreground"
						>
							<RefreshCw className={`size-3.5 ${isLoading ? "animate-spin" : ""}`} />
							Refresh
						</Button>
					</div>
				</div>

				{/* Results Meta */}
				<div className="flex items-center justify-between">
					<span className="text-muted-foreground text-xs">
						{isLoading ? (
							"Searching content..."
						) : totalHits > 0 ? (
							<>
								Found <strong className="text-foreground">{totalHits.toLocaleString()}</strong>{" "}
								results
							</>
						) : (
							"No results"
						)}
					</span>

					{totalPages > 1 && (
						<div className="flex items-center gap-2">
							<Button
								size="sm"
								variant="outline"
								disabled={page === 0 || isLoading}
								onClick={() => setPage((p) => Math.max(0, p - 1))}
								className="size-7 p-0"
							>
								<ChevronLeft className="size-3.5" />
							</Button>
							<span className="text-muted-foreground text-xs">
								Page {page + 1} of {Math.max(1, totalPages)}
							</span>
							<Button
								size="sm"
								variant="outline"
								disabled={page >= totalPages - 1 || isLoading}
								onClick={() => setPage((p) => p + 1)}
								className="size-7 p-0"
							>
								<ChevronRight className="size-3.5" />
							</Button>
						</div>
					)}
				</div>

				{/* Content Grid */}
				{isLoading && items.length === 0 ? (
					<div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
						{Array.from({ length: 9 }).map((_, i) => (
							<div
								// biome-ignore lint/suspicious/noArrayIndexKey: Loading skeleton placeholder
								key={i}
								className="flex animate-pulse flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/30 p-4"
							>
								<div className="flex items-center gap-3">
									<div className="size-12 rounded-lg bg-zinc-800" />
									<div className="flex flex-1 flex-col gap-2">
										<div className="h-4 w-3/4 rounded bg-zinc-800" />
										<div className="h-3 w-1/2 rounded bg-zinc-800/60" />
									</div>
								</div>
								<div className="h-10 rounded bg-zinc-800/40" />
								<div className="mt-2 flex justify-between">
									<div className="h-4 w-16 rounded bg-zinc-800/50" />
									<div className="h-4 w-16 rounded bg-zinc-800/50" />
								</div>
							</div>
						))}
					</div>
				) : error ? (
					<div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-destructive/30 bg-destructive/5 py-12 text-center">
						<p className="font-medium text-destructive text-sm">{error}</p>
						<Button
							size="sm"
							variant="outline"
							onClick={() =>
								fetchContent(
									searchQuery,
									activeCategory,
									activeSource,
									activeLoader,
									activeVersion,
									activeSort,
									page,
								)
							}
						>
							Try Again
						</Button>
					</div>
				) : items.length === 0 ? (
					<div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-border/40 bg-zinc-900/20 py-16 text-center">
						<Sparkles className="size-10 text-muted-foreground/40" />
						<div className="flex flex-col gap-1">
							<p className="font-semibold text-foreground text-sm">
								No content matches your filters
							</p>
							<p className="text-muted-foreground text-xs">
								Try adjusting your search query, changing mod loaders, or switching sources.
							</p>
						</div>
						<Button
							size="sm"
							variant="outline"
							onClick={handleClearFilters}
							className="mt-2 text-xs"
						>
							Reset all filters
						</Button>
					</div>
				) : (
					<div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
						{items.map((item) => (
							// biome-ignore lint/a11y/useKeyWithClickEvents: Clicking card opens details dialog
							// biome-ignore lint/a11y/noStaticElementInteractions: Clicking card opens details dialog
							<div
								key={`${item.source}-${item.id}`}
								onClick={() => handleCardClick(item)}
								className="group flex cursor-pointer flex-col justify-between rounded-xl border border-border/40 bg-zinc-900/40 p-4 backdrop-blur-xs transition-all hover:border-primary/40 hover:bg-zinc-900/70"
							>
								<div className="flex flex-col gap-3">
									{/* Top row: Icon + Info */}
									<div className="flex items-start gap-3">
										{item.iconUrl ? (
											<img
												src={item.iconUrl}
												alt={item.title}
												className="size-12 shrink-0 rounded-lg bg-zinc-800 object-cover"
												loading="lazy"
												onError={(e) => {
													;(e.currentTarget as HTMLElement).style.display = "none"
												}}
											/>
										) : (
											<div className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-zinc-800 font-bold text-muted-foreground text-sm">
												{item.title.charAt(0).toUpperCase()}
											</div>
										)}

										<div className="flex min-w-0 flex-1 flex-col">
											<div className="flex items-center gap-1.5">
												{/* Source Badge */}
												<span
													className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-medium text-[10px] ${
														item.source === "modrinth"
															? "border border-emerald-500/20 bg-emerald-500/15 text-emerald-400"
															: "border border-amber-500/20 bg-amber-500/15 text-amber-400"
													}`}
												>
													{item.source === "curseforge" && <Flame className="size-2.5" />}
													{item.source === "modrinth" ? "Modrinth" : "CurseForge"}
												</span>

												{/* Type Badge */}
												<span className="rounded bg-zinc-800/80 px-1.5 py-0.5 text-[10px] text-muted-foreground capitalize">
													{item.projectType}
												</span>
											</div>

											<h3 className="mt-1 truncate font-semibold text-foreground text-sm transition-colors group-hover:text-primary">
												{item.title}
											</h3>
											<span className="truncate text-muted-foreground text-xs">
												by {item.author}
											</span>
										</div>
									</div>

									{/* Description */}
									<p className="line-clamp-2 text-muted-foreground text-xs leading-relaxed">
										{item.description || "No description provided."}
									</p>

									{/* Tags & Loaders */}
									<div className="flex flex-wrap items-center gap-1">
										{item.loaders.slice(0, 3).map((ldr) => (
											<span
												key={ldr}
												className="rounded bg-zinc-800/60 px-1.5 py-0.5 font-mono text-[10px] text-zinc-400 capitalize"
											>
												{ldr}
											</span>
										))}
										{item.latestVersion && (
											<span className="rounded bg-primary/10 px-1.5 py-0.5 font-mono text-[10px] text-primary">
												{item.latestVersion}
											</span>
										)}
									</div>
								</div>

								{/* Card Footer */}
								<div className="mt-4 flex items-center justify-between border-border/30 border-t pt-3">
									<div className="flex items-center gap-1 text-muted-foreground text-xs">
										<Download className="size-3.5" />
										<span>{formatDownloads(item.downloads)}</span>
									</div>

									<div className="flex items-center gap-1.5">
										<Button
											size="sm"
											variant="ghost"
											onClick={(e) => {
												e.stopPropagation()
												handleCardClick(item)
											}}
											className="h-7 px-2 text-muted-foreground text-xs hover:text-foreground"
										>
											<Info className="size-3" />
											Details
										</Button>

										<Button
											size="sm"
											variant="default"
											onClick={(e) => {
												e.stopPropagation()
												handleStartInstall(item)
											}}
											className="h-7 gap-1 px-2.5 font-semibold text-xs"
										>
											<Download className="size-3" />
											{item.projectType === "modpack" ? "Install" : "Add"}
										</Button>
									</div>
								</div>
							</div>
						))}
					</div>
				)}

				{/* Bottom Pagination */}
				{totalPages > 1 && (
					<div className="mt-2 flex items-center justify-center gap-2">
						<Button
							size="sm"
							variant="outline"
							disabled={page === 0 || isLoading}
							onClick={() => {
								setPage((p) => Math.max(0, p - 1))
								window.scrollTo({ top: 0, behavior: "smooth" })
							}}
							className="gap-1 text-xs"
						>
							<ChevronLeft className="size-3.5" />
							Previous
						</Button>
						<span className="text-muted-foreground text-xs">
							Page {page + 1} of {Math.max(1, totalPages)}
						</span>
						<Button
							size="sm"
							variant="outline"
							disabled={page >= totalPages - 1 || isLoading}
							onClick={() => {
								setPage((p) => p + 1)
								window.scrollTo({ top: 0, behavior: "smooth" })
							}}
							className="gap-1 text-xs"
						>
							Next
							<ChevronRight className="size-3.5" />
						</Button>
					</div>
				)}
			</div>

			{/* Details Modal */}
			<ContentDetailsDialog
				open={detailsItem !== null}
				onOpenChange={(open) => {
					if (!open) setDetailsItem(null)
				}}
				item={detailsItem}
				onInstall={(item, ver) => {
					setDetailsItem(null)
					handleStartInstall(item, ver)
				}}
			/>

			{/* Install Modal */}
			<InstallDialog
				open={installItem !== null}
				onOpenChange={(open) => {
					if (!open) {
						setInstallItem(null)
						setInstallVersion(null)
					}
				}}
				item={installItem}
				specificVersion={installVersion}
				onSuccess={handleInstallSuccess}
			/>
		</ScrollArea>
	)
}

ModpacksPage.displayName = "ModpacksPage"

const MemoizedModpacksPage = memo(ModpacksPage)

export const Route = createFileRoute("/modpacks")({
	component: MemoizedModpacksPage,
})
