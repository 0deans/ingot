import { queryOptions, useQuery } from "@tanstack/react-query"
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
import { memo, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import * as v from "valibot"
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
import { formatCount } from "@/lib/format"
import {
	type ContentSort,
	type ContentSource,
	type ContentType,
	contentService,
	type UnifiedContentItem,
	type UnifiedContentVersion,
} from "@/services/content-service"

export const categorySchema = v.picklist(["all", "modpack", "mod", "resourcepack", "shader"])
export const sourceSchema = v.picklist(["all", "modrinth", "curseforge"])
export const sortSchema = v.picklist(["downloads", "relevance", "updated", "newest"])

export const modpacksSearchSchema = v.object({
	query: v.optional(v.fallback(v.string(), ""), ""),
	category: v.optional(v.fallback(categorySchema, "all"), "all"),
	source: v.optional(v.fallback(sourceSchema, "all"), "all"),
	loader: v.optional(v.fallback(v.string(), ""), ""),
	version: v.optional(v.fallback(v.string(), ""), ""),
	sort: v.optional(v.fallback(sortSchema, "downloads"), "downloads"),
	page: v.optional(v.fallback(v.number(), 0), 0),
	details: v.optional(v.string()),
})

export type ModpacksSearchParams = v.InferOutput<typeof modpacksSearchSchema>

const CATEGORIES = [
	{ id: "all" as const, icon: Sparkles },
	{ id: "modpack" as const, icon: Package },
	{ id: "mod" as const, icon: Box },
	{ id: "resourcepack" as const, icon: Paintbrush },
	{ id: "shader" as const, icon: SunMedium },
]

const SOURCES = [{ id: "all" as const }, { id: "modrinth" as const }, { id: "curseforge" as const }]

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

const SORTS = [
	{ id: "downloads" as const },
	{ id: "relevance" as const },
	{ id: "updated" as const },
	{ id: "newest" as const },
]

const formatDownloads = formatCount

const PAGE_SIZE = 24

export const modpacksQueryOptions = (search: ModpacksSearchParams) =>
	queryOptions({
		queryKey: [
			"content",
			"search",
			{
				query: search.query ?? "",
				category: search.category ?? "all",
				source: search.source ?? "all",
				loader: search.loader ?? "",
				version: search.version ?? "",
				sort: search.sort ?? "downloads",
				page: search.page ?? 0,
			},
		],
		queryFn: () =>
			contentService.searchContent({
				source: search.source ?? "all",
				projectType: search.category ?? "all",
				query: search.query?.trim() ? search.query.trim() : null,
				gameVersion: search.version?.trim() ? search.version.trim() : null,
				loader: search.loader?.trim() ? search.loader.trim() : null,
				sort: search.sort ?? "downloads",
				page: search.page ?? 0,
				pageSize: PAGE_SIZE,
			}),
	})

const ModpacksPage = () => {
	const { t } = useTranslation()
	const search = Route.useSearch()
	const navigate = Route.useNavigate()

	const activeCategory = search.category ?? "all"
	const activeSource = search.source ?? "all"
	const activeLoader = search.loader ?? ""
	const activeVersion = search.version ?? ""
	const activeSort = search.sort ?? "downloads"
	const page = search.page ?? 0
	const urlQuery = search.query ?? ""
	const detailsId = search.details

	const [searchInput, setSearchInput] = useState(urlQuery)

	// Keep input synced if URL search param changes (e.g. back/forward navigation)
	useEffect(() => {
		setSearchInput(urlQuery)
	}, [urlQuery])

	// Debounce input to URL search parameters
	useEffect(() => {
		const timer = setTimeout(() => {
			if (searchInput !== urlQuery) {
				navigate({
					search: (prev) => ({ ...prev, query: searchInput, page: 0 }),
					replace: true,
				})
			}
		}, 300)

		return () => clearTimeout(timer)
	}, [searchInput, urlQuery, navigate])

	// TanStack Query integration
	const { data, isLoading, isFetching, error, refetch } = useQuery(
		modpacksQueryOptions({
			query: urlQuery,
			category: activeCategory,
			source: activeSource,
			loader: activeLoader,
			version: activeVersion,
			sort: activeSort,
			page,
		}),
	)

	const items = data?.items ?? []
	const totalHits = data?.totalHits ?? 0

	// Details dialog is derived directly from the URL search param
	const detailsItem = detailsId ? (items.find((i) => i.id === detailsId) ?? null) : null

	const [installItem, setInstallItem] = useState<UnifiedContentItem | null>(null)
	const [installVersion, setInstallVersion] = useState<UnifiedContentVersion | null>(null)
	const [successNotification, setSuccessNotification] = useState<string | null>(null)

	const handleCategoryChange = (category: ContentType) => {
		navigate({
			search: (prev) => ({ ...prev, category, page: 0 }),
			replace: true,
		})
	}

	const handleSourceChange = (source: ContentSource) => {
		navigate({
			search: (prev) => ({ ...prev, source, page: 0 }),
			replace: true,
		})
	}

	const handleLoaderChange = (loader: string) => {
		navigate({
			search: (prev) => ({ ...prev, loader, page: 0 }),
			replace: true,
		})
	}

	const handleVersionChange = (version: string) => {
		navigate({
			search: (prev) => ({ ...prev, version, page: 0 }),
			replace: true,
		})
	}

	const handleSortChange = (sort: ContentSort) => {
		navigate({
			search: (prev) => ({ ...prev, sort, page: 0 }),
			replace: true,
		})
	}

	const handlePageChange = (newPage: number) => {
		navigate({
			search: (prev) => ({ ...prev, page: newPage }),
			replace: true,
		})
	}

	const handleClearFilters = () => {
		setSearchInput("")
		navigate({
			search: () => ({
				query: "",
				category: "all",
				source: "all",
				loader: "",
				version: "",
				sort: "downloads",
				page: 0,
				details: undefined,
			}),
			replace: true,
		})
	}

	const handleOpenDetails = (item: UnifiedContentItem) => {
		navigate({
			search: (prev) => ({ ...prev, details: item.id }),
			replace: true,
		})
	}

	const handleCloseDetails = () => {
		navigate({
			search: (prev) => ({ ...prev, details: undefined }),
			replace: true,
		})
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
	const isBusy = isLoading || isFetching

	return (
		<ScrollArea className="size-full flex-1" scrollFade>
			<div className="flex flex-col gap-6 p-4 pb-12 sm:p-5 lg:p-6">
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
						<h1 className="font-bold text-2xl text-foreground tracking-tight">
							{t("modpacks.title")}
						</h1>
					</div>
					<p className="text-muted-foreground text-sm">{t("modpacks.subtitle")}</p>
				</div>

				{/* Search & Category Tabs */}
				<div className="flex flex-col gap-4">
					<div className="flex flex-wrap items-center gap-3">
						{/* Search Input */}
						<div className="relative min-w-[280px] flex-1">
							<Search className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
							<Input
								type="text"
								value={searchInput}
								onChange={(e) => setSearchInput(e.target.value)}
								placeholder={t("modpacks.searchPlaceholder")}
								className="h-10 px-9 text-sm"
							/>
							{searchInput && (
								<button
									type="button"
									onClick={() => {
										setSearchInput("")
										navigate({
											search: (prev) => ({ ...prev, query: "", page: 0 }),
											replace: true,
										})
									}}
									aria-label={t("common.clearSearch")}
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
									{t(`modpacks.sources.${s.id}`)}
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
									{t(`modpacks.categories.${cat.id}`)}
								</button>
							)
						})}
					</div>

					{/* Filter Dropdowns / Options */}
					<div className="flex flex-wrap items-center gap-3">
						{/* Loader Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">{t("modpacks.filters.loader")}</span>
							<Select
								value={activeLoader || "all"}
								onValueChange={(val) => handleLoaderChange(!val || val === "all" ? "" : val)}
							>
								<SelectTrigger className="h-8 w-28 text-xs">
									<SelectValue placeholder={t("modpacks.filters.allLoaders")} />
								</SelectTrigger>
								<SelectContent>
									{LOADERS.map((ldr) => (
										<SelectItem key={ldr.id || "all"} value={ldr.id || "all"}>
											{ldr.id ? ldr.label : t("modpacks.filters.allLoaders")}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>

						{/* Version Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">{t("modpacks.filters.version")}</span>
							<Select
								value={activeVersion || "all"}
								onValueChange={(val) => handleVersionChange(!val || val === "all" ? "" : val)}
							>
								<SelectTrigger className="h-8 w-32 text-xs">
									<SelectValue placeholder={t("modpacks.filters.allVersions")} />
								</SelectTrigger>
								<SelectContent>
									{POPULAR_VERSIONS.map((v) => (
										<SelectItem key={v.id || "all"} value={v.id || "all"}>
											{v.id ? v.label : t("modpacks.filters.allVersions")}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>

						{/* Sort Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">{t("modpacks.filters.sort")}</span>
							<Select
								value={activeSort}
								onValueChange={(val) => val && v.is(sortSchema, val) && handleSortChange(val)}
							>
								<SelectTrigger className="h-8 w-36 text-xs">
									<SelectValue placeholder={t("modpacks.filters.sortBy")} />
								</SelectTrigger>
								<SelectContent>
									{SORTS.map((s) => (
										<SelectItem key={s.id} value={s.id}>
											{t(`modpacks.sorts.${s.id}`)}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>

						{/* Active Filters Clear */}
						{(urlQuery ||
							activeCategory !== "all" ||
							activeSource !== "all" ||
							activeLoader ||
							activeVersion ||
							activeSort !== "downloads" ||
							detailsId) && (
							<Button
								size="sm"
								variant="ghost"
								onClick={handleClearFilters}
								className="h-8 gap-1 px-2.5 text-muted-foreground text-xs hover:text-foreground"
							>
								<X className="size-3" />
								{t("common.resetFilters")}
							</Button>
						)}

						{/* Refresh */}
						<Button
							size="sm"
							variant="ghost"
							disabled={isBusy}
							onClick={() => refetch()}
							className="ml-auto h-8 gap-1 text-muted-foreground text-xs hover:text-foreground"
						>
							<RefreshCw className={`size-3.5 ${isBusy ? "animate-spin" : ""}`} />
							{t("common.refresh")}
						</Button>
					</div>
				</div>

				{/* Results Meta */}
				<div className="flex items-center justify-between">
					<span className="text-muted-foreground text-xs">
						{isLoading
							? t("modpacks.searching")
							: totalHits > 0
								? t("modpacks.foundResults", { count: totalHits })
								: t("modpacks.noResults")}
					</span>

					{totalPages > 1 && (
						<div className="flex items-center gap-2">
							<Button
								size="sm"
								variant="outline"
								disabled={page === 0 || isBusy}
								onClick={() => handlePageChange(Math.max(0, page - 1))}
								className="size-7 p-0"
							>
								<ChevronLeft className="size-3.5" />
							</Button>
							<span className="text-muted-foreground text-xs">
								{t("modpacks.pageOf", { page: page + 1, total: Math.max(1, totalPages) })}
							</span>
							<Button
								size="sm"
								variant="outline"
								disabled={page >= totalPages - 1 || isBusy}
								onClick={() => handlePageChange(page + 1)}
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
						<p className="font-medium text-destructive text-sm">
							{error instanceof Error ? error.message : t("modpacks.loadFailed")}
						</p>
						<Button size="sm" variant="outline" onClick={() => refetch()}>
							{t("common.tryAgain")}
						</Button>
					</div>
				) : items.length === 0 ? (
					<div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-border/40 bg-zinc-900/20 py-16 text-center">
						<Sparkles className="size-10 text-muted-foreground/40" />
						<div className="flex flex-col gap-1">
							<p className="font-semibold text-foreground text-sm">{t("modpacks.noMatchTitle")}</p>
							<p className="text-muted-foreground text-xs">{t("modpacks.noMatchSubtitle")}</p>
						</div>
						<Button
							size="sm"
							variant="outline"
							onClick={handleClearFilters}
							className="mt-2 text-xs"
						>
							{t("modpacks.resetAllFilters")}
						</Button>
					</div>
				) : (
					<div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
						{items.map((item) => (
							// biome-ignore lint/a11y/useKeyWithClickEvents: Clicking card opens details dialog
							// biome-ignore lint/a11y/noStaticElementInteractions: Clicking card opens details dialog
							<div
								key={`${item.source}-${item.id}`}
								onClick={() => handleOpenDetails(item)}
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
													e.currentTarget.style.display = "none"
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
												{t("modpacks.byAuthor", { author: item.author })}
											</span>
										</div>
									</div>

									{/* Description */}
									<p className="line-clamp-2 text-muted-foreground text-xs leading-relaxed">
										{item.description || t("modpacks.noDescription")}
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
												handleOpenDetails(item)
											}}
											className="h-7 px-2 text-muted-foreground text-xs hover:text-foreground"
										>
											<Info className="size-3" />
											{t("modpacks.details")}
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
											{t("common.install")}
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
							disabled={page === 0 || isBusy}
							onClick={() => handlePageChange(Math.max(0, page - 1))}
							className="size-7 p-0"
						>
							<ChevronLeft className="size-3.5" />
						</Button>
						<span className="text-muted-foreground text-xs">
							{t("modpacks.pageOf", { page: page + 1, total: Math.max(1, totalPages) })}
						</span>
						<Button
							size="sm"
							variant="outline"
							disabled={page >= totalPages - 1 || isBusy}
							onClick={() => handlePageChange(page + 1)}
							className="size-7 p-0"
						>
							<ChevronRight className="size-3.5" />
						</Button>
					</div>
				)}
			</div>

			{/* Details Modal */}
			<ContentDetailsDialog
				open={Boolean(detailsId) && detailsItem !== null}
				onOpenChange={(open) => {
					if (!open) handleCloseDetails()
				}}
				item={detailsItem}
				onInstall={(item, ver) => {
					handleCloseDetails()
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
	validateSearch: modpacksSearchSchema,
	loaderDeps: ({ search }) => ({ search }),
	loader: async ({ context: { queryClient }, deps: { search } }) => {
		return queryClient.ensureQueryData(modpacksQueryOptions(search))
	},
	component: MemoizedModpacksPage,
})
