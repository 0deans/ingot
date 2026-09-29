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
	Users,
} from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
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
import { CompanionCard } from "./companion-card"
import { PluginDetailsSheet, PluginIcon } from "./plugin-details-sheet"

/** What the server core can load; null = nothing */
type AddonKind = { kind: "mod" | "plugin"; noun: string; nouns: string }

/** Whether a server runs plugins or mods. `kind` picks the right wording (i18next context). */
export function addonKind(core: ServerConfig["core"]): AddonKind | null {
	if (core === "fabric" || core === "neoforge" || core === "forge" || core === "quilt")
		return { kind: "mod", noun: "mod", nouns: "Mods" }
	if (core === "paper" || core === "purpur" || core === "folia")
		return { kind: "plugin", noun: "plugin", nouns: "Plugins" }
	return null
}

interface Notice {
	kind: "success" | "error"
	text: string
}

export function PluginsPanel({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const kind = addonKind(server.core)
	const { isRunning } = useServerStatus(server.id)
	const [view, setView] = useState<"installed" | "browse">("installed")
	const [notice, setNotice] = useState<Notice | null>(null)
	const [needsRestart, setNeedsRestart] = useState(false)
	const installed = useInstalledPlugins(server.id)
	const actions = usePluginActions(server.id)
	// Ingot's own plugin has its own card above the list
	const userPlugins = (installed.data ?? []).filter((p) => !p.system)

	useEffect(() => {
		if (!isRunning) setNeedsRestart(false)
	}, [isRunning])

	if (!kind) {
		return (
			<Card>
				<EmptyState
					icon={Puzzle}
					title={t("plugins.unsupported", {
						core: server.core === "pumpkin" ? "Pumpkin" : t("versionChange.vanilla"),
					})}
					description={t("plugins.unsupportedHint")}
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
			const extras = report.installed.filter((title) => title !== project.title)
			const done = extras.length
				? t("plugins.installedWith", { name: project.title, extras: extras.join(", ") })
				: t("plugins.installed", { name: project.title })
			changed(report.warnings.length ? `${done} ${report.warnings.join(". ")}.` : done)
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
						label: installed.data
							? `${t("common.installed")} · ${installed.data.length}`
							: t("common.installed"),
					},
					{ value: "browse", label: t("plugins.browse", { context: kind.kind }) },
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

			{view === "installed" && <CompanionCard server={server} onChanged={changed} />}
			{view === "installed" ? (
				<InstalledList
					server={server}
					kind={kind}
					plugins={userPlugins}
					hasSystem={userPlugins.length < (installed.data?.length ?? 0)}
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
	const { t } = useTranslation()
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
		<div className="flex flex-wrap items-center justify-between gap-x-3 gap-y-2 rounded-xl border border-amber-500/20 bg-amber-500/10 py-2 pr-2 pl-3">
			<span className="min-w-40 flex-1 text-amber-200 text-xs">{t("plugins.restartToApply")}</span>
			<Button
				size="sm"
				onClick={restart}
				disabled={restarting}
				className="ml-auto h-8 shrink-0 gap-1.5 rounded-lg bg-amber-500/20 text-amber-100 hover:bg-amber-500/30"
			>
				{restarting ? (
					<Loader2 className="size-3.5 animate-spin" />
				) : (
					<RotateCw className="size-3.5" />
				)}
				{t("plugins.restart")}
			</Button>
		</div>
	)
}

// ─── Installed ────────────────────────────────────────────────────────────────

function InstalledList({
	server,
	kind,
	plugins,
	hasSystem,
	loading,
	onBrowse,
	onChanged,
}: {
	server: ServerConfig
	kind: AddonKind
	plugins: InstalledPlugin[]
	/** Ingot's own plugin/mod is installed (shown above this list) */
	hasSystem: boolean
	loading: boolean
	onBrowse: () => void
	onChanged: (text: string, error?: boolean) => void
}) {
	const { t } = useTranslation()
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
	// Only Ingot's own: a short hint instead of a big "nothing installed"
	if (plugins.length === 0 && hasSystem) {
		return (
			<Card className="flex flex-wrap items-center justify-between gap-x-3 gap-y-2 px-4 py-3">
				<p className="min-w-40 flex-1 text-sm text-zinc-400">
					{t("plugins.noOther", { context: kind.kind })}
				</p>
				<Button
					variant="outline"
					onClick={onBrowse}
					className="ml-auto h-9 shrink-0 gap-2 rounded-xl border-zinc-800"
				>
					<Search className="size-4" /> {t("plugins.browse", { context: kind.kind })}
				</Button>
			</Card>
		)
	}
	if (plugins.length === 0) {
		return (
			<Card>
				<EmptyState
					icon={Package}
					title={t("plugins.none", { context: kind.kind })}
					description={
						server.core === "paper" || server.core === "purpur"
							? t("plugins.findHangar", { context: kind.kind })
							: t("plugins.find", { context: kind.kind })
					}
					action={
						<Button
							onClick={onBrowse}
							className="h-10 gap-2 rounded-xl bg-emerald-600 text-white hover:bg-emerald-500"
						>
							<Search className="size-4" /> {t("plugins.browse", { context: kind.kind })}
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
												{t("plugins.updateAvailable", { version: update.latest.versionNumber })}
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
										onChanged(
											enabled
												? t("plugins.enabled", { name: p.name })
												: t("plugins.disabled", { name: p.name }),
										)
									} catch (e) {
										onChanged(String(e), true)
									}
								}}
								aria-label={
									p.enabled
										? t("plugins.disable", { name: p.name })
										: t("plugins.enable", { name: p.name })
								}
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
									onChanged(t("plugins.updated", { name: current.name }))
									setSelected(null)
								} catch (e) {
									onChanged(String(e), true)
								}
							}}
							onRemove={async () => {
								try {
									await actions.remove.mutateAsync(current.fileName)
									onChanged(t("plugins.removed", { name: current.name }))
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
	const { t } = useTranslation()
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
						{plugin.version ?? t("plugins.unknownVersion")}
						{plugin.authors.length > 0 &&
							` · ${t("plugins.byAuthors", { authors: plugin.authors.join(", ") })}`}
					</p>
				</div>
			</div>
			{plugin.description && (
				<p className="text-sm text-zinc-300 leading-relaxed">{plugin.description}</p>
			)}
			<dl className="grid grid-cols-2 gap-2 text-xs">
				<div className="rounded-xl bg-zinc-900/60 p-2.5">
					<dt className="text-zinc-500">{t("plugins.file")}</dt>
					<dd className="truncate font-mono text-zinc-300">{plugin.fileName}</dd>
				</div>
				<div className="rounded-xl bg-zinc-900/60 p-2.5">
					<dt className="text-zinc-500">{t("plugins.size")}</dt>
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
					{t("plugins.updateTo", { version: update.versionNumber })}
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
						<ExternalLink className="size-4" /> {t("pluginDetails.page")}
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
						{t("plugins.reallyDelete")}
					</Button>
				) : (
					<Button
						variant="destructive"
						onClick={() => setConfirm(true)}
						className="h-10 flex-1 gap-1.5 rounded-xl"
					>
						<Trash2 className="size-4" /> {t("common.delete")}
					</Button>
				)}
			</div>
		</>
	)
}

