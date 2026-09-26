import {
	ArrowUpCircle,
	Check,
	Download,
	ExternalLink,
	Loader2,
	Package,
	Puzzle,
	RotateCw,
	Search,
	Trash2,
} from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import type {
	InstalledPlugin,
	PluginProject,
	PluginSource,
	PluginVersion,
	ServerConfig,
} from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { formatBytes, formatCount } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import { useServerControls } from "@/services/hosting"
import {
	useInstalledPlugins,
	usePluginActions,
	usePluginSearch,
	usePluginUpdates,
	useServerStatus,
} from "@/services/server-data"
import { rpc } from "@/services/server-service"
import { Card, EmptyState, ErrorNote, Segmented, useSticky } from "../shared/primitives"
import { PluginDetailsSheet, PluginIcon } from "./plugin-details-sheet"

/** What the server core can load; null = nothing */
export function addonKind(core: ServerConfig["core"]): { noun: string; nouns: string } | null {
	if (core === "fabric") return { noun: "mod", nouns: "Mods" }
	if (core === "paper" || core === "purpur" || core === "folia")
		return { noun: "plugin", nouns: "Plugins" }
	return null
}

interface Notice {
	kind: "success" | "error"
	text: string
}

export function PluginsPanel({ server }: { server: ServerConfig }) {
	const kind = addonKind(server.core)
	const { isRunning } = useServerStatus(server.id)
	const [view, setView] = useState<"installed" | "browse">("installed")
	const [notice, setNotice] = useState<Notice | null>(null)
	const [needsRestart, setNeedsRestart] = useState(false)
	const installed = useInstalledPlugins(server.id)
	const actions = usePluginActions(server.id)

	useEffect(() => {
		if (!isRunning) setNeedsRestart(false)
	}, [isRunning])

	if (!kind) {
		return (
			<Card>
				<EmptyState
					icon={Puzzle}
					title={`${server.core === "pumpkin" ? "Pumpkin" : "Vanilla"} servers can't use plugins`}
					description="Create a Paper or Purpur server to add plugins, or a Fabric server for mods."
				/>
			</Card>
		)
	}

	const changed = (text: string, error = false) => {
		setNotice({ kind: error ? "error" : "success", text })
		if (!error && isRunning) setNeedsRestart(true)
	}

	const install = async (project: PluginProject, version?: PluginVersion) => {
		try {
			const report = await actions.install.mutateAsync({
				source: project.source,
				projectId: project.id,
				versionId: version?.id,
			})
			const extras = report.installed.filter((t) => t !== project.title)
			changed(
				`Installed ${project.title}${extras.length ? ` and ${extras.join(", ")}` : ""}.${
					report.warnings.length ? ` ${report.warnings.join(". ")}.` : ""
				}`,
			)
		} catch (e) {
			changed(String(e), true)
		}
	}

	return (
		<div className="flex flex-col gap-4">
			<Segmented
				value={view}
				onChange={setView}
				options={[
					{
						value: "installed",
						label: `Installed${installed.data ? ` · ${installed.data.length}` : ""}`,
					},
					{ value: "browse", label: `Browse ${kind.nouns.toLowerCase()}` },
				]}
			/>

			{needsRestart && <RestartBanner serverId={server.id} onDone={() => setNeedsRestart(false)} />}
			{notice && (
				<button
					type="button"
					onClick={() => setNotice(null)}
					className={cn(
						"rounded-xl border px-3 py-2.5 text-left text-xs",
						notice.kind === "success"
							? "border-emerald-500/20 bg-emerald-500/10 text-emerald-200"
							: "border-rose-500/20 bg-rose-500/10 text-rose-300",
					)}
				>
					{notice.text}
				</button>
			)}

			{view === "installed" ? (
				<InstalledList
					server={server}
					kind={kind}
					plugins={installed.data ?? []}
					loading={installed.isLoading}
					onBrowse={() => setView("browse")}
					onChanged={changed}
				/>
			) : (
				<BrowseView
					server={server}
					kind={kind}
					installed={installed.data ?? []}
					installingId={
						actions.install.isPending ? (actions.install.variables?.projectId ?? null) : null
					}
					onInstall={install}
				/>
			)}
		</div>
	)
}

