import { useQueryClient } from "@tanstack/react-query"
import { Check, Loader2, Map as MapIcon, Radio, RefreshCw } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { dimensionStyle } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import {
	serverKeys,
	useCompanionStatus,
	useMapDimensions,
	useOnlinePlayers,
	usePluginActions,
	useServerStatus,
} from "@/services/server-data"
import { EmptyState, PlayerAvatar } from "../shared/primitives"
import { PlayerSheet } from "./player-sheet"
import { type MapView, WorldMap } from "./world-map"

/** How often to look for changed regions: live chunks from the Ingot plugin, else saves */
const LIVE_INTERVAL_MS = 3_000
const SAVED_INTERVAL_MS = 10_000

export function MapPanel({
	server,
	focus,
	className,
}: {
	server: ServerConfig
	/** Player to center on when opened from the player list */
	focus?: PlayerDetails | null
	className?: string
}) {
	const queryClient = useQueryClient()
	const { isRunning } = useServerStatus(server.id)
	const { data: dimensions, isLoading, refetch } = useMapDimensions(server.id)
	const { data: online = [] } = useOnlinePlayers(server.id, isRunning, 1500)
	const [dimension, setDimension] = useState(focus?.dimension ?? "minecraft:overworld")
	const [view, setView] = useState<MapView | null>(null)
	const [refreshing, setRefreshing] = useState(false)
	/** What the last refresh found, shown briefly on the button */
	const [refreshNote, setRefreshNote] = useState<string | null>(null)
	const [selected, setSelected] = useState<string | null>(null)
	const { data: companion } = useCompanionStatus(server.id)
	const { installCompanion } = usePluginActions(server.id)
	// Installed while this server was running: it loads on the next start
	const [installedAt, setInstalledAt] = useState<string | null>(null)
	const companionReady = companion?.fileName != null && companion.enabled

	const current = dimensions?.find((d) => d.id === dimension) ?? dimensions?.[0]
	const players = useMemo(
		() => (isRunning ? online.filter((p) => p.dimension === current?.id) : []),
		[online, current?.id, isRunning],
	)

	// Initial view: the focused player, else the first player here, else the middle of the explored area
	const initialized = useRef<string | null>(null)
	useEffect(() => {
		if (!current || initialized.current === current.id) return
		initialized.current = current.id
		const target = focus?.dimension === current.id ? focus : players[0]
		if (target) {
			setView({ x: target.x, z: target.z, scale: 2 })
			return
		}
		const xs = current.regions.map((r) => r.x)
		const zs = current.regions.map((r) => r.z)
		setView({
			x: ((Math.min(...xs) + Math.max(...xs) + 1) / 2) * 512,
			z: ((Math.min(...zs) + Math.max(...zs) + 1) / 2) * 512,
			scale: 0.5,
		})
	}, [current, focus, players])

	useEffect(() => {
		if (focus) {
			setDimension(focus.dimension)
			initialized.current = null
		}
	}, [focus])

	// The map never makes the server save. It re-lists regions now and then and redraws
	// the ones that changed: live chunks the Ingot plugin reads from memory, or whatever
	// the server saved on its own schedule.
	const live = isRunning && companionReady && installedAt !== server.id
	useEffect(() => {
		if (!isRunning) {
			setInstalledAt(null)
			return
		}
		const timer = setInterval(
			() => queryClient.invalidateQueries({ queryKey: serverKeys.dimensions(server.id) }),
			live ? LIVE_INTERVAL_MS : SAVED_INTERVAL_MS,
		)
		return () => clearInterval(timer)
	}, [isRunning, live, server.id, queryClient])

	// Re-reads what the server has written and says how much changed, so the button
	// visibly does something even when the answer is "nothing new"
	const refresh = async () => {
		setRefreshing(true)
		setRefreshNote(null)
		const before = current?.regions ?? []
		const [{ data: fresh }] = await Promise.all([refetch(), new Promise((r) => setTimeout(r, 600))])
		const after = fresh?.find((d) => d.id === current?.id)?.regions ?? []
		const changed = after.filter(
			(r) => !before.some((b) => b.x === r.x && b.z === r.z && b.modified === r.modified),
		).length
		setRefreshing(false)
		setRefreshNote(
			changed === 0 ? "Up to date" : `Updated ${changed} ${changed === 1 ? "area" : "areas"}`,
		)
	}
	useEffect(() => {
		if (!refreshNote) return
		const timer = setTimeout(() => setRefreshNote(null), 2500)
		return () => clearTimeout(timer)
	}, [refreshNote])

	if (isLoading) {
		return (
			<div className={cn("flex items-center justify-center", className)}>
				<Loader2 className="size-5 animate-spin text-zinc-500" />
			</div>
		)
	}

	if (!current || !view) {
		return (
			<div className={cn("flex items-center justify-center", className)}>
				<EmptyState
					icon={MapIcon}
					title="No world yet"
					description="Start the server once so it generates a world. Explored areas will appear on the map."
				/>
			</div>
		)
	}

	return (
		<div className={cn("relative flex flex-col overflow-hidden", className)}>
			<WorldMap
				serverId={server.id}
				dimension={current.id}
				regions={current.regions}
				players={players}
				view={view}
				onViewChange={setView}
				onSelectPlayer={setSelected}
				voidColor={dimensionStyle(current.id).void}
			/>

			{/* Top overlay: dimension switcher and refresh */}
			<div className="pointer-events-none absolute inset-x-3 top-3 flex items-start justify-between gap-2">
				<div className="pointer-events-auto flex gap-1 overflow-x-auto rounded-xl border border-white/10 bg-black/60 p-1 backdrop-blur-md [scrollbar-width:none]">
					{dimensions?.map((d) => (
						<button
							key={d.id}
							type="button"
							onClick={() => {
								setDimension(d.id)
								initialized.current = null
							}}
							className={cn(
								"whitespace-nowrap rounded-lg px-3 py-1.5 font-medium text-xs transition-colors",
								d.id === current.id
									? "bg-white/15 text-white"
									: "text-zinc-400 hover:text-zinc-200",
							)}
						>
							{dimensionStyle(d.id).label}
						</button>
					))}
				</div>
				<div className="pointer-events-auto flex shrink-0 items-center gap-1.5">
					{live && (
						<span
							title="The Ingot plugin keeps the map current from server memory"
							className="flex h-9 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 px-2.5 font-medium text-[11px] text-emerald-300 backdrop-blur-md"
						>
							<span className="size-1.5 animate-pulse rounded-full bg-emerald-400" />
							Live
						</span>
					)}
					{isRunning && installedAt === server.id && (
						<span className="flex h-9 items-center rounded-xl border border-amber-500/20 bg-black/60 px-2.5 font-medium text-[11px] text-amber-200 backdrop-blur-md">
							Restart to go live
						</span>
					)}
					{companion?.supported && companion.fileName === null && (
						<button
							type="button"
							onClick={async () => {
								await installCompanion.mutateAsync().catch(() => {})
								if (isRunning) setInstalledAt(server.id)
							}}
							disabled={installCompanion.isPending}
							title="Install the Ingot plugin to update the map in real time, without saving the world"
							className="flex h-9 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 px-2.5 font-medium text-[11px] text-zinc-200 backdrop-blur-md transition-colors hover:bg-black/80"
						>
							{installCompanion.isPending ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<Radio className="size-3.5 text-emerald-400" />
							)}
							Make live
						</button>
					)}
					{/* Live maps update themselves; otherwise re-read what the server saved */}
					{!live && (
						<button
							type="button"
							onClick={refresh}
							disabled={refreshing}
							title="Check for newly saved areas"
							className="pointer-events-auto flex h-9 shrink-0 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 px-3 font-medium text-xs text-zinc-200 backdrop-blur-md transition-colors hover:bg-black/80"
						>
							{refreshNote ? (
								<Check className="size-3.5 text-emerald-400" />
							) : (
								<RefreshCw className={cn("size-3.5", refreshing && "animate-spin")} />
							)}
							{refreshNote ? (
								<span>{refreshNote}</span>
							) : (
								<span className="hidden sm:inline">{refreshing ? "Checking..." : "Refresh"}</span>
							)}
						</button>
					)}
				</div>
			</div>

			{/* Players in this dimension */}
			{players.length > 0 && (
				<div className="absolute top-16 left-3 flex max-w-[60%] flex-col gap-1.5">
					{players.map((p) => (
						<button
							key={p.name}
							type="button"
							onClick={() =>
								setView((v) => ({ x: p.x, z: p.z, scale: Math.max(v?.scale ?? 2, 2) }))
							}
							className="flex items-center gap-2 rounded-xl border border-white/10 bg-black/60 py-1 pr-3 pl-1 text-left backdrop-blur-md transition-colors hover:bg-black/80"
						>
							<PlayerAvatar name={p.name} size={24} />
							<span className="truncate font-medium text-white text-xs">{p.name}</span>
						</button>
					))}
				</div>
			)}

			<PlayerSheet
				serverId={server.id}
				name={selected}
				isRunning={isRunning}
				onClose={() => setSelected(null)}
			/>
		</div>
	)
}
