import { queryOptions, useQuery } from "@tanstack/react-query"
import { createFileRoute } from "@tanstack/react-router"
import {
	AlertCircle,
	Box,
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
import { toast } from "sonner"
import * as v from "valibot"
import { ScrollArea } from "@/components/common/scroll-area"
import { ContentDetailsDialog } from "@/components/content/content-details-dialog"
import { InstallDialog } from "@/components/content/install-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
} from "@/components/ui/empty"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupInput,
} from "@/components/ui/input-group"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
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
		toast.success(message)
	}

	const totalPages = Math.ceil(totalHits / PAGE_SIZE)
	const isBusy = isLoading || isFetching

	return (
		<ScrollArea className="size-full flex-1" scrollFade>
			<div className="flex flex-col gap-6 p-4 pb-12 sm:p-5 lg:p-6">
				{/* Notification Banner */}

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
						<InputGroup className="min-w-[280px] flex-1">
							<InputGroupAddon>
								<Search />
							</InputGroupAddon>
							<InputGroupInput
								type="text"
								value={searchInput}
								onChange={(e) => setSearchInput(e.target.value)}
								placeholder={t("modpacks.searchPlaceholder")}
								className="text-sm"
							/>
							{searchInput && (
								<InputGroupAddon align="inline-end">
									<InputGroupButton
										size="icon-xs"
										aria-label={t("common.clearSearch")}
										onClick={() => {
											setSearchInput("")
											navigate({
												search: (prev) => ({ ...prev, query: "", page: 0 }),
												replace: true,
											})
										}}
									>
										<X />
									</InputGroupButton>
								</InputGroupAddon>
							)}
						</InputGroup>

						{/* Source Selector */}
						<ToggleGroup
							variant="outline"
							spacing={0}
							value={[activeSource]}
							onValueChange={(value) => {
								const source = SOURCES.find((s) => s.id === value[0])?.id
								if (source) handleSourceChange(source)
							}}
						>
							{SOURCES.map((s) => (
								<ToggleGroupItem key={s.id} value={s.id}>
									{s.id === "curseforge" && <Flame className="text-orange-400" />}
									{t(`modpacks.sources.${s.id}`)}
								</ToggleGroupItem>
							))}
						</ToggleGroup>
					</div>

					{/* Category Tabs */}
					<Tabs
						value={activeCategory}
						onValueChange={(value) => {
							const category = CATEGORIES.find((c) => c.id === value)?.id
							if (category !== undefined) handleCategoryChange(category)
						}}
					>
						<TabsList
							variant="line"
							className="max-w-full overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
						>
							{CATEGORIES.map(({ id, icon: Icon }) => (
								<TabsTrigger key={id} value={id}>
									<Icon />
									{t(`modpacks.categories.${id}`)}
								</TabsTrigger>
							))}
						</TabsList>
					</Tabs>

					{/* Filter Dropdowns / Options */}
					<div className="flex flex-wrap items-center gap-3">
						{/* Loader Selector */}
						<div className="flex items-center gap-1.5">
							<span className="text-muted-foreground text-xs">{t("modpacks.filters.loader")}</span>
							<Select
								items={LOADERS.map((ldr) => ({
									value: ldr.id || "all",
									label: ldr.id ? ldr.label : t("modpacks.filters.allLoaders"),
								}))}
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
								items={POPULAR_VERSIONS.map((v) => ({
									value: v.id || "all",
									label: v.id ? v.label : t("modpacks.filters.allVersions"),
								}))}
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
								items={SORTS.map((s) => ({ value: s.id, label: t(`modpacks.sorts.${s.id}`) }))}
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
								className="h-8 gap-1 px-2.5 text-muted-foreground text-xs"
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
							className="ml-auto h-8 gap-1 text-muted-foreground text-xs"
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
								className="flex flex-col gap-3 rounded-xl border border-border/40 p-4"
							>
								<div className="flex items-center gap-3">
									<Skeleton className="size-12 rounded-lg" />
									<div className="flex flex-1 flex-col gap-2">
										<Skeleton className="h-4 w-3/4" />
										<Skeleton className="h-3 w-1/2" />
									</div>
								</div>
								<Skeleton className="h-10" />
								<div className="mt-2 flex justify-between">
									<Skeleton className="h-4 w-16" />
									<Skeleton className="h-4 w-16" />
								</div>
							</div>
						))}
					</div>
				) : error ? (
					<Empty className="border">
						<EmptyHeader>
							<EmptyMedia variant="icon" className="text-destructive">
								<AlertCircle />
							</EmptyMedia>
							<EmptyDescription className="text-destructive">
								{error instanceof Error ? error.message : t("modpacks.loadFailed")}
							</EmptyDescription>
						</EmptyHeader>
						<EmptyContent>
							<Button size="sm" variant="outline" onClick={() => refetch()}>
								{t("common.tryAgain")}
							</Button>
						</EmptyContent>
					</Empty>
				) : items.length === 0 ? (
					<div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-border/40 bg-card/20 py-16 text-center">
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
								className="group flex cursor-pointer flex-col justify-between rounded-xl border border-border/40 bg-card/40 p-4 backdrop-blur-xs transition-all hover:border-primary/40 hover:bg-card/70"
							>
								<div className="flex flex-col gap-3">
									{/* Top row: Icon + Info */}
									<div className="flex items-start gap-3">
										{item.iconUrl ? (
											<img
												src={item.iconUrl}
												alt={item.title}
												className="size-12 shrink-0 rounded-lg bg-muted object-cover"
												loading="lazy"
												onError={(e) => {
													e.currentTarget.style.display = "none"
												}}
											/>
										) : (
											<div className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-muted font-bold text-muted-foreground text-sm">
												{item.title.charAt(0).toUpperCase()}
											</div>
										)}

										<div className="flex min-w-0 flex-1 flex-col">
											<div className="flex items-center gap-1.5">
												{/* Source Badge */}
												<Badge
													variant="outline"
													className={
														item.source === "modrinth"
															? "border-primary/30 text-primary"
															: "border-warning/30 text-warning"
													}
												>
													{item.source === "curseforge" && <Flame />}
													{item.source === "modrinth" ? "Modrinth" : "CurseForge"}
												</Badge>

												{/* Type Badge */}
												<Badge variant="secondary" className="capitalize">
													{item.projectType}
												</Badge>
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
											<Badge key={ldr} variant="secondary" className="font-mono capitalize">
												{ldr}
											</Badge>
										))}
										{item.latestVersion && (
											<Badge variant="outline" className="font-mono">
												{item.latestVersion}
											</Badge>
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
											className="h-7 px-2 text-muted-foreground text-xs"
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
