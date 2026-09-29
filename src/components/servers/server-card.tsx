import {
	Check,
	Copy,
	Cpu,
	FolderOpen,
	Gamepad2,
	Globe,
	Play,
	Server,
	Settings,
	Signal,
	Square,
	Terminal,
	Trash2,
	Users,
} from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type {
	RunningServerSummary,
	ServerConfig,
	ServerCoreType,
	ServerPingResponse,
} from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { formatDuration, formatMegabytes } from "@/lib/format"
import { cn } from "@/lib/utils"
import { serverService } from "@/services/server-service"

export interface ServerCardProps {
	server: ServerConfig
	runningInfo?: RunningServerSummary
	onStart: (serverId: string) => void
	onStop: (serverId: string) => void
	onOpenConsole: (server: ServerConfig) => void
	onOpenSettings: (server: ServerConfig) => void
	onOpenFolder: (serverId: string) => void
	onDelete: (server: ServerConfig) => void
	/** Opens the server page */
	onOpen?: (server: ServerConfig) => void
	onJoinServer?: (server: ServerConfig) => void
}

function getCoreBadgeStyle(core: ServerCoreType) {
	switch (core) {
		case "paper":
			return "border-emerald-500/30 bg-emerald-500/10 text-emerald-400"
		case "purpur":
			return "border-purple-500/30 bg-purple-500/10 text-purple-400"
		case "fabric":
			return "border-sky-500/30 bg-sky-500/10 text-sky-400"
		case "neoforge":
			return "border-orange-500/30 bg-orange-500/10 text-orange-400"
		case "forge":
			return "border-blue-500/30 bg-blue-500/10 text-blue-400"
		case "quilt":
			return "border-violet-500/30 bg-violet-500/10 text-violet-400"
		case "folia":
			return "border-cyan-500/30 bg-cyan-500/10 text-cyan-400"
		case "vanilla":
			return "border-amber-500/30 bg-amber-500/10 text-amber-400"
		default:
			return "border-zinc-800 bg-zinc-900 text-zinc-400"
	}
}

