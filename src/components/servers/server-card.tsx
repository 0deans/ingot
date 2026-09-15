import {
	Check,
	Copy,
	Cpu,
	FolderOpen,
	Globe,
	Loader2,
	Play,
	Settings,
	Square,
	Terminal,
	Trash2,
} from "lucide-react"
import { useEffect, useState } from "react"
import type { RunningServerSummary, ServerConfig, ServerCoreType } from "@/bindings"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

export interface ServerCardProps {
	server: ServerConfig
	runningInfo?: RunningServerSummary
	onStart: (serverId: string) => void
	onStop: (serverId: string) => void
	onOpenConsole: (server: ServerConfig) => void
	onOpenSettings: (server: ServerConfig) => void
	onOpenFolder: (serverId: string) => void
	onDelete: (server: ServerConfig) => void
}

function getCoreBadgeStyle(core: ServerCoreType) {
	switch (core) {
		case "paper":
			return "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
		case "purpur":
			return "border-purple-500/30 bg-purple-500/10 text-purple-400"
		case "fabric":
			return "border-sky-500/30 bg-sky-500/10 text-sky-400"
		case "folia":
			return "border-cyan-500/30 bg-cyan-500/10 text-cyan-400"
		case "vanilla":
			return "border-amber-500/30 bg-amber-500/10 text-amber-400"
		default:
			return "border-zinc-800 bg-zinc-900 text-zinc-400"
	}
}

function formatUptime(seconds: number): string {
	const h = Math.floor(seconds / 3600)
	const m = Math.floor((seconds % 3600) / 60)
	const s = seconds % 60
	if (h > 0) {
		return `${h}h ${m}m ${s}s`
	}
	if (m > 0) {
		return `${m}m ${s}s`
	}
	return `${s}s`
}

