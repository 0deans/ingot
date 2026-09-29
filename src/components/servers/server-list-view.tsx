import { getRouteApi } from "@tanstack/react-router"
import { Plus, RefreshCw, Search, Server, Square, Terminal, X } from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type * as v from "valibot"
import type { ServerConfig } from "@/bindings"
import { ScrollArea } from "@/components/common/scroll-area"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { cn } from "@/lib/utils"
import type { serverCoreSchema } from "@/routes/servers"
import { instanceService } from "@/services/instance-service"
import { serverService, useRunningServers, useServers } from "@/services/server-service"
import DeleteServerDialog from "./delete-server-dialog"
import { NewServerWizard } from "./new-server-wizard"
import { ImportServerButton } from "./panels/transfer-card"
import { QuickJoinDialog } from "./quick-join-dialog"
import ServerCard from "./server-card"
import { ServerWorkspace } from "./server-workspace"

const routeApi = getRouteApi("/servers")

const CORE_FILTERS: { id: v.InferOutput<typeof serverCoreSchema>; label: string }[] = [
	{ id: "all", label: "All Cores" },
	{ id: "paper", label: "Paper" },
	{ id: "purpur", label: "Purpur" },
	{ id: "fabric", label: "Fabric" },
	{ id: "neoforge", label: "NeoForge" },
	{ id: "forge", label: "Forge" },
	{ id: "quilt", label: "Quilt" },
	{ id: "folia", label: "Folia" },
	{ id: "vanilla", label: "Vanilla" },
]