function formatUptime(seconds: number): string {
	return formatDuration(seconds)
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
	onJoinServer,
	onOpen,
}: ServerCardProps) {
	const { t } = useTranslation()
	const isRunning = Boolean(runningInfo && runningInfo.status === "running")
	const isStarting = Boolean(runningInfo && runningInfo.status === "starting")
	const isStopping = Boolean(runningInfo && runningInfo.status === "stopping")

	const [copied, setCopied] = useState(false)
	const [uptime, setUptime] = useState(runningInfo?.uptimeSeconds || 0)
	const [pingInfo, setPingInfo] = useState<ServerPingResponse | null>(null)
	const [serverIcon, setServerIcon] = useState<string | null>(server.icon || null)

	// Live ticker for uptime
	useEffect(() => {
		if (!isRunning) return
		setUptime(runningInfo?.uptimeSeconds || 0)
		const timer = setInterval(() => {
			setUptime((u) => u + 1)
		}, 1000)
		return () => clearInterval(timer)
	}, [isRunning, runningInfo?.uptimeSeconds])

	// Fetch custom icon if not in server config
	useEffect(() => {
		let cancelled = false
		serverService
			.getServerIcon(server.id)
			.then((icon) => {
				if (!cancelled && icon) setServerIcon(icon)
			})
			.catch(() => {})
		return () => {
			cancelled = true
		}
	}, [server.id])

	// Query Server List Ping (SLP) when server is running
	useEffect(() => {
		if (!isRunning) {
			setPingInfo(null)
			return
		}
		let cancelled = false
		const fetchSlp = async () => {
			try {
				const info = await serverService.pingServer(server.port)
				if (!cancelled) {
					setPingInfo(info)
					if (info.favicon) setServerIcon(info.favicon)
				}
			} catch {
				// Server may still be finishing startup
			}
		}
		fetchSlp()
		const timer = setInterval(fetchSlp, 5000)
		return () => {
			cancelled = true
			clearInterval(timer)
		}
	}, [isRunning, server.port])

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
					? "border-primary/40 bg-card/70 shadow-lg shadow-primary/20 ring-1 ring-primary/20"
					: "border-border/50 bg-card/40 hover:border-border/80 hover:bg-card/70",
			)}
		>
			{/* Top Bar: Core Badge + Status */}
			<div className="flex items-start justify-between gap-2">
				<div className="flex items-center gap-2">
					<Badge
						variant="outline"
						className={cn("uppercase tracking-wider", getCoreBadgeStyle(server.core))}
					>
						{server.core}
					</Badge>
					<span className="font-mono text-[11px] text-muted-foreground">{server.gameVersion}</span>
				</div>

				{/* Live Status Indicator */}
				{isRunning ? (
					<Badge variant="outline" className="border-primary/30 font-mono text-primary">
						<span className="size-1.5 animate-ping rounded-full bg-primary" />
						{formatUptime(uptime)}
					</Badge>
				) : isStarting ? (
					<Badge variant="outline" className="border-warning/30 text-warning">
						<Spinner />
						{t("overview.starting")}
					</Badge>
				) : isStopping ? (
					<Badge variant="outline" className="border-destructive/30 text-destructive">
						<Spinner />
						{t("overview.stopping")}
					</Badge>
				) : (
					<Badge variant="outline">{t("serverCard.offline")}</Badge>
				)}
			</div>

			{/* Center Info: Icon + Name & Specs */}
			<div className="my-3 flex items-start gap-3">
				{/* Server Icon / Favicon */}
				<div className="relative flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-border/50 bg-muted/80 shadow-inner">
					{serverIcon ? (
						<img
							src={serverIcon}
							alt={server.name}
							className="size-full object-cover [image-rendering:pixelated]"
						/>
					) : (
						<Server className="size-5 text-muted-foreground" />
					)}
				</div>

				<div className="flex min-w-0 flex-1 flex-col gap-1">
					<button
						type="button"
						onClick={() => onOpen?.(server)}
						className="truncate text-left font-semibold text-base text-foreground transition-colors hover:text-primary"
						title={t("serverCard.openPage")}
					>
						{server.name}
					</button>

					<div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-muted-foreground text-xs">
						{/* Address / Port with Click-to-copy */}
						<button
							type="button"
							onClick={handleCopyAddress}
							className="flex cursor-pointer items-center gap-1 font-mono text-[11px] text-muted-foreground transition-colors hover:text-primary"
							title={t("serverCard.copyAddress")}
						>
							<Globe className="size-3 text-primary/70" />
							<span>localhost:{server.port}</span>
							{copied ? (
								<Check className="size-3 text-primary" />
							) : (
								<Copy className="size-2.5 opacity-60" />
							)}
						</button>

						{/* RAM */}
						<div className="flex items-center gap-1 font-mono text-[11px] text-muted-foreground">
							<Cpu className="size-3 text-primary/70" />
							<span>{formatMegabytes(server.memoryMaxMb, 0)}</span>
						</div>

						{/* Live SLP Player count badge (only when running) */}
						{isRunning && (
							<div className="flex items-center gap-1 font-mono text-[11px] text-primary">
								<Users className="size-3 text-primary" />
								<span>
									{pingInfo
										? `${pingInfo.players.online}/${pingInfo.players.max}`
										: t("serverCard.noneOnline")}
								</span>
							</div>
						)}

						{/* Live SLP Ping latency badge */}
						{isRunning && pingInfo && (
							<div className="flex items-center gap-1 font-mono text-[11px] text-muted-foreground">
								<Signal className="size-2.5 text-primary" />
								<span>{pingInfo.pingMs}ms</span>
							</div>
						)}
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
								onClick={() => onJoinServer?.(server)}
								className="h-8 gap-1.5 font-medium text-xs"
								title={t("serverCard.joinTitle")}
							>
								<Gamepad2 className="size-3.5" />
								<span>{t("serverCard.join")}</span>
							</Button>
							<Button
								size="sm"
								variant="secondary"
								onClick={() => onOpenConsole(server)}
								className="h-8 gap-1.5 font-medium text-xs"
							>
								<Terminal className="size-3.5" />
								<span>{t("serverTabs.console")}</span>
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => onStop(server.id)}
								className="h-8 gap-1 border-destructive/30 text-destructive text-xs hover:bg-destructive/10 hover:text-destructive"
								title={t("servers.stopServer")}
							>
								<Square className="size-3 fill-current" />
								<span>{t("common.stop")}</span>
							</Button>
						</>
					) : (
						<Button
							size="sm"
							onClick={() => onStart(server.id)}
							disabled={isStarting || isStopping}
							className="h-8 gap-1.5 font-medium text-xs"
						>
							{isStarting ? (
								<Spinner className="size-3.5" />
							) : (
								<Play className="size-3.5 fill-current" />
							)}
							<span>{t("serverCard.start")}</span>
						</Button>
					)}
				</div>

				{/* Secondary Management Buttons */}
				<div className="flex items-center gap-1">
					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onOpenConsole(server)}
						className="size-8 text-muted-foreground"
						title={t("servers.openConsole")}
					>
						<Terminal className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onOpenSettings(server)}
						className="size-8 text-muted-foreground"
						title={t("mobileServers.settings")}
					>
						<Settings className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onOpenFolder(server.id)}
						className="size-8 text-muted-foreground"
						title={t("serverCard.openFolder")}
					>
						<FolderOpen className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon-xs"
						onClick={() => onDelete(server)}
						disabled={isRunning}
						className="size-8 text-muted-foreground hover:text-destructive"
						title={t("deleteServer.title")}
					>
						<Trash2 className="size-3.5" />
					</Button>
				</div>
			</div>
		</div>
	)
}