export default function ServerCard({
	server,
	runningInfo,
	onStart,
	onStop,
	onOpenConsole,
	onOpenSettings,
	onOpenFolder,
	onDelete,
}: ServerCardProps) {
	const isRunning = Boolean(runningInfo && runningInfo.status === "running")
	const isStarting = Boolean(runningInfo && runningInfo.status === "starting")
	const isStopping = Boolean(runningInfo && runningInfo.status === "stopping")

	const [copied, setCopied] = useState(false)
	const [uptime, setUptime] = useState(runningInfo?.uptimeSeconds || 0)

	// Live ticker for uptime
	useEffect(() => {
		if (!isRunning) return
		setUptime(runningInfo?.uptimeSeconds || 0)
		const timer = setInterval(() => {
			setUptime((u) => u + 1)
		}, 1000)
		return () => clearInterval(timer)
	}, [isRunning, runningInfo?.uptimeSeconds])

	const handleCopyAddress = (e: React.MouseEvent) => {
		e.stopPropagation()
		navigator.clipboard.writeText(`localhost:${server.port}`)
		setCopied(true)
		setTimeout(() => setCopied(false), 2000)
	}

	return (
		<div
			className={cn(
				"group relative flex flex-col justify-between overflow-hidden rounded-2xl border p-4 transition-all duration-200",
				isRunning
					? "border-emerald-500/40 bg-zinc-900/70 shadow-emerald-950/20 shadow-lg ring-1 ring-emerald-500/20"
					: "border-border/50 bg-zinc-900/40 hover:border-border/80 hover:bg-zinc-900/70",
			)}
		>
			{/* Top Bar: Core Badge + Status */}
			<div className="flex items-start justify-between gap-2">
				<div className="flex items-center gap-2">
					<span
						className={cn(
							"rounded-lg border px-2 py-0.5 font-semibold text-[10px] uppercase tracking-wider",
							getCoreBadgeStyle(server.core),
						)}
					>
						{server.core}
					</span>
					<span className="font-mono text-[11px] text-muted-foreground">{server.gameVersion}</span>
				</div>

				{/* Live Status Indicator */}
				{isRunning ? (
					<div className="flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[11px] text-emerald-400">
						<span className="size-1.5 animate-ping rounded-full bg-emerald-400" />
						<span className="font-medium font-mono">{formatUptime(uptime)}</span>
					</div>
				) : isStarting ? (
					<div className="flex items-center gap-1.5 rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[11px] text-amber-400">
						<Loader2 className="size-3 animate-spin" />
						<span>Starting...</span>
					</div>
				) : isStopping ? (
					<div className="flex items-center gap-1.5 rounded-full border border-rose-500/30 bg-rose-500/10 px-2 py-0.5 text-[11px] text-rose-400">
						<Loader2 className="size-3 animate-spin" />
						<span>Stopping...</span>
					</div>
				) : (
					<span className="rounded-full border border-border/40 bg-zinc-900/80 px-2 py-0.5 text-[10px] text-zinc-500">
						Offline
					</span>
				)}
			</div>

			{/* Center Info: Name & Specs */}
			<div className="my-3 flex flex-col gap-1.5">
				<h3 className="truncate font-semibold text-base text-foreground transition-colors group-hover:text-emerald-300">
					{server.name}
				</h3>

				<div className="flex flex-wrap items-center gap-3 text-muted-foreground text-xs">
					{/* Address / Port with Click-to-copy */}
					<button
						type="button"
						onClick={handleCopyAddress}
						className="flex items-center gap-1 font-mono text-[11px] text-zinc-400 transition-colors hover:text-emerald-400"
						title="Click to copy server address"
					>
						<Globe className="size-3 text-emerald-500/70" />
						<span>localhost:{server.port}</span>
						{copied ? (
							<Check className="size-3 text-emerald-400" />
						) : (
							<Copy className="size-2.5 opacity-60" />
						)}
					</button>

					{/* RAM */}
					<div className="flex items-center gap-1 font-mono text-[11px] text-zinc-400">
						<Cpu className="size-3 text-emerald-500/70" />
						<span>{(server.memoryMaxMb / 1024).toFixed(0)} GB RAM</span>
					</div>
				</div>
			</div>

			{/* Bottom Action Controls */}
			<div className="flex items-center justify-between border-border/30 border-t pt-3">
				{/* Primary Run / Console Action */}
				<div className="flex items-center gap-2">
					{isRunning ? (
						<>
							<Button
								size="sm"
								onClick={() => onOpenConsole(server)}
								className="h-8 gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
							>
								<Terminal className="size-3.5" />
								<span>Console</span>
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => onStop(server.id)}
								className="h-8 gap-1 border-rose-500/30 text-rose-400 text-xs hover:bg-rose-500/10 hover:text-rose-300"
								title="Stop server"
							>
								<Square className="size-3 fill-current" />
								<span>Stop</span>
							</Button>
						</>
					) : (
						<Button
							size="sm"
							onClick={() => onStart(server.id)}
							disabled={isStarting || isStopping}
							className="h-8 gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
						>
							{isStarting ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<Play className="size-3.5 fill-current" />
							)}
							<span>Start</span>
						</Button>
					)}
				</div>

				{/* Secondary Management Buttons */}
				<div className="flex items-center gap-1">
					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onOpenConsole(server)}
						className="size-8 text-muted-foreground hover:text-foreground"
						title="Open Console"
					>
						<Terminal className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onOpenSettings(server)}
						className="size-8 text-muted-foreground hover:text-foreground"
						title="Server Settings"
					>
						<Settings className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onOpenFolder(server.id)}
						className="size-8 text-muted-foreground hover:text-foreground"
						title="Open server folder"
					>
						<FolderOpen className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onDelete(server)}
						disabled={isRunning}
						className="size-8 text-muted-foreground hover:text-destructive"
						title="Delete Server"
					>
						<Trash2 className="size-3.5" />
					</Button>
				</div>
			</div>
		</div>
	)
}
