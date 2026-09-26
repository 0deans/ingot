import { Apple, Heart, Search, Users } from "lucide-react"
import { useMemo, useState } from "react"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { Input } from "@/components/ui/input"
import { dimensionStyle, formatRelativeTime } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import { useKnownPlayers, useOnlinePlayers, useServerStatus } from "@/services/server-data"
import { Card, EmptyState, ErrorNote, PlayerAvatar, Segmented } from "../shared/primitives"
import { AccessPanel } from "./access-panel"
import { PlayerSheet } from "./player-sheet"

type View = "online" | "all" | "access"

export function PlayersPanel({
	server,
	onShowOnMap,
}: {
	server: ServerConfig
	onShowOnMap?: (player: PlayerDetails) => void
}) {
	const { isRunning } = useServerStatus(server.id)
	const [view, setView] = useState<View>("online")
	const [selected, setSelected] = useState<string | null>(null)
	const online = useOnlinePlayers(server.id, isRunning)
	const onlinePlayers = isRunning ? (online.data ?? []) : []

	return (
		<div className="flex flex-col gap-4">
			<Segmented
				value={view}
				onChange={setView}
				options={[
					{ value: "online", label: `Online${isRunning ? ` · ${onlinePlayers.length}` : ""}` },
					{ value: "all", label: "All players" },
					{ value: "access", label: "Access" },
				]}
			/>

			{view === "online" && (
				<OnlineList
					isRunning={isRunning}
					players={onlinePlayers}
					error={online.error}
					isLoading={online.isLoading}
					onSelect={setSelected}
				/>
			)}
			{view === "all" && (
				<AllPlayersList serverId={server.id} isRunning={isRunning} onSelect={setSelected} />
			)}
			{view === "access" && <AccessPanel server={server} />}

			<PlayerSheet
				serverId={server.id}
				name={selected}
				isRunning={isRunning}
				onClose={() => setSelected(null)}
				onShowOnMap={
					onShowOnMap
						? (p) => {
								setSelected(null)
								onShowOnMap(p)
							}
						: undefined
				}
			/>
		</div>
	)
}

function OnlineList({
	isRunning,
	players,
	error,
	isLoading,
	onSelect,
}: {
	isRunning: boolean
	players: PlayerDetails[]
	error: unknown
	isLoading: boolean
	onSelect: (name: string) => void
}) {
	if (!isRunning) {
		return (
			<Card>
				<EmptyState
					icon={Users}
					title="Server is offline"
					description="Start the server to see who's playing, their health, inventory and location."
				/>
			</Card>
		)
	}
	if (String(error ?? "").includes("still starting")) {
		return (
			<Card>
				<EmptyState
					icon={Users}
					title="Server is starting..."
					description="Players will show up once the world has loaded."
				/>
			</Card>
		)
	}
	if (error && players.length === 0) {
		return (
			<ErrorNote>
				Couldn't read live player data: {String(error)}. Live data needs a Java server (Paper,
				Purpur, Fabric or Vanilla).
			</ErrorNote>
		)
	}
	if (players.length === 0) {
		return (
			<Card>
				<EmptyState
					icon={Users}
					title={isLoading ? "Checking who's online..." : "Nobody is online"}
					description={isLoading ? undefined : "Players will show up here as soon as they join."}
				/>
			</Card>
		)
	}
	return (
		<div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
			{players.map((p) => (
				<OnlinePlayerCard key={p.name} player={p} onClick={() => onSelect(p.name)} />
			))}
		</div>
	)
}

function OnlinePlayerCard({ player, onClick }: { player: PlayerDetails; onClick: () => void }) {
	const dim = dimensionStyle(player.dimension)
	const health = Math.min(100, ((player.health + player.absorption) / player.maxHealth) * 100)
	return (
		<button
			type="button"
			onClick={onClick}
			className="flex items-center gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-left transition-colors hover:border-zinc-700 hover:bg-zinc-900/70 active:scale-[0.99]"
		>
			<PlayerAvatar name={player.name} size={44} online />
			<div className="min-w-0 flex-1">
				<div className="flex items-center justify-between gap-2">
					<span className="truncate font-semibold text-sm text-zinc-100">{player.name}</span>
					<span className={cn("shrink-0 rounded-full border px-1.5 py-px text-[10px]", dim.badge)}>
						{dim.label}
					</span>
				</div>
				<div className="mt-2 grid grid-cols-2 gap-2">
					<MiniBar icon={Heart} pct={health} color="#f43f5e" />
					<MiniBar icon={Apple} pct={(player.food / 20) * 100} color="#f59e0b" />
				</div>
				<p className="mt-1.5 truncate font-mono text-[10px] text-zinc-500">
					{player.gamemode} · {Math.floor(player.x)}, {Math.floor(player.y)}, {Math.floor(player.z)}
				</p>
			</div>
		</button>
	)
}

function MiniBar({ icon: Icon, pct, color }: { icon: typeof Heart; pct: number; color: string }) {
	return (
		<div className="flex items-center gap-1.5">
			<Icon className="size-3 shrink-0" style={{ color }} />
			<div className="h-1 flex-1 overflow-hidden rounded-full bg-zinc-800">
				<div className="h-full rounded-full" style={{ width: `${pct}%`, background: color }} />
			</div>
		</div>
	)
}

function AllPlayersList({
	serverId,
	isRunning,
	onSelect,
}: {
	serverId: string
	isRunning: boolean
	onSelect: (name: string) => void
}) {
	const { data = [], isLoading } = useKnownPlayers(serverId, isRunning)
	const [query, setQuery] = useState("")
	const filtered = useMemo(
		() => data.filter((p) => p.name.toLowerCase().includes(query.trim().toLowerCase())),
		[data, query],
	)

	if (!isLoading && data.length === 0) {
		return (
			<Card>
				<EmptyState
					icon={Users}
					title="No players yet"
					description="Everyone who has joined this server will be listed here, with their last saved inventory."
				/>
			</Card>
		)
	}

	return (
		<div className="flex flex-col gap-3">
			{data.length > 6 && (
				<div className="relative">
					<Search className="absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-zinc-500" />
					<Input
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						placeholder="Search players"
						className="h-10 rounded-xl border-zinc-800 bg-zinc-900/60 pl-9 text-sm"
					/>
				</div>
			)}
			<Card className="divide-y divide-zinc-800/70 overflow-hidden">
				{filtered.map((p) => (
					<button
						key={p.uuid || p.name}
						type="button"
						onClick={() => onSelect(p.name)}
						className="flex w-full items-center gap-3 px-3.5 py-3 text-left transition-colors hover:bg-zinc-900/70"
					>
						<PlayerAvatar name={p.name} size={34} online={p.online} />
						<div className="min-w-0 flex-1">
							<p className="truncate font-medium text-sm text-zinc-100">{p.name}</p>
							<p className="text-[11px] text-zinc-500">
								{p.online ? "Online now" : `Last seen ${formatRelativeTime(p.lastSeen)}`}
							</p>
						</div>
					</button>
				))}
			</Card>
		</div>
	)
}
