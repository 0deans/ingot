import { useQuery, useQueryClient } from "@tanstack/react-query"
import {
	Boxes,
	Check,
	ChevronDown,
	Download,
	ExternalLink,
	FolderOpen,
	Package,
	Paintbrush,
	Search,
	Trash2,
} from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { BedrockPackInfo, ServerConfig, UnifiedContentItem } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button, buttonVariants } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { formatCount } from "@/lib/minecraft"
import { rpc } from "@/lib/rpc"
import { contentService } from "@/services/content-service"
import { Card, EmptyState, Segmented } from "../shared/primitives"

export function BedrockPacksPanel({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const queryClient = useQueryClient()
	const [view, setView] = useState<"installed" | "browse">("installed")

	// Search state
	const [input, setInput] = useState("")
	const [query, setQuery] = useState("")
	const [category, setCategory] = useState<"all" | "addon" | "resourcepack">("all")
	const [sort, setSort] = useState("downloads")
	const [page, setPage] = useState(0)
	const [installingId, setInstallingId] = useState<string | null>(null)

	useEffect(() => {
		const timer = setTimeout(() => {
			setQuery(input.trim())
			setPage(0)
		}, 350)
		return () => clearTimeout(timer)
	}, [input])

	// Installed packs query
	const installedQuery = useQuery({
		queryKey: ["bedrock-packs", server.id],
		queryFn: () => rpc.list_bedrock_packs(server.id),
	})

	const installed = installedQuery.data ?? []
	const userPacks = useMemo(() => installed.filter((p) => !p.system), [installed])

	// Public packs search query (CurseForge Bedrock gameId 78022)
	const searchQuery = useQuery({
		queryKey: ["bedrock-search", query, category, sort, page],
		queryFn: () =>
			rpc.search_content(
				"curseforge",
				category === "all" ? "bedrock" : category,
				query || null,
				null,
				"bedrock",
				sort,
				page,
				20,
			),
		enabled: view === "browse",
	})

	const searchItems = searchQuery.data?.items ?? []
	const totalHits = searchQuery.data?.totalHits ?? 0

	const openFolder = async (subfolder?: string) => {
		try {
			if (subfolder) {
				await rpc.open_bedrock_server_subfolder(server.id, subfolder)
			} else {
				await rpc.open_server_folder(server.id)
			}
		} catch (e) {
			toast.error(String(e))
		}
	}

	const togglePack = async (pack: BedrockPackInfo) => {
		try {
			await rpc.set_bedrock_pack_enabled(server.id, pack.id, pack.packType, !pack.enabled)
			await queryClient.invalidateQueries({ queryKey: ["bedrock-packs", server.id] })
			toast.success(pack.enabled ? t("plugins.inactiveInWorld") : t("plugins.activeInWorld"))
		} catch (e) {
			toast.error(String(e))
		}
	}

	const deletePack = async (pack: BedrockPackInfo) => {
		try {
			await rpc.delete_bedrock_pack(server.id, pack.folderName, pack.packType)
			await queryClient.invalidateQueries({ queryKey: ["bedrock-packs", server.id] })
			toast.success(t("plugins.removed", { name: pack.name }))
		} catch (e) {
			toast.error(String(e))
		}
	}

	const installPack = async (item: UnifiedContentItem) => {
		setInstallingId(item.id)
		try {
			const details = await contentService.getContentDetails("curseforge", item.id)
			const version = details.versions?.[0]
			if (version?.downloadUrl && version?.filename) {
				await rpc.install_bedrock_pack(server.id, version.downloadUrl, version.filename)
				await queryClient.invalidateQueries({ queryKey: ["bedrock-packs", server.id] })
				toast.success(t("plugins.packInstalled", { name: item.title }))
			} else if (item.websiteUrl || details.websiteUrl) {
				const url = details.websiteUrl || item.websiteUrl
				if (url) window.open(url, "_blank")
				toast.info(t("plugins.downloadFromCurseForgeHint", { name: item.title }))
			} else {
				toast.error("Download link not available for this pack")
			}
		} catch (e) {
			toast.error(t("plugins.packInstallFailed", { name: item.title, error: String(e) }))
		} finally {
			setInstallingId(null)
		}
	}

	const isPackInstalled = (item: UnifiedContentItem) => {
		return userPacks.some(
			(p) =>
				p.name.toLowerCase() === item.title.toLowerCase() ||
				p.folderName.toLowerCase().includes(item.slug.toLowerCase()),
		)
	}

	return (
		<div className="flex flex-col gap-4">
			{/* Top Bar: Tabs + Folder Dropdown */}
			<div className="flex items-center justify-between gap-3">
				<Segmented
					value={view}
					onChange={setView}
					options={[
						{
							value: "installed",
							label:
								userPacks.length > 0
									? `${t("common.installed")} · ${userPacks.length}`
									: t("common.installed"),
						},
						{
							value: "browse",
							label: t("plugins.browsePacks"),
						},
					]}
					className="w-auto shrink-0"
				/>

				<DropdownMenu>
					<DropdownMenuTrigger
						render={
							<Button variant="outline" size="sm" className="h-8 gap-1.5 rounded-xl text-xs">
								<FolderOpen className="size-3.5 text-muted-foreground" />
								<span>{t("common.openFolder")}</span>
								<ChevronDown className="size-3 text-muted-foreground" />
							</Button>
						}
					/>
					<DropdownMenuContent align="end" className="w-56">
						<DropdownMenuGroup>
							<DropdownMenuLabel className="text-2xs text-muted-foreground uppercase">
								{t("instances.openFolder", { defaultValue: "Directories" })}
							</DropdownMenuLabel>
							<DropdownMenuItem onClick={() => openFolder("behavior_packs")}>
								<FolderOpen className="mr-2 size-4 text-muted-foreground" />
								<div className="flex flex-col">
									<span className="font-medium text-xs">behavior_packs</span>
									<span className="text-3xs text-muted-foreground">
										{t("plugins.behaviorPacks")}
									</span>
								</div>
							</DropdownMenuItem>
							<DropdownMenuItem onClick={() => openFolder("resource_packs")}>
								<FolderOpen className="mr-2 size-4 text-muted-foreground" />
								<div className="flex flex-col">
									<span className="font-medium text-xs">resource_packs</span>
									<span className="text-3xs text-muted-foreground">
										{t("plugins.resourcePacks")}
									</span>
								</div>
							</DropdownMenuItem>
						</DropdownMenuGroup>
						<DropdownMenuSeparator />
						<DropdownMenuItem onClick={() => openFolder()}>
							<ExternalLink className="mr-2 size-4 text-muted-foreground" />
							<span className="text-xs">{t("plugins.openServerFolder")}</span>
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			</div>

			{/* Installed View */}
			{view === "installed" && (
				<div className="flex flex-col gap-3">
					{installedQuery.isLoading ? (
						<div className="flex justify-center py-12">
							<Spinner className="size-6 text-muted-foreground" />
						</div>
					) : userPacks.length === 0 ? (
						<Card>
							<EmptyState
								icon={Package}
								title={t("plugins.noInstalledPacks")}
								description={t("plugins.noInstalledPacksDesc")}
								action={
									<Button onClick={() => setView("browse")} className="h-9 gap-1.5 rounded-xl">
										<Search className="size-4" />
										{t("plugins.browsePacks")}
									</Button>
								}
							/>
						</Card>
					) : (
						<Card className="gap-0 divide-y divide-border/70 py-0">
							{userPacks.map((pack) => (
								<div
									key={`${pack.packType}-${pack.id}`}
									className="flex items-center justify-between gap-3 px-3.5 py-3"
								>
									<div className="flex min-w-0 items-center gap-3">
										<div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-border bg-card">
											{pack.packType === "resource" ? (
												<Paintbrush className="size-5 text-muted-foreground" />
											) : (
												<Boxes className="size-5 text-muted-foreground" />
											)}
										</div>
										<div className="min-w-0">
											<div className="flex flex-wrap items-center gap-2">
												<span className="truncate font-medium text-foreground text-sm">
													{pack.name}
												</span>
												<Badge variant="secondary" className="px-1.5 py-0 font-mono text-3xs">
													v{pack.version}
												</Badge>
												<Badge
													variant={pack.packType === "resource" ? "outline" : "default"}
													className="px-1.5 py-0 text-3xs capitalize"
												>
													{pack.packType === "resource"
														? t("plugins.resourcePacks")
														: t("plugins.behaviorPacks")}
												</Badge>
											</div>
											<p className="line-clamp-1 text-muted-foreground text-xs">
												{pack.description || pack.folderName}
											</p>
										</div>
									</div>

									<div className="flex shrink-0 items-center gap-3">
										<div className="flex items-center gap-2">
											<span className="hidden text-muted-foreground text-xs sm:inline">
												{pack.enabled ? t("plugins.activeInWorld") : t("plugins.inactiveInWorld")}
											</span>
											<Switch
												checked={pack.enabled}
												onCheckedChange={() => togglePack(pack)}
												aria-label={
													pack.enabled ? t("plugins.activeInWorld") : t("plugins.inactiveInWorld")
												}
											/>
										</div>
										<Button
											variant="ghost"
											size="icon"
											onClick={() => deletePack(pack)}
											className="size-8 text-destructive hover:bg-destructive/10 hover:text-destructive"
											aria-label="Delete pack"
										>
											<Trash2 className="size-4" />
										</Button>
									</div>
								</div>
							))}
						</Card>
					)}
				</div>
			)}

			{/* Browse Public Packs View */}
			{view === "browse" && (
				<div className="flex flex-col gap-3">
					{/* Search & Filter Bar */}
					<div className="flex flex-col gap-2 sm:flex-row sm:items-center">
						<InputGroup className="flex-1">
							<InputGroupAddon>
								<Search className="size-4" />
							</InputGroupAddon>
							<InputGroupInput
								value={input}
								onChange={(e) => setInput(e.target.value)}
								placeholder={t("plugins.searchPacksPlaceholder")}
								className="text-sm"
							/>
						</InputGroup>

						<div className="flex items-center gap-2">
							<Select
								value={category}
								onValueChange={(v) => {
									if (v) {
										setCategory(v as typeof category)
										setPage(0)
									}
								}}
							>
								<SelectTrigger className="h-10 w-44 rounded-xl text-xs">
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="all">{t("plugins.allPacks")}</SelectItem>
									<SelectItem value="addon">{t("plugins.behaviorPacks")}</SelectItem>
									<SelectItem value="resourcepack">{t("plugins.resourcePacks")}</SelectItem>
								</SelectContent>
							</Select>

							<Select
								value={sort}
								onValueChange={(v) => {
									if (v) {
										setSort(v)
										setPage(0)
									}
								}}
							>
								<SelectTrigger className="h-10 w-36 rounded-xl text-xs">
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="downloads">{t("plugins.sort.downloads")}</SelectItem>
									<SelectItem value="relevance">{t("plugins.sort.relevance")}</SelectItem>
									<SelectItem value="updated">{t("plugins.sort.updated")}</SelectItem>
									<SelectItem value="newest">{t("plugins.sort.newest")}</SelectItem>
								</SelectContent>
							</Select>
						</div>
					</div>

					{/* Search Results */}
					{searchQuery.isLoading ? (
						<div className="flex justify-center py-16">
							<Spinner className="size-6 text-muted-foreground" />
						</div>
					) : searchItems.length === 0 ? (
						<Card>
							<EmptyState
								icon={Search}
								title={t("plugins.noPacksFound")}
								description={t("plugins.tryAnotherSearch")}
							/>
						</Card>
					) : (
						<div className="flex flex-col gap-2.5">
							<div className="flex items-center justify-between px-1 text-2xs text-muted-foreground">
								<span>
									{t("plugins.results", {
										count: totalHits,
										formatted: formatCount(totalHits),
									})}
								</span>
								<span>CurseForge Bedrock</span>
							</div>

							<div className="grid grid-cols-1 gap-2.5 md:grid-cols-2">
								{searchItems.map((item) => {
									const installed = isPackInstalled(item)
									const isInstalling = installingId === item.id

									return (
										<div
											key={item.id}
											className="flex flex-col justify-between gap-3 rounded-2xl border border-border/80 bg-card/40 p-3.5 transition-colors hover:border-input"
										>
											<div className="flex items-start gap-3">
												{/* Pack Icon */}
												<div className="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-border bg-card">
													{item.iconUrl ? (
														<img
															src={item.iconUrl}
															alt=""
															className="size-full object-cover"
															loading="lazy"
														/>
													) : (
														<Package className="size-6 text-muted-foreground/60" />
													)}
												</div>

												{/* Pack Information */}
												<div className="min-w-0 flex-1">
													<div className="flex items-center justify-between gap-2">
														<h4 className="truncate font-semibold text-foreground text-sm">
															{item.title}
														</h4>
													</div>
													<p className="text-2xs text-muted-foreground">
														{item.author} • {formatCount(item.downloads)} downloads
													</p>
													<p className="mt-1 line-clamp-2 text-muted-foreground text-xs">
														{item.description}
													</p>
												</div>
											</div>

											{/* Bottom Action Footer */}
											<div className="flex items-center justify-between border-border/40 border-t pt-2.5">
												<div className="flex flex-wrap items-center gap-1">
													{item.categories.slice(0, 2).map((cat) => (
														<Badge
															key={cat}
															variant="secondary"
															className="px-1.5 py-0 font-normal text-3xs"
														>
															{cat}
														</Badge>
													))}
												</div>

												<div className="flex items-center gap-1.5">
													{item.websiteUrl && (
														<a
															href={item.websiteUrl}
															target="_blank"
															rel="noopener noreferrer"
															aria-label="View on CurseForge"
															className={buttonVariants({
																variant: "ghost",
																size: "icon",
																className: "size-8 text-muted-foreground",
															})}
														>
															<ExternalLink className="size-3.5" />
														</a>
													)}

													{installed ? (
														<Badge
															variant="outline"
															className="gap-1 px-2 py-1 text-primary text-xs"
														>
															<Check className="size-3" />
															{t("plugins.installedPacks")}
														</Badge>
													) : (
														<Button
															size="sm"
															variant="default"
															disabled={isInstalling}
															onClick={() => installPack(item)}
															className="h-8 gap-1.5 text-xs"
														>
															{isInstalling ? (
																<Spinner className="size-3.5" />
															) : (
																<Download className="size-3.5" />
															)}
															{t("plugins.installPack")}
														</Button>
													)}
												</div>
											</div>
										</div>
									)
								})}
							</div>

							{/* Simple Pagination */}
							<div className="flex items-center justify-center gap-2 pt-4">
								<Button
									variant="outline"
									size="sm"
									disabled={page === 0}
									onClick={() => setPage((p) => Math.max(0, p - 1))}
								>
									{t("common.previous")}
								</Button>
								<span className="font-mono text-muted-foreground text-xs">Page {page + 1}</span>
								<Button
									variant="outline"
									size="sm"
									disabled={searchItems.length < 20}
									onClick={() => setPage((p) => p + 1)}
								>
									{t("common.next")}
								</Button>
							</div>
						</div>
					)}
				</div>
			)}
		</div>
	)
}
