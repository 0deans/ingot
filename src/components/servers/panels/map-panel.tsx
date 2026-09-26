import { useQueryClient } from "@tanstack/react-query"
import { Loader2, Map as MapIcon, RefreshCw } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { dimensionStyle } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import {
	serverKeys,
	useMapDimensions,
	useOnlinePlayers,
	useServerStatus,
} from "@/services/server-data"
import { rpc } from "@/services/server-service"
import { EmptyState, PlayerAvatar } from "../shared/primitives"
import { PlayerSheet } from "./player-sheet"
import { type MapView, WorldMap } from "./world-map"

const LIVE_INTERVAL_MS = 20_000

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
	const { data: dimensions, isLoading } = useMapDimensions(server.id)
	const { data: online = [] } = useOnlinePlayers(server.id, isRunning, 1500)
	const [dimension, setDimension] = useState(focus?.dimension ?? "minecraft:overworld")
	const [view, setView] = useState<MapView | null>(null)
	const [revision, setRevision] = useState(0)
	const [refreshing, setRefreshing] = useState(false)
	const [selected, setSelected] = useState<string | null>(null)

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

	// Live updates: while people play, save regularly and redraw changed regions
	const live = isRunning && online.length > 0
	useEffect(() => {
		if (!live) return
		const timer = setInterval(async () => {
			await rpc.save_server_world(server.id, false).catch(() => {})
			await queryClient.invalidateQueries({ queryKey: serverKeys.dimensions(server.id) })
			setRevision((r) => r + 1)
		}, LIVE_INTERVAL_MS)
		return () => clearInterval(timer)
	}, [live, server.id, queryClient])

	const refresh = async () => {
		setRefreshing(true)
		try {
			if (isRunning) await rpc.save_server_world(server.id, true).catch(() => {})
			await queryClient.invalidateQueries({ queryKey: serverKeys.dimensions(server.id) })
			setRevision((r) => r + 1)
		} finally {
			setRefreshing(false)
		}
	}

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
				revision={revision}
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
							title="The map updates automatically while players are online"
							className="flex h-9 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 px-2.5 font-medium text-[11px] text-emerald-300 backdrop-blur-md"
						>
							<span className="size-1.5 animate-pulse rounded-full bg-emerald-400" />
							Live
						</span>
					)}
					<button
						type="button"
						onClick={refresh}
						disabled={refreshing}
						title={isRunning ? "Save the world and redraw the map" : "Redraw the map"}
						className="pointer-events-auto flex h-9 shrink-0 items-center gap-1.5 rounded-xl border border-white/10 bg-black/60 px-3 font-medium text-xs text-zinc-200 backdrop-blur-md transition-colors hover:bg-black/80"
					>
						<RefreshCw className={cn("size-3.5", refreshing && "animate-spin")} />
						<span className="hidden sm:inline">{refreshing ? "Saving..." : "Refresh"}</span>
					</button>
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