function RestartBanner({ serverId, onDone }: { serverId: string; onDone: () => void }) {
	const controls = useServerControls(serverId)
	const [restarting, setRestarting] = useState(false)
	const restart = async () => {
		setRestarting(true)
		await controls.stop()
		// Wait for the process to exit before starting again
		for (let i = 0; i < 60; i++) {
			const running = await rpc.get_running_servers()
			if (!running.some((s) => s.serverId === serverId)) break
			await new Promise((r) => setTimeout(r, 500))
		}
		await controls.start()
		setRestarting(false)
		onDone()
	}
	return (
		<div className="flex items-center justify-between gap-3 rounded-xl border border-amber-500/20 bg-amber-500/10 py-2 pr-2 pl-3">
			<span className="text-amber-200 text-xs">Restart the server to apply changes.</span>
			<Button
				size="sm"
				onClick={restart}
				disabled={restarting}
				className="h-8 shrink-0 gap-1.5 rounded-lg bg-amber-500/20 text-amber-100 hover:bg-amber-500/30"
			>
				{restarting ? (
					<Loader2 className="size-3.5 animate-spin" />
				) : (
					<RotateCw className="size-3.5" />
				)}
				Restart
			</Button>
		</div>
	)
}

// ─── Installed ────────────────────────────────────────────────────────────────

function InstalledList({
	server,
	kind,
	plugins,
	loading,
	onBrowse,
	onChanged,
}: {
	server: ServerConfig
	kind: { noun: string; nouns: string }
	plugins: InstalledPlugin[]
	loading: boolean
	onBrowse: () => void
	onChanged: (text: string, error?: boolean) => void
}) {
	const actions = usePluginActions(server.id)
	const hasTracked = plugins.some((p) => p.projectId)
	const updates = usePluginUpdates(server.id, hasTracked)
	const [selected, setSelected] = useState<string | null>(null)
	const current = useSticky(plugins.find((p) => p.fileName === selected))
	const updateFor = (p: InstalledPlugin) =>
		updates.data?.find((u) => u.fileName === p.fileName.replace(/\.disabled$/, ""))

	if (loading) {
		return (
			<div className="flex justify-center py-10">
				<Loader2 className="size-5 animate-spin text-zinc-500" />
			</div>
		)
	}
	if (plugins.length === 0) {
		return (
			<Card>
				<EmptyState
					icon={Package}
					title={`No ${kind.nouns.toLowerCase()} yet`}
					description={`Find ${kind.nouns.toLowerCase()} on Modrinth${server.core !== "fabric" && server.core !== "folia" ? " and Hangar" : ""} and install them in one tap.`}
					action={
						<Button
							onClick={onBrowse}
							className="h-10 gap-2 rounded-xl bg-emerald-600 text-white hover:bg-emerald-500"
						>
							<Search className="size-4" /> Browse {kind.nouns.toLowerCase()}
						</Button>
					}
				/>
			</Card>
		)
	}

	return (
		<>
			<Card className="divide-y divide-zinc-800/70 overflow-hidden">
				{plugins.map((p) => {
					const update = updateFor(p)
					return (
						<div key={p.fileName} className="flex items-center gap-3 px-3.5 py-3">
							<button
								type="button"
								onClick={() => setSelected(p.fileName)}
								className="flex min-w-0 flex-1 items-center gap-3 text-left"
							>
								<PluginIcon
									url={p.iconUrl}
									name={p.name}
									className={cn("size-10", !p.enabled && "opacity-40")}
								/>
								<div className="min-w-0 flex-1">
									<p
										className={cn(
											"truncate font-medium text-sm",
											p.enabled ? "text-zinc-100" : "text-zinc-500",
										)}
									>
										{p.name}
										{p.version && (
											<span className="ml-1.5 font-normal text-xs text-zinc-500">{p.version}</span>
										)}
									</p>
									<p className="truncate text-[11px] text-zinc-500">
										{update ? (
											<span className="text-sky-300">
												Update available: {update.latest.versionNumber}
											</span>
										) : (
											(p.description ?? p.authors.join(", ")) || p.fileName
										)}
									</p>
								</div>
							</button>
							<Switch
								checked={p.enabled}
								disabled={actions.toggle.isPending}
								onCheckedChange={async (enabled) => {
									try {
										await actions.toggle.mutateAsync({ fileName: p.fileName, enabled })
										onChanged(`${enabled ? "Enabled" : "Disabled"} ${p.name}.`)
									} catch (e) {
										onChanged(String(e), true)
									}
								}}
								aria-label={`${p.enabled ? "Disable" : "Enable"} ${p.name}`}
							/>
						</div>
					)
				})}
			</Card>

			<Dialog open={selected !== null} onOpenChange={(open) => !open && setSelected(null)}>
				<DialogContent className="gap-4 p-5 sm:max-w-md">
					{current && (
						<InstalledDetails
							plugin={current}
							update={updateFor(current)?.latest ?? null}
							busy={actions.install.isPending || actions.remove.isPending}
							onUpdate={async () => {
								if (!current.source || !current.projectId) return
								try {
									await actions.install.mutateAsync({
										source: current.source,
										projectId: current.projectId,
									})
									onChanged(`Updated ${current.name}.`)
									setSelected(null)
								} catch (e) {
									onChanged(String(e), true)
								}
							}}
							onRemove={async () => {
								try {
									await actions.remove.mutateAsync(current.fileName)
									onChanged(`Removed ${current.name}.`)
									setSelected(null)
								} catch (e) {
									onChanged(String(e), true)
								}
							}}
						/>
					)}
				</DialogContent>
			</Dialog>
		</>
	)
}

