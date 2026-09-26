import {
	ChevronRight,
	Clock,
	Cpu,
	Loader2,
	Moon,
	Play,
	Server,
	Square,
	Terminal,
	Users,
	Zap,
} from "lucide-react"
import type { ServerConfig, ServerStatus } from "@/bindings"
import { Button } from "@/components/ui/button"
import { formatUptime } from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import { useServerControls, useStartupStep } from "@/services/hosting"
import {
	useLiveUptime,
	useOnlinePlayers,
	useServerIcon,
	useServerPropertiesAll,
	useServerStatus,
} from "@/services/server-data"
import { useServerLogs } from "@/services/server-service"
import { MotdText } from "../shared/motd"
import { Card, ErrorNote, PlayerAvatar } from "../shared/primitives"
import { JoinCard, PerformanceCard, StorageCard } from "./overview-cards"

export type WorkspaceTab = "overview" | "console" | "players" | "map" | "plugins" | "settings"

export function OverviewPanel({
	server,
	onNavigate,
}: {
	server: ServerConfig
	onNavigate: (tab: WorkspaceTab) => void
}) {
	return (
		<div className="flex flex-col gap-4">
			<ServerHero server={server} />
			<StatTiles server={server} onNavigate={onNavigate} />
			<PerformanceCard server={server} />
			<div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
				<JoinCard server={server} />
				<ConsolePeek server={server} onOpen={() => onNavigate("console")} />
			</div>
			<StorageCard server={server} />
		</div>
	)
}

export function StatusPill({ status }: { status: ServerStatus }) {
	const styles: Record<ServerStatus, string> = {
		running: "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
		starting: "border-sky-500/30 bg-sky-500/10 text-sky-300",
		stopping: "border-amber-500/30 bg-amber-500/10 text-amber-300",
		sleeping: "border-violet-500/30 bg-violet-500/10 text-violet-300",
		stopped: "border-zinc-700 bg-zinc-800/60 text-zinc-400",
	}
	return (
		<span
			className={cn(
				"inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 font-medium text-[11px] capitalize",
				styles[status],
			)}
		>
			<span
				className={cn(
					"size-1.5 rounded-full bg-current",
					(status === "running" || status === "starting") && "animate-pulse",
				)}
			/>
			{status}
		</span>
	)
}

/** Minecraft's connection bars: all five lit when online, crossed out when not */
function SignalBars({ online }: { online: boolean }) {
	return (
		<span className="flex h-3 items-end gap-px" aria-hidden>
			{[0.35, 0.5, 0.65, 0.8, 1].map((h) => (
				<span
					key={h}
					className={cn("w-[3px]", online ? "bg-emerald-400" : "bg-zinc-700")}
					style={{ height: `${h * 100}%` }}
				/>
			))}
		</span>
	)
}

/**
 * The server as players see it in Minecraft's multiplayer list (icon, name, MOTD,
 * player count), with start/sleep/stop underneath
 */
function ServerHero({ server }: { server: ServerConfig }) {
	const { status, isRunning } = useServerStatus(server.id)
	const { data: icon } = useServerIcon(server.id)
	const { values } = useServerPropertiesAll(server.id)
	const { data: players = [] } = useOnlinePlayers(server.id, isRunning)
	const controls = useServerControls(server.id)
	const step = useStartupStep(server.id)
	const shownStatus: ServerStatus =
		controls.isStartRequested && status === "stopped" ? "starting" : status
	const online = shownStatus === "running" || shownStatus === "sleeping"
	const maxPlayers = values.get("max-players") || "20"

	return (
		<Card className="flex flex-col gap-3 p-3 sm:p-4">
			<p className="px-1 text-[10px] text-zinc-500 uppercase tracking-wider">How players see it</p>
			<div className="flex items-center gap-3 rounded-xl bg-black/50 p-2.5 ring-1 ring-white/5">
				<div className="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-zinc-900">
					{icon ? (
						<img src={icon} alt="" className="size-full object-cover [image-rendering:pixelated]" />
					) : (
						<Server className="size-6 text-zinc-600" />
					)}
				</div>
				<div className="flex min-w-0 flex-1 flex-col gap-0.5">
					<div className="flex items-center gap-2">
						<p className="min-w-0 flex-1 truncate font-semibold text-sm text-zinc-100">
							{server.name}
						</p>
						<span className="flex shrink-0 items-center gap-1.5 font-mono text-[11px] text-zinc-400">
							{online ? `${players.length}/${maxPlayers}` : ""}
							<SignalBars online={online} />
						</span>
					</div>
					{online ? (
						<MotdText motd={values.get("motd") || server.name} className="text-[11px]" />
					) : (
						<p className="font-mono text-[11px] text-rose-400/80">
							{shownStatus === "starting"
								? "Starting up..."
								: shownStatus === "stopping"
									? "Shutting down..."
									: "Can't connect to server"}
						</p>
					)}
				</div>
			</div>

			{shownStatus === "starting" && (
				<div className="flex items-center gap-2.5 rounded-xl bg-sky-500/[0.07] px-3 py-2.5 ring-1 ring-sky-500/20">
					<Loader2 className="size-4 shrink-0 animate-spin text-sky-400" />
					<span className="truncate text-sky-200 text-xs">{step ?? "Preparing..."}</span>
				</div>
			)}
			{shownStatus === "sleeping" && (
				<div className="rounded-xl bg-violet-500/[0.07] px-3 py-2.5 text-violet-200 text-xs ring-1 ring-violet-500/20">
					Sleeping to save resources. It wakes up automatically when a player joins.
				</div>
			)}
			{controls.error && shownStatus === "stopped" && (
				<ErrorNote>
					<span className="font-semibold">Failed to start. </span>
					{controls.error}
				</ErrorNote>
			)}

			<div className="flex gap-2">
				{shownStatus === "stopped" && (
					<Button
						onClick={controls.start}
						className="h-11 flex-1 gap-2 rounded-xl bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
					>
						<Play className="size-4 fill-current" /> Start server
					</Button>
				)}
				{shownStatus === "sleeping" && (
					<Button
						onClick={controls.start}
						className="h-11 flex-1 gap-2 rounded-xl bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
					>
						<Zap className="size-4" /> Wake up
					</Button>
				)}
				{shownStatus === "running" && (
					<Button
						variant="ghost"
						onClick={controls.sleep}
						className="h-11 flex-1 gap-2 rounded-xl bg-zinc-900 text-zinc-200 hover:bg-zinc-800"
					>
						<Moon className="size-4" /> Sleep
					</Button>
				)}
				{online && (
					<Button
						variant="ghost"
						onClick={controls.stop}
						disabled={controls.isStopping}
						className="h-11 flex-1 gap-2 rounded-xl bg-rose-500/10 text-rose-300 hover:bg-rose-500/20 hover:text-rose-200"
					>
						{controls.isStopping ? (
							<Loader2 className="size-4 animate-spin" />
						) : (
							<Square className="size-3.5 fill-current" />
						)}
						Stop
					</Button>
				)}
				{(shownStatus === "starting" || shownStatus === "stopping") && (
					<Button disabled variant="ghost" className="h-11 flex-1 gap-2 rounded-xl bg-zinc-900">
						<Loader2 className="size-4 animate-spin" />
						{shownStatus === "starting" ? "Starting..." : "Stopping..."}
					</Button>
				)}
			</div>
		</Card>
	)
}

