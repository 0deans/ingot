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
import { JoinCard, PerformanceCard } from "./overview-cards"

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

function ServerHero({ server }: { server: ServerConfig }) {
	const { status } = useServerStatus(server.id)
	const { data: icon } = useServerIcon(server.id)
	const { values } = useServerPropertiesAll(server.id)
	const controls = useServerControls(server.id)
	const step = useStartupStep(server.id)
	const shownStatus: ServerStatus =
		controls.isStartRequested && status === "stopped" ? "starting" : status

	return (
		<Card className="relative overflow-hidden">
			<div className="pointer-events-none absolute inset-x-0 top-0 h-28 bg-gradient-to-b from-emerald-500/[0.07] to-transparent" />
			<div className="relative flex flex-col gap-4 p-4 sm:p-5">
				<div className="flex items-start gap-3.5">
					<div className="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-900">
						{icon ? (
							<img
								src={icon}
								alt=""
								className="size-full object-cover [image-rendering:pixelated]"
							/>
						) : (
							<Server className="size-6 text-zinc-500" />
						)}
					</div>
					{/* Name and status are already in the page header / mobile top bar */}
					<div className="min-w-0 flex-1 pt-0.5">
						<MotdText
							motd={values.get("motd") || server.name}
							className="text-[13px] leading-relaxed"
						/>
						<p className="mt-1.5 font-mono text-[11px] text-zinc-500">localhost:{server.port}</p>
					</div>
				</div>

				{shownStatus === "starting" && (
					<div className="flex items-center gap-2.5 rounded-xl border border-sky-500/20 bg-sky-500/[0.07] px-3 py-2.5">
						<Loader2 className="size-4 shrink-0 animate-spin text-sky-400" />
						<span className="truncate text-sky-200 text-xs">{step ?? "Preparing..."}</span>
					</div>
				)}
				{shownStatus === "sleeping" && (
					<div className="rounded-xl border border-violet-500/20 bg-violet-500/[0.07] px-3 py-2.5 text-violet-200 text-xs">
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
							className="h-11 flex-1 gap-2 rounded-xl bg-emerald-600 font-semibold text-white shadow-emerald-950/40 shadow-lg hover:bg-emerald-500"
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
							variant="outline"
							onClick={controls.sleep}
							className="h-11 flex-1 gap-2 rounded-xl border-zinc-800"
						>
							<Moon className="size-4" /> Sleep
						</Button>
					)}
					{(shownStatus === "running" || shownStatus === "sleeping") && (
						<Button
							variant="destructive"
							onClick={controls.stop}
							disabled={controls.isStopping}
							className="h-11 flex-1 gap-2 rounded-xl"
						>
							{controls.isStopping ? (
								<Loader2 className="size-4 animate-spin" />
							) : (
								<Square className="size-4 fill-current" />
							)}
							Stop
						</Button>
					)}
					{(shownStatus === "starting" || shownStatus === "stopping") && (
						<Button disabled className="h-11 flex-1 gap-2 rounded-xl">
							<Loader2 className="size-4 animate-spin" />
							{shownStatus === "starting" ? "Starting..." : "Stopping..."}
						</Button>
					)}
				</div>
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
			<div className="flex min-h-24 flex-col justify-end px-4 pb-4 font-mono text-[10.5px] text-zinc-400 leading-relaxed">
				{lines.length === 0 ? (
					<span className="text-zinc-600">No output yet</span>
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