function InstalledDetails({
	plugin,
	update,
	busy,
	onUpdate,
	onRemove,
}: {
	plugin: InstalledPlugin
	update: PluginVersion | null
	busy: boolean
	onUpdate: () => void
	onRemove: () => void
}) {
	const [confirm, setConfirm] = useState(false)
	const pageUrl =
		plugin.source === "modrinth" && plugin.projectId
			? `https://modrinth.com/project/${plugin.projectId}`
			: plugin.source === "hangar" && plugin.projectId
				? `https://hangar.papermc.io/search?query=${encodeURIComponent(plugin.projectId)}`
				: null
	return (
		<>
			<div className="flex items-start gap-3 pr-8">
				<PluginIcon url={plugin.iconUrl} name={plugin.name} className="size-12" />
				<div className="min-w-0">
					<DialogTitle className="truncate text-base">{plugin.name}</DialogTitle>
					<p className="text-xs text-zinc-500">
						{plugin.version ?? "Unknown version"}
						{plugin.authors.length > 0 && ` · by ${plugin.authors.join(", ")}`}
					</p>
				</div>
			</div>
			{plugin.description && (
				<p className="text-sm text-zinc-300 leading-relaxed">{plugin.description}</p>
			)}
			<dl className="grid grid-cols-2 gap-2 text-xs">
				<div className="rounded-xl bg-zinc-900/60 p-2.5">
					<dt className="text-zinc-500">File</dt>
					<dd className="truncate font-mono text-zinc-300">{plugin.fileName}</dd>
				</div>
				<div className="rounded-xl bg-zinc-900/60 p-2.5">
					<dt className="text-zinc-500">Size</dt>
					<dd className="text-zinc-300">{formatBytes(plugin.size)}</dd>
				</div>
			</dl>

			{update && (
				<Button
					onClick={onUpdate}
					disabled={busy}
					className="h-11 gap-2 rounded-xl bg-sky-600 text-white hover:bg-sky-500"
				>
					{busy ? (
						<Loader2 className="size-4 animate-spin" />
					) : (
						<ArrowUpCircle className="size-4" />
					)}
					Update to {update.versionNumber}
				</Button>
			)}
			<div className="flex gap-2">
				{pageUrl && (
					<a
						href={pageUrl}
						target="_blank"
						rel="noopener noreferrer"
						className="flex h-10 flex-1 items-center justify-center gap-1.5 rounded-xl border border-zinc-800 text-sm text-zinc-300 hover:bg-zinc-900"
					>
						<ExternalLink className="size-4" /> Page
					</a>
				)}
				{confirm ? (
					<Button
						variant="destructive"
						onClick={onRemove}
						disabled={busy}
						className="h-10 flex-1 gap-1.5 rounded-xl"
					>
						{busy ? <Loader2 className="size-4 animate-spin" /> : <Trash2 className="size-4" />}
						Really delete?
					</Button>
				) : (
					<Button
						variant="destructive"
						onClick={() => setConfirm(true)}
						className="h-10 flex-1 gap-1.5 rounded-xl"
					>
						<Trash2 className="size-4" /> Delete
					</Button>
				)}
			</div>
		</>
	)
}

