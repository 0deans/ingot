import { getRouteApi } from "@tanstack/react-router"
import { Plus, RefreshCw, Search, Server, Square, Terminal, X } from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import type * as v from "valibot"
import type { ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"
import type { serverCoreSchema } from "@/routes/servers"
import { instanceService } from "@/services/instance-service"
import { serverService, useRunningServers, useServers } from "@/services/server-service"
import DeleteServerDialog from "./delete-server-dialog"
import NewServerDialog from "./new-server-dialog"
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
	{ id: "folia", label: "Folia" },
	{ id: "vanilla", label: "Vanilla" },
]

export default function ServerListView() {
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
			alert(`Failed to start server: ${err}`)
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
			alert(`Failed to delete server: ${err}`)
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

	return (
		<ScrollArea className="size-full flex-1" scrollFade>
			<div className="flex flex-1 flex-col gap-6 p-4 pb-12 sm:p-5 lg:p-6">
				{/* Top Header */}
				<div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
					<div>
						<div className="flex items-center gap-2.5">
							<div className="flex size-9 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-400">
								<Server className="size-5" />
							</div>
							<h1 className="font-bold text-2xl text-foreground tracking-tight">
								Dedicated Servers
							</h1>
						</div>
						<p className="mt-1 text-muted-foreground text-xs">
							Create, configure, and manage local Minecraft servers with Paper, Purpur, Fabric,
							Folia, and Vanilla cores.
						</p>
					</div>

					<div className="flex items-center gap-2">
						<Button
							variant="outline"
							size="sm"
							onClick={() => refresh()}
							disabled={isLoading}
							className="h-9 gap-1.5 text-xs"
						>
							<RefreshCw className={cn("size-3.5", isLoading && "animate-spin text-emerald-400")} />
							<span>Refresh</span>
						</Button>

						<Button
							size="sm"
							onClick={() =>
								navigate({
									search: (prev) => ({ ...prev, action: "new" }),
								})
							}
							className="h-9 gap-1.5 bg-emerald-600 font-semibold text-white text-xs shadow-emerald-950/20 shadow-md hover:bg-emerald-500"
						>
							<Plus className="size-4" />
							<span>Create Server</span>
						</Button>
						<ImportServerButton
							className="h-9 gap-1.5 text-xs"
							onImported={(created) => openPage(created.id)}
						/>
					</div>
				</div>

				{/* Active Running Servers Multi-Banner */}
				{runningList.length > 0 && (
					<div className="flex flex-wrap items-center gap-3 rounded-2xl border border-emerald-500/30 bg-emerald-950/20 px-4 py-3 shadow-emerald-950/10 shadow-lg backdrop-blur-md">
						<div className="flex items-center gap-2 font-medium text-emerald-400 text-xs">
							<span className="size-2 animate-ping rounded-full bg-emerald-400" />
							Active Servers ({runningList.length}):
						</div>

						<div className="flex flex-wrap items-center gap-2">
							{runningList.map((proc) => {
								const srv = servers.find((s) => s.id === proc.serverId)
								return (
									<div
										key={proc.serverId}
										className="flex items-center gap-2 rounded-lg border border-emerald-500/40 bg-emerald-900/30 px-3 py-1 font-medium text-emerald-200 text-xs shadow-sm"
									>
										<span>
											{srv?.name || "Server"} (Port {proc.port})
										</span>
										<button
											type="button"
											onClick={() => srv && openPage(srv.id, "console")}
											className="rounded p-0.5 text-emerald-300 transition-colors hover:text-white"
											title="Open Console"
										>
											<Terminal className="size-3" />
										</button>
										<button
											type="button"
											onClick={() => handleStop(proc.serverId)}
											className="rounded p-0.5 text-emerald-400 transition-colors hover:bg-destructive/20 hover:text-destructive"
											title="Stop server"
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
							placeholder="Search servers by name, version, port..."
							value={searchInput}
							onChange={(e) => setSearchInput(e.target.value)}
							className="h-8.5 rounded-lg border-border/60 bg-zinc-900/50 pr-8 pl-8.5 text-foreground text-xs placeholder:text-muted-foreground focus-visible:ring-emerald-500/50"
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

					<div className="flex items-center gap-1 rounded-lg border border-border/40 bg-zinc-900/50 p-0.5">
						{CORE_FILTERS.map((c) => (
							<button
								key={c.id}
								type="button"
								onClick={() =>
									navigate({
										search: (prev) => ({ ...prev, core: c.id }),
									})
								}
								className={cn(
									"rounded-md px-2.5 py-1 font-medium text-xs transition-colors",
									coreFilter === c.id
										? "bg-zinc-800 text-foreground shadow-xs"
										: "text-muted-foreground hover:text-foreground",
								)}
							>
								{c.label}
							</button>
						))}
					</div>
				</div>

				{/* Servers Grid or Empty State */}
				{servers.length === 0 ? (
					<div className="flex flex-1 flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-zinc-900/10 p-12 text-center">
						<div className="flex size-14 items-center justify-center rounded-2xl border border-emerald-500/20 bg-emerald-500/5 text-emerald-400 shadow-inner">
							<Server className="size-7" />
						</div>
						<h3 className="mt-4 font-semibold text-base text-foreground">No Servers Created Yet</h3>
						<p className="mt-1.5 max-w-md text-muted-foreground text-xs leading-relaxed">
							Create your first local Minecraft server. Ingot will automatically download the server
							JAR, accept the EULA, configure the port, and set up your Java runtime.
						</p>
						<Button
							onClick={() =>
								navigate({
									search: (prev) => ({ ...prev, action: "new" }),
								})
							}
							className="mt-5 gap-2 bg-emerald-600 text-white text-xs hover:bg-emerald-500"
							size="sm"
						>
							<Plus className="size-4" />
							Create Your First Server
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
							className="group flex min-h-[180px] flex-col items-center justify-center gap-3 rounded-2xl border border-border/50 border-dashed bg-zinc-950/30 p-6 text-muted-foreground transition-all duration-200 hover:border-emerald-500/60 hover:bg-zinc-900/40 hover:text-white"
						>
							<div className="flex size-11 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 shadow-inner transition-transform group-hover:scale-110">
								<Plus className="size-5 text-zinc-400 group-hover:text-emerald-400" />
							</div>
							<div className="flex flex-col items-center gap-0.5 text-center">
								<span className="font-semibold text-xs text-zinc-300 group-hover:text-white">
									Create New Server
								</span>
								<span className="text-[11px] text-zinc-500">
									Paper, Purpur, Fabric, Folia, or Vanilla
								</span>
							</div>
						</button>
					</div>
				) : (
					<div className="flex flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-zinc-900/10 p-12 text-center">
						<p className="font-medium text-foreground text-sm">No servers match your filters</p>
						<p className="mt-1 text-muted-foreground text-xs">
							Try changing your search query or selecting "All Cores".
						</p>
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
							Clear Filters
						</Button>
					</div>
				)}

				{/* Dialogs */}
				<NewServerDialog
					open={action === "new"}
					onOpenChange={(open) =>
						navigate({
							search: (prev) => ({ ...prev, action: open ? "new" : undefined }),
						})
					}
					onServerCreated={() => refresh()}
				/>

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
							alert(`Failed to quick play server: ${e}`)
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