// ─── Browse ───────────────────────────────────────────────────────────────────

const SORT_VALUES = ["downloads", "updated", "newest"] as const

function BrowseView({
	server,
	kind,
	installed,
	installingId,
	onInstall,
}: {
	server: ServerConfig
	kind: AddonKind
	installed: InstalledPlugin[]
	installingId: string | null
	onInstall: (project: PluginProject, version?: PluginVersion) => void
}) {
	const { t } = useTranslation()
	const hangarAvailable = server.core === "paper" || server.core === "purpur"
	const sortOptions = SORT_VALUES.map((value) => ({ value, label: t(`plugins.sort.${value}`) }))
	const [source, setSource] = useState<PluginSource>("modrinth")
	const [input, setInput] = useState("")
	const [query, setQuery] = useState("")
	const [sort, setSort] = useState("downloads")
	const [compatibleOnly, setCompatibleOnly] = useState(true)
	const [selected, setSelected] = useState<PluginProject | null>(null)

	useEffect(() => {
		const timer = setTimeout(() => setQuery(input.trim()), 350)
		return () => clearTimeout(timer)
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
					placeholder={t("plugins.searchPlaceholder", { context: kind.kind })}
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
				<Select items={sortOptions} value={sort} onValueChange={(v) => v && setSort(v)}>
					<SelectTrigger className="h-10 w-40 rounded-xl border-zinc-800 bg-zinc-900/60 text-xs">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{sortOptions.map((o) => (
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
						aria-label={t("plugins.compatibleOnly", { version: server.gameVersion })}
					/>
					{t("plugins.forVersion", { version: server.gameVersion })}
				</div>
			</div>

			{search.error ? (
				<ErrorNote>{t("plugins.searchFailed", { error: String(search.error) })}</ErrorNote>
			) : null}

			{search.isLoading ? (
				<div className="flex justify-center py-10">
					<Loader2 className="size-5 animate-spin text-zinc-500" />
				</div>
			) : results.length === 0 ? (
				<Card>
					<EmptyState
						icon={Search}
						title={t("plugins.nothingFound")}
						description={
							compatibleOnly
								? t("plugins.noneForVersion", { context: kind.kind, version: server.gameVersion })
								: t("plugins.tryAnother")
						}
					/>
				</Card>
			) : (
				<>
					<p className="px-1 text-[11px] text-zinc-500">
						{t("plugins.results", { count: total, formatted: formatCount(total) })}
					</p>
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
								t("plugins.loadMore")
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
	const { t } = useTranslation()
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
					{project.playersNeedIt && (
						<p
							className="mt-1.5 inline-flex items-center gap-1 rounded-full bg-amber-500/10 px-2 py-0.5 font-medium text-[10px] text-amber-300"
							title={t("plugins.playersNeedItTitle")}
						>
							<Users className="size-3" />
							{t("plugins.playersNeedIt")}
						</p>
					)}
				</div>
			</button>
			<Button
				size="sm"
				onClick={onInstall}
				disabled={installed || installing}
				aria-label={
					installed
						? t("plugins.isInstalled", { name: project.title })
						: t("plugins.install", { name: project.title })
				}
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