// ─── Browse ───────────────────────────────────────────────────────────────────

const SORT_OPTIONS = [
	{ value: "downloads", label: "Most popular" },
	{ value: "updated", label: "Recently updated" },
	{ value: "newest", label: "Newest" },
]

function BrowseView({
	server,
	kind,
	installed,
	installingId,
	onInstall,
}: {
	server: ServerConfig
	kind: { noun: string; nouns: string }
	installed: InstalledPlugin[]
	installingId: string | null
	onInstall: (project: PluginProject, version?: PluginVersion) => void
}) {
	const hangarAvailable = server.core === "paper" || server.core === "purpur"
	const [source, setSource] = useState<PluginSource>("modrinth")
	const [input, setInput] = useState("")
	const [query, setQuery] = useState("")
	const [sort, setSort] = useState("downloads")
	const [compatibleOnly, setCompatibleOnly] = useState(true)
	const [selected, setSelected] = useState<PluginProject | null>(null)

	useEffect(() => {
		const t = setTimeout(() => setQuery(input.trim()), 350)
		return () => clearTimeout(t)
	}, [input])

	const search = usePluginSearch(server.id, source, query, sort, compatibleOnly)
	const results = useMemo(() => search.data?.pages.flatMap((p) => p.items) ?? [], [search.data])
	const total = search.data?.pages[0]?.total ?? 0

	const isInstalled = (p: PluginProject) =>
		installed.some((i) => i.projectId === p.id || i.name.toLowerCase() === p.title.toLowerCase())

	return (
		<div className="flex flex-col gap-3">
			<div className="relative">
				<Search className="absolute top-1/2 left-3.5 size-4 -translate-y-1/2 text-zinc-500" />
				<Input
					value={input}
					onChange={(e) => setInput(e.target.value)}
					placeholder={`Search ${kind.nouns.toLowerCase()}, e.g. ${kind.noun === "mod" ? "Lithium" : "LuckPerms"}`}
					autoCapitalize="off"
					autoCorrect="off"
					enterKeyHint="search"
					className="h-11 rounded-xl border-zinc-800 bg-zinc-900/60 pl-10 text-sm"
				/>
			</div>

			<div className="flex flex-wrap items-center gap-2">
				{hangarAvailable && (
					<Segmented
						value={source}
						onChange={setSource}
						options={[
							{ value: "modrinth", label: "Modrinth" },
							{ value: "hangar", label: "Hangar" },
						]}
						className="min-w-44 flex-1 sm:flex-none"
					/>
				)}
				<Select items={SORT_OPTIONS} value={sort} onValueChange={(v) => v && setSort(v)}>
					<SelectTrigger className="h-10 w-40 rounded-xl border-zinc-800 bg-zinc-900/60 text-xs">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{SORT_OPTIONS.map((o) => (
							<SelectItem key={o.value} value={o.value}>
								{o.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
				<div className="flex h-10 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 text-xs text-zinc-300">
					<Switch
						checked={compatibleOnly}
						onCheckedChange={setCompatibleOnly}
						aria-label={`Only show ${server.gameVersion} compatible results`}
					/>
					For {server.gameVersion}
				</div>
			</div>

			{search.error ? <ErrorNote>Search failed: {String(search.error)}</ErrorNote> : null}

			{search.isLoading ? (
				<div className="flex justify-center py-10">
					<Loader2 className="size-5 animate-spin text-zinc-500" />
				</div>
			) : results.length === 0 ? (
				<Card>
					<EmptyState
						icon={Search}
						title="Nothing found"
						description={
							compatibleOnly
								? `No ${kind.nouns.toLowerCase()} match for Minecraft ${server.gameVersion}. Try turning off the version filter.`
								: "Try another search."
						}
					/>
				</Card>
			) : (
				<>
					<p className="px-1 text-[11px] text-zinc-500">{formatCount(total)} results</p>
					<div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
						{results.map((p) => (
							<ResultCard
								key={`${p.source}:${p.id}`}
								project={p}
								installed={isInstalled(p)}
								installing={installingId === p.id}
								onOpen={() => setSelected(p)}
								onInstall={() => onInstall(p)}
							/>
						))}
					</div>
					{search.hasNextPage && (
						<Button
							variant="outline"
							onClick={() => search.fetchNextPage()}
							disabled={search.isFetchingNextPage}
							className="h-11 rounded-xl border-zinc-800"
						>
							{search.isFetchingNextPage ? (
								<Loader2 className="size-4 animate-spin" />
							) : (
								"Load more"
							)}
						</Button>
					)}
				</>
			)}

			<PluginDetailsSheet
				serverId={server.id}
				gameVersion={server.gameVersion}
				project={selected}
				installed={selected ? isInstalled(selected) : false}
				installing={selected ? installingId === selected.id : false}
				onInstall={onInstall}
				onClose={() => setSelected(null)}
			/>
		</div>
	)
}

function ResultCard({
	project,
	installed,
	installing,
	onOpen,
	onInstall,
}: {
	project: PluginProject
	installed: boolean
	installing: boolean
	onOpen: () => void
	onInstall: () => void
}) {
	return (
		<div className="flex min-w-0 items-start gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-3 transition-colors hover:border-zinc-700">
			<button
				type="button"
				onClick={onOpen}
				className="flex min-w-0 flex-1 items-start gap-3 text-left"
			>
				<PluginIcon url={project.iconUrl} name={project.title} className="size-12" />
				<div className="min-w-0 flex-1">
					<p className="truncate font-semibold text-sm text-zinc-100">{project.title}</p>
					<p className="truncate text-[11px] text-zinc-500">
						{project.author} · <Download className="inline size-3" />{" "}
						{formatCount(project.downloads)}
					</p>
					<p className="mt-1 line-clamp-2 text-xs text-zinc-400 leading-relaxed">
						{project.description}
					</p>
				</div>
			</button>
			<Button
				size="sm"
				onClick={onInstall}
				disabled={installed || installing}
				aria-label={installed ? `${project.title} is installed` : `Install ${project.title}`}
				className={cn(
					"size-9 shrink-0 rounded-xl p-0",
					installed
						? "bg-emerald-500/15 text-emerald-300"
						: "bg-emerald-600 text-white hover:bg-emerald-500",
				)}
			>
				{installing ? (
					<Loader2 className="size-4 animate-spin" />
				) : installed ? (
					<Check className="size-4" />
				) : (
					<Download className="size-4" />
				)}
			</Button>
		</div>
	)
}