export default function ServerListView() {
	const { t } = useTranslation()
	const search = routeApi.useSearch()
	const navigate = routeApi.useNavigate()

	const searchQuery = search.q ?? ""
	const coreFilter = search.core ?? "all"
	const action = search.action
	const openServerId = search.server
	const deletingServerId = search.delete

	const [searchInput, setSearchInput] = useState(searchQuery)
	const [quickJoinServer, setQuickJoinServer] = useState<ServerConfig | null>(null)

	const { servers, isLoading, refresh } = useServers()
	const { runningMap, runningList } = useRunningServers()

	useEffect(() => {
		setSearchInput(searchQuery)
	}, [searchQuery])

	useEffect(() => {
		const timer = setTimeout(() => {
			if (searchInput !== searchQuery) {
				navigate({
					search: (prev) => ({ ...prev, q: searchInput }),
					replace: true,
				})
			}
		}, 300)
		return () => clearTimeout(timer)
	}, [searchInput, searchQuery, navigate])

	const openServer = useMemo(
		() => (openServerId ? (servers.find((s) => s.id === openServerId) ?? null) : null),
		[servers, openServerId],
	)
	const openPage = (serverId: string, tab: NonNullable<typeof search.tab> = "overview") =>
		navigate({ search: (prev) => ({ ...prev, server: serverId, tab }) })
	const deletingServer = useMemo(
		() => (deletingServerId ? (servers.find((s) => s.id === deletingServerId) ?? null) : null),
		[servers, deletingServerId],
	)

	const handleStart = async (serverId: string) => {
		try {
			await serverService.startServer(serverId)
		} catch (err) {
			console.error("Failed to start server:", err)
			alert(t("errors.startServer", { error: String(err) }))
		}
	}

	const handleStop = async (serverId: string) => {
		try {
			await serverService.stopServer(serverId)
		} catch (err) {
			console.error("Failed to stop server:", err)
		}
	}

	const handleOpenFolder = async (serverId: string) => {
		try {
			await serverService.openServerFolder(serverId)
		} catch (err) {
			console.error("Failed to open server folder:", err)
		}
	}

	const handleDeleteConfirm = async (serverId: string, deleteFiles: boolean) => {
		try {
			await serverService.deleteServer(serverId, deleteFiles)
			navigate({
				search: (prev) => ({
					...prev,
					delete: undefined,
					server: prev.server === serverId ? undefined : prev.server,
				}),
			})
		} catch (err) {
			console.error("Failed to delete server:", err)
			alert(t("errors.deleteServer", { error: String(err) }))
		}
	}

	const filteredServers = useMemo(() => {
		let list = servers
		if (coreFilter !== "all") {
			list = list.filter((s) => s.core === coreFilter)
		}
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase()
			list = list.filter(
				(s) =>
					s.name.toLowerCase().includes(q) ||
					s.gameVersion.toLowerCase().includes(q) ||
					s.port.toString().includes(q),
			)
		}
		return list
	}, [servers, coreFilter, searchQuery])

	if (openServer) {
		return (
			<ServerWorkspace
				server={openServer}
				tab={search.tab ?? "overview"}
				onTabChange={(tab) => openPage(openServer.id, tab)}
				onBack={() =>
					navigate({ search: (prev) => ({ ...prev, server: undefined, tab: undefined }) })
				}
			/>
		)
	}

	if (action === "new") {
		return (
			<div className="size-full overflow-hidden">
				<NewServerWizard
					onCancel={() => navigate({ search: (prev) => ({ ...prev, action: undefined }) })}
					onServerCreated={(created) => {
						navigate({ search: (prev) => ({ ...prev, action: undefined }) })
						refresh()
						openPage(created.id)
					}}
					importSlot={
						<ImportServerButton
							className="h-12 gap-2 rounded-2xl border-border"
							onImported={(created) => {
								navigate({ search: (prev) => ({ ...prev, action: undefined }) })
								refresh()
								openPage(created.id)
							}}
						/>
					}
				/>
			</div>
		)
	}

	return (
		<ScrollArea className="size-full flex-1" scrollFade>
			<div className="flex flex-1 flex-col gap-6 p-4 pb-12 sm:p-5 lg:p-6">
				{/* Top Header */}
				<div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
					<div>
						<div className="flex items-center gap-2.5">
							<div className="flex size-9 items-center justify-center rounded-xl border border-primary/30 bg-primary/10 text-primary">
								<Server className="size-5" />
							</div>
							<h1 className="font-bold text-2xl text-foreground tracking-tight">
								{t("servers.title")}
							</h1>
						</div>
						<p className="mt-1 text-muted-foreground text-xs">{t("servers.subtitle")}</p>
					</div>

					<div className="flex items-center gap-2">
						<Button
							variant="outline"
							size="sm"
							onClick={() => refresh()}
							disabled={isLoading}
							className="h-9 gap-1.5 text-xs"
						>
							<RefreshCw className={cn("size-3.5", isLoading && "animate-spin text-primary")} />
							<span>{t("common.refresh")}</span>
						</Button>

						<Button
							size="sm"
							onClick={() =>
								navigate({
									search: (prev) => ({ ...prev, action: "new" }),
								})
							}
							className="h-9 gap-1.5 font-semibold text-xs"
						>
							<Plus className="size-4" />
							<span>{t("servers.createServer")}</span>
						</Button>
						<ImportServerButton
							className="h-9 gap-1.5 text-xs"
							onImported={(created) => openPage(created.id)}
						/>
					</div>
				</div>

				{/* Active Running Servers Multi-Banner */}
				{runningList.length > 0 && (
					<div className="flex flex-wrap items-center gap-3 rounded-2xl border border-primary/30 bg-primary/10 px-4 py-3 shadow-lg shadow-primary/10 backdrop-blur-md">
						<div className="flex items-center gap-2 font-medium text-primary text-xs">
							<span className="size-2 animate-ping rounded-full bg-primary" />
							{t("servers.activeServers", { count: runningList.length })}
						</div>

						<div className="flex flex-wrap items-center gap-2">
							{runningList.map((proc) => {
								const srv = servers.find((s) => s.id === proc.serverId)
								return (
									<div
										key={proc.serverId}
										className="flex items-center gap-2 rounded-lg border border-primary/40 bg-primary/10 px-3 py-1 font-medium text-primary text-xs shadow-sm"
									>
										<span>
											{t("servers.activeEntry", { name: srv?.name ?? "", port: proc.port })}
										</span>
										<button
											type="button"
											onClick={() => srv && openPage(srv.id, "console")}
											className="rounded p-0.5 text-primary transition-colors hover:text-foreground"
											title={t("servers.openConsole")}
										>
											<Terminal className="size-3" />
										</button>
										<button
											type="button"
											onClick={() => handleStop(proc.serverId)}
											className="rounded p-0.5 text-primary transition-colors hover:bg-destructive/20 hover:text-destructive"
											title={t("servers.stopServer")}
										>
											<Square className="size-3 fill-current" />
										</button>
									</div>
								)
							})}
						</div>
					</div>
				)}

				{/* Search & Core Filter Bar */}
				<div className="flex flex-wrap items-center justify-between gap-3 border-border/30 border-b pb-3">
					<div className="relative min-w-[220px] max-w-sm flex-1">
						<Search className="absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted-foreground" />
						<Input
							type="text"
							placeholder={t("servers.searchPlaceholder")}
							value={searchInput}
							onChange={(e) => setSearchInput(e.target.value)}
							className="h-8.5 rounded-lg pr-8 pl-8.5 text-xs placeholder:text-muted-foreground"
						/>
						{searchInput && (
							<button
								type="button"
								onClick={() => {
									setSearchInput("")
									navigate({
										search: (prev) => ({ ...prev, q: "" }),
										replace: true,
									})
								}}
								className="absolute top-1/2 right-2.5 -translate-y-1/2 text-muted-foreground hover:text-foreground"
							>
								<X className="size-3.5" />
							</button>
						)}
					</div>

					<ToggleGroup
						variant="outline"
						size="sm"
						spacing={0}
						value={[coreFilter]}
						onValueChange={(value) => {
							const core = CORE_FILTERS.find((c) => c.id === value[0])?.id
							if (core) navigate({ search: (prev) => ({ ...prev, core }) })
						}}
					>
						{CORE_FILTERS.map((c) => (
							<ToggleGroupItem key={c.id} value={c.id}>
								{c.id === "all" ? t("servers.allCores") : c.label}
							</ToggleGroupItem>
						))}
					</ToggleGroup>
				</div>

				{/* Servers Grid or Empty State */}
				{servers.length === 0 ? (
					<div className="flex flex-1 flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-card/10 p-12 text-center">
						<div className="flex size-14 items-center justify-center rounded-2xl border border-primary/20 bg-primary/5 text-primary shadow-inner">
							<Server className="size-7" />
						</div>
						<h3 className="mt-4 font-semibold text-base text-foreground">
							{t("servers.emptyTitle")}
						</h3>
						<p className="mt-1.5 max-w-md text-muted-foreground text-xs leading-relaxed">
							{t("servers.emptySubtitle")}
						</p>
						<Button
							onClick={() =>
								navigate({
									search: (prev) => ({ ...prev, action: "new" }),
								})
							}
							className="mt-5 gap-2 text-xs"
							size="sm"
						>
							<Plus className="size-4" />
							{t("servers.createFirst")}
						</Button>
					</div>
				) : filteredServers.length > 0 ? (
					<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
						{filteredServers.map((srv) => (
							<ServerCard
								key={srv.id}
								server={srv}
								runningInfo={runningMap.get(srv.id)}
								onStart={handleStart}
								onStop={handleStop}
								onOpen={(s) => openPage(s.id)}
								onOpenConsole={(s) => openPage(s.id, "console")}
								onOpenSettings={(s) => openPage(s.id, "settings")}
								onOpenFolder={handleOpenFolder}
								onDelete={(s) =>
									navigate({
										search: (prev) => ({ ...prev, delete: s.id }),
									})
								}
								onJoinServer={(s) => setQuickJoinServer(s)}
							/>
						))}

						{/* Create New Server Dashed Card */}
						<button
							type="button"
							onClick={() =>
								navigate({
									search: (prev) => ({ ...prev, action: "new" }),
								})
							}
							className="group flex min-h-[180px] flex-col items-center justify-center gap-3 rounded-2xl border border-border/50 border-dashed bg-background/30 p-6 text-muted-foreground transition-all duration-200 hover:border-primary/60 hover:bg-card/40 hover:text-foreground"
						>
							<div className="flex size-11 items-center justify-center rounded-xl border border-border bg-card shadow-inner transition-transform group-hover:scale-110">
								<Plus className="size-5 text-muted-foreground group-hover:text-primary" />
							</div>
							<div className="flex flex-col items-center gap-0.5 text-center">
								<span className="font-semibold text-foreground/80 text-xs group-hover:text-foreground">
									{t("servers.createCardTitle")}
								</span>
								<span className="text-[11px] text-muted-foreground">
									{t("servers.createCardSubtitle")}
								</span>
							</div>
						</button>
					</div>
				) : (
					<div className="flex flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-card/10 p-12 text-center">
						<p className="font-medium text-foreground text-sm">{t("servers.noMatchTitle")}</p>
						<p className="mt-1 text-muted-foreground text-xs">{t("servers.noMatchSubtitle")}</p>
						<Button
							variant="outline"
							size="sm"
							onClick={() => {
								setSearchInput("")
								navigate({
									search: (prev) => ({ ...prev, q: "", core: "all" }),
								})
							}}
							className="mt-4 text-xs"
						>
							{t("common.clearFilters")}
						</Button>
					</div>
				)}

				{/* Dialogs */}
				<DeleteServerDialog
					server={deletingServer}
					open={Boolean(deletingServer)}
					onOpenChange={(open) =>
						navigate({
							search: (prev) => ({ ...prev, delete: open ? prev.delete : undefined }),
						})
					}
					onConfirm={handleDeleteConfirm}
				/>

				<QuickJoinDialog
					server={quickJoinServer}
					open={Boolean(quickJoinServer)}
					onOpenChange={(open) => !open && setQuickJoinServer(null)}
					onLaunch={async (instanceId, serverAddress) => {
						try {
							await instanceService.launchInstance(instanceId, {
								server: serverAddress,
								world: null,
							})
						} catch (e) {
							console.error("Failed to quick play server:", e)
							alert(t("errors.quickPlay", { error: String(e) }))
						}
					}}
					onCreateInstance={() => {
						navigate({
							to: "/",
							search: { action: "new" },
						})
					}}
				/>
			</div>
		</ScrollArea>
	)
}