function StatTiles({
	server,
	onNavigate,
}: {
	server: ServerConfig
	onNavigate: (tab: WorkspaceTab) => void
}) {
	const { isRunning } = useServerStatus(server.id)
	const uptimeSeconds = useLiveUptime(server.id)
	const { data: players = [] } = useOnlinePlayers(server.id, isRunning)
	const { values } = useServerPropertiesAll(server.id)
	const max = values.get("max-players") ?? "20"

	return (
		<div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
			<button
				type="button"
				onClick={() => onNavigate("players")}
				className="col-span-2 flex items-center justify-between gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-3.5 text-left transition-colors hover:bg-zinc-900/70 sm:col-span-2"
			>
				<div>
					<p className="flex items-center gap-1.5 text-[11px] text-zinc-500 uppercase tracking-wider">
						<Users className="size-3" /> Players
					</p>
					<p className="mt-1 font-semibold text-lg text-zinc-100">
						{isRunning ? players.length : 0}
						<span className="text-sm text-zinc-600"> / {max}</span>
					</p>
				</div>
				<div className="flex -space-x-2">
					{players.slice(0, 5).map((p) => (
						<PlayerAvatar
							key={p.name}
							name={p.name}
							size={30}
							className="rounded-lg ring-2 ring-zinc-950"
						/>
					))}
					<ChevronRight className="ml-3 size-4 self-center text-zinc-600" />
				</div>
			</button>
			<Tile icon={Clock} label="Uptime" value={isRunning ? formatUptime(uptimeSeconds) : "—"} />
			<Tile
				icon={Cpu}
				label="Memory"
				value={
					server.core === "pumpkin" ? "Native" : `${(server.memoryMaxMb / 1024).toFixed(1)} GB`
				}
			/>
		</div>
	)
}

function Tile({ icon: Icon, label, value }: { icon: typeof Clock; label: string; value: string }) {
	return (
		<div className="rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-3.5">
			<p className="flex items-center gap-1.5 text-[11px] text-zinc-500 uppercase tracking-wider">
				<Icon className="size-3" /> {label}
			</p>
			<p className="mt-1 font-semibold text-lg text-zinc-100">{value}</p>
		</div>
	)
}

function ConsolePeek({ server, onOpen }: { server: ServerConfig; onOpen: () => void }) {
	const { logs } = useServerLogs(server.id)
	const lines = logs.slice(-5)
	return (
		<button
			type="button"
			onClick={onOpen}
			className="flex min-w-0 flex-col rounded-2xl border border-zinc-800/80 bg-[#0a0a0c] text-left transition-colors hover:border-zinc-700"
		>
			<div className="flex items-center justify-between px-4 pt-4 pb-2">
				<span className="flex items-center gap-2 font-semibold text-sm text-zinc-100">
					<Terminal className="size-3.5 text-zinc-400" /> Console
				</span>
				<ChevronRight className="size-4 text-zinc-600" />
			</div>
			<div
				className={cn(
					"flex min-h-24 flex-1 flex-col px-4 pb-4 font-mono text-[10.5px] text-zinc-400 leading-relaxed",
					lines.length === 0 ? "items-center justify-center" : "justify-end",
				)}
			>
				{lines.length === 0 ? (
					<span className="font-sans text-xs text-zinc-600">No output yet</span>
				) : (
					lines.map((line, i) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: log lines are append-only
						<span key={i} className="truncate">
							{line}
						</span>
					))
				)}
			</div>
		</button>
	)
}
