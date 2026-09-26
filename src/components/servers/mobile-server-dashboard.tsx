import {
	Check,
	Copy,
	Globe,
	Moon,
	Play,
	Plus,
	Power,
	QrCode,
	RefreshCw,
	Server,
	ShieldCheck,
	Square,
	Terminal,
	Zap,
} from "lucide-react"
import { memo, useCallback, useEffect, useState } from "react"
import type { PlayitTunnelStatus, ServerStatus } from "@/bindings"
import { MobileConsoleSheet } from "@/components/servers/mobile-console-sheet"
import NewServerDialog from "@/components/servers/new-server-dialog"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import {
	serverService,
	useRunningServers,
	useServerLogs,
	useServers,
} from "@/services/server-service"

/** Injected by MainActivity.kt; controls the Android foreground service */
interface IngotHostBridge {
	start(title: string, detail: string): void
	stop(): void
}

export const MobileServerDashboard = memo(() => {
	const { servers } = useServers()
	const { runningMap } = useRunningServers()

	const [selectedServerId, setSelectedServerId] = useState<string | null>(null)
	const [isConsoleOpen, setIsConsoleOpen] = useState(false)
	const [isNewServerOpen, setIsNewServerOpen] = useState(false)
	const [isQrOpen, setIsQrOpen] = useState(false)
	const [copied, setCopied] = useState(false)

	// Playit.gg tunnel state
	const [playitStatus, setPlayitStatus] = useState<PlayitTunnelStatus>({
		isRunning: false,
		status: "stopped",
		claimUrl: null,
		publicAddress: null,
		pingMs: null,
		message: null,
	})
	const [isTunnelLoading, setIsTunnelLoading] = useState(false)
	const [isStartRequested, setIsStartRequested] = useState(false)
	const [startError, setStartError] = useState<string | null>(null)

	// Default to first server if not selected
	useEffect(() => {
		if (!selectedServerId && servers.length > 0) {
			setSelectedServerId(servers[0].id)
		}
	}, [selectedServerId, servers])

	const currentServer = servers.find((s) => s.id === selectedServerId) || servers[0]
	const runningInfo = currentServer ? runningMap.get(currentServer.id) : undefined
	const reportedStatus: ServerStatus = runningInfo ? runningInfo.status : "stopped"
	// Show "starting" immediately on tap, before the backend's first status event arrives
	const currentStatus: ServerStatus =
		isStartRequested && reportedStatus === "stopped" ? "starting" : reportedStatus

	// Latest setup step ("[Ingot] ...") to show while the server is starting
	const { logs } = useServerLogs(currentServer?.id ?? null)
	const latestStep = [...logs]
		.reverse()
		.find((line) => line.startsWith("[Ingot]"))
		?.replace("[Ingot] ", "")

	// Fetch playit status periodically
	const refreshPlayit = useCallback(async () => {
		try {
			const status = await serverService.getPlayitStatus()
			if (status) {
				setPlayitStatus(status)
			}
		} catch (_e) {
			// ignore if playit not running
		}
	}, [])

	// Poll quickly while the tunnel is being set up so each step shows up promptly
	const isTunnelSettingUp = playitStatus.isRunning && playitStatus.status !== "connected"
	useEffect(() => {
		refreshPlayit()
		const interval = setInterval(refreshPlayit, isTunnelSettingUp ? 1000 : 5000)
		return () => clearInterval(interval)
	}, [refreshPlayit, isTunnelSettingUp])

	// Keep the Android foreground service up while anything is hosted. Without it the
	// app is "cached" as soon as it leaves the screen and Android cuts off its network.
	const activeServers = [...runningMap.values()].filter((s) => s.status !== "stopped")
	const isHosting = activeServers.length > 0 || playitStatus.isRunning || isStartRequested
	const hostTitle =
		activeServers.length > 0
			? `Hosting ${activeServers.length === 1 ? (servers.find((s) => s.id === activeServers[0].serverId)?.name ?? "server") : `${activeServers.length} servers`}`
			: "Ingot tunnel active"
	const hostDetail = playitStatus.publicAddress
		? `Public address: ${playitStatus.publicAddress}`
		: playitStatus.isRunning
			? "playit.gg tunnel connecting..."
			: "Local network only"
	useEffect(() => {
		const host = (window as Window & { IngotHost?: IngotHostBridge }).IngotHost
		if (!host) return
		if (isHosting) {
			host.start(hostTitle, hostDetail)
		} else {
			host.stop()
		}
	}, [isHosting, hostTitle, hostDetail])

	// Handlers for server actions
	const handleStartServer = async () => {
		if (!currentServer || isStartRequested) return
		setStartError(null)
		setIsStartRequested(true)
		try {
			await serverService.startServer(currentServer.id)
		} catch (e) {
			console.error("Failed to start server:", e)
			setStartError(String(e))
		} finally {
			setIsStartRequested(false)
		}
	}

	const handleSleepServer = async () => {
		if (!currentServer) return
		try {
			await serverService.putServerToSleep(currentServer.id)
		} catch (e) {
			console.error("Failed to put server to sleep:", e)
		}
	}

	const handleStopServer = async () => {
		if (!currentServer) return
		try {
			await serverService.stopServer(currentServer.id)
		} catch (e) {
			console.error("Failed to stop server:", e)
		}
	}

	const handleToggleTunnel = async () => {
		setIsTunnelLoading(true)
		try {
			if (playitStatus.isRunning) {
				await serverService.stopPlayitTunnel()
			} else {
				// Show progress immediately instead of waiting for the first status poll
				setPlayitStatus((prev) => ({
					...prev,
					isRunning: true,
					status: "starting",
					claimUrl: null,
					publicAddress: null,
					message: null,
				}))
				await serverService.startPlayitTunnel(currentServer?.playitSecretKey)
			}
			await refreshPlayit()
		} catch (e) {
			console.error("Failed to toggle Playit tunnel:", e)
		} finally {
			setIsTunnelLoading(false)
		}
	}

	const handleCopyAddress = (text: string) => {
		navigator.clipboard.writeText(text)
		setCopied(true)
		setTimeout(() => setCopied(false), 2000)
	}

	if (!currentServer) {
		return (
			<div className="flex h-screen flex-col items-center justify-center bg-zinc-950 p-6 text-center text-zinc-100">
				<Server className="mb-4 size-16 animate-bounce text-emerald-500" />
				<h2 className="font-bold text-xl">No Servers Created Yet</h2>
				<p className="mt-2 max-w-xs text-sm text-zinc-400">
					Create your first mobile Minecraft server to start hosting right from your phone.
				</p>
				<Button
					onClick={() => setIsNewServerOpen(true)}
					className="mt-6 bg-emerald-600 hover:bg-emerald-500"
				>
					<Plus className="mr-2 size-4" />
					Create Server
				</Button>
				<NewServerDialog open={isNewServerOpen} onOpenChange={setIsNewServerOpen} />
			</div>
		)
	}

	return (
		<div className="flex min-h-screen flex-col bg-zinc-950 pb-12 text-zinc-100">
			{/* Top Mobile Bar */}
			<header className="sticky top-0 z-30 flex items-center justify-between border-zinc-800/80 border-b bg-zinc-950/90 px-4 py-3 backdrop-blur-md">
				<div className="flex items-center gap-2">
					<div className="flex size-8 items-center justify-center rounded-lg border border-emerald-500/20 bg-emerald-500/10 text-emerald-400">
						<Server className="size-4" />
					</div>
					<div>
						<h1 className="font-bold text-sm tracking-tight">Ingot Host</h1>
						<p className="text-[10px] text-zinc-400">Android Dedicated Server</p>
					</div>
				</div>

				<div className="flex items-center gap-2">
					{/* Server selector if multiple exist */}
					{servers.length > 1 && (
						<select
							value={selectedServerId || currentServer.id}
							onChange={(e) => setSelectedServerId(e.target.value)}
							className="rounded-lg border border-zinc-800 bg-zinc-900 px-2 py-1 font-medium text-xs text-zinc-300 focus:outline-none"
						>
							{servers.map((s) => (
								<option key={s.id} value={s.id}>
									{s.name}
								</option>
							))}
						</select>
					)}

					<Button
						variant="outline"
						size="sm"
						className="h-8 border-zinc-800 bg-zinc-900 px-2 text-xs"
						onClick={() => setIsNewServerOpen(true)}
					>
						<Plus className="size-3.5" />
					</Button>
				</div>
			</header>

			{/* Main Scrollable Content */}
			<main className="flex-1 space-y-4 p-4">
				{/* Primary Server Status Card */}
				<section className="relative overflow-hidden rounded-2xl border border-zinc-800/90 bg-zinc-900/50 p-5 shadow-xl">
					<div className="flex items-start justify-between">
						<div>
							<div className="flex items-center gap-2">
								<span className="font-bold text-lg">{currentServer.name}</span>
								<span className="rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2 py-0.5 font-medium text-[10px] text-emerald-400">
									{currentServer.core.toUpperCase()}
								</span>
							</div>
							<p className="mt-0.5 text-xs text-zinc-400">
								MC {currentServer.gameVersion} • Port {currentServer.port}
							</p>
						</div>

						{/* Live Status Pill */}
						<div>
							{currentStatus === "running" && (
								<div className="flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/15 px-3 py-1 font-semibold text-emerald-400 text-xs">
									<span className="size-2 animate-pulse rounded-full bg-emerald-400" />
									Running
								</div>
							)}
							{currentStatus === "sleeping" && (
								<div className="flex items-center gap-1.5 rounded-full border border-amber-500/30 bg-amber-500/15 px-3 py-1 font-semibold text-amber-300 text-xs">
									<Moon className="size-3" />
									Sleeping
								</div>
							)}
							{currentStatus === "starting" && (
								<div className="flex items-center gap-1.5 rounded-full border border-blue-500/30 bg-blue-500/15 px-3 py-1 font-semibold text-blue-400 text-xs">
									<RefreshCw className="size-3 animate-spin" />
									Starting
								</div>
							)}
							{currentStatus === "stopped" && (
								<div className="flex items-center gap-1.5 rounded-full border border-zinc-700/50 bg-zinc-800 px-3 py-1 font-medium text-xs text-zinc-400">
									<Power className="size-3" />
									Stopped
								</div>
							)}
						</div>
					</div>

					{/* Sleeping Notice Banner */}
					{currentStatus === "sleeping" && (
						<div className="mt-4 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-amber-200 text-xs">
							<div className="flex items-center gap-2 font-semibold">
								<Moon className="size-4 shrink-0 text-amber-400" />
								Server is sleeping to save battery & RAM
							</div>
							<p className="mt-1 text-[11px] text-amber-300/80 leading-relaxed">
								The proxy is holding port {currentServer.port}. It will automatically cold-boot the
								server when a player connects!
							</p>
						</div>
					)}

					{/* Start Error */}
					{startError && currentStatus === "stopped" && (
						<div className="mt-4 whitespace-pre-wrap break-words rounded-xl border border-red-500/20 bg-red-500/10 p-3 text-red-300 text-xs">
							<p className="font-semibold">Failed to start</p>
							<p className="mt-1 text-[11px] text-red-300/90">{startError}</p>
						</div>
					)}

					{/* Action Buttons Row */}
					<div className="mt-5 grid grid-cols-2 gap-2">
						{currentStatus === "starting" ? (
							<div className="col-span-2 flex items-center gap-3 rounded-xl border border-blue-500/20 bg-blue-500/10 p-3 text-xs">
								<RefreshCw className="size-4 shrink-0 animate-spin text-blue-400" />
								<div className="min-w-0">
									<p className="font-semibold text-blue-300">Starting server...</p>
									<p className="mt-0.5 truncate text-[11px] text-zinc-400">
										{latestStep ?? "Preparing..."}
									</p>
								</div>
							</div>
						) : currentStatus === "stopped" ? (
							<Button
								onClick={handleStartServer}
								className="col-span-2 h-11 bg-emerald-600 font-semibold text-sm shadow-emerald-900/20 shadow-lg hover:bg-emerald-500"
							>
								<Play className="mr-2 size-4 fill-current" />
								Start Server
							</Button>
						) : currentStatus === "sleeping" ? (
							<>
								<Button
									onClick={handleStartServer}
									className="h-10 bg-emerald-600 font-semibold text-xs hover:bg-emerald-500"
								>
									<Zap className="mr-1.5 size-3.5" />
									Wake Up
								</Button>
								<Button
									variant="outline"
									onClick={handleStopServer}
									className="h-10 border-zinc-800 bg-zinc-800/60 text-xs text-zinc-300 hover:bg-zinc-800"
								>
									<Square className="mr-1.5 size-3.5" />
									Stop Server
								</Button>
							</>
						) : (
							<>
								<Button
									onClick={handleSleepServer}
									variant="outline"
									className="h-10 border-amber-500/30 bg-amber-500/10 font-medium text-amber-300 text-xs hover:bg-amber-500/20"
								>
									<Moon className="mr-1.5 size-3.5" />
									Put to Sleep
								</Button>
								<Button
									onClick={handleStopServer}
									variant="outline"
									className="h-10 border-rose-500/30 bg-rose-500/10 font-medium text-rose-300 text-xs hover:bg-rose-500/20"
								>
									<Square className="mr-1.5 size-3.5" />
									Stop Server
								</Button>
							</>
						)}
					</div>
				</section>

				{/* Playit.gg Reverse Tunnel Section */}
				<section className="rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4">
					<div className="flex items-center justify-between">
						<div className="flex items-center gap-2.5">
							<div className="flex size-8 items-center justify-center rounded-lg border border-sky-500/20 bg-sky-500/10 text-sky-400">
								<Globe className="size-4" />
							</div>
							<div>
								<h2 className="font-semibold text-sm">Public Tunnel (playit.gg)</h2>
								<p className="text-[11px] text-zinc-400">
									Share with friends without port forwarding
								</p>
							</div>
						</div>

						<Button
							size="sm"
							variant={playitStatus.isRunning ? "destructive" : "default"}
							disabled={isTunnelLoading}
							onClick={handleToggleTunnel}
							className="h-8 px-3 text-xs"
						>
							{isTunnelLoading ? (
								<RefreshCw className="size-3 animate-spin" />
							) : playitStatus.isRunning ? (
								"Disconnect"
							) : (
								"Connect"
							)}
						</Button>
					</div>

					{/* Tunnel Status Details */}
					{playitStatus.status === "error" && playitStatus.message && (
						<div className="mt-3.5 rounded-xl border border-red-500/20 bg-red-500/10 p-3 text-red-300 text-xs">
							{playitStatus.message}
						</div>
					)}
					{playitStatus.isRunning && (
						<div className="mt-3.5 space-y-2 border-zinc-800/60 border-t pt-3">
							{playitStatus.status === "claiming" && playitStatus.claimUrl ? (
								<div className="rounded-xl border border-sky-500/20 bg-sky-500/10 p-3 text-xs">
									<p className="font-semibold text-sky-300">Tunnel Setup Required</p>
									<p className="mt-0.5 text-[11px] text-zinc-300">
										Click below to link this server to your playit.gg account:
									</p>
									<a
										href={playitStatus.claimUrl}
										target="_blank"
										rel="noreferrer"
										className="mt-2 inline-flex items-center rounded-lg bg-sky-600 px-3 py-1.5 font-semibold text-white text-xs hover:bg-sky-500"
									>
										Open Claim Link ↗
									</a>
								</div>
							) : playitStatus.publicAddress ? (
								<div className="rounded-xl border border-zinc-800 bg-zinc-950 p-3">
									<span className="font-medium text-[10px] text-zinc-400 uppercase tracking-wider">
										Public Address to Share:
									</span>
									<div className="mt-1 flex items-center justify-between">
										<code className="select-all font-bold font-mono text-emerald-400 text-xs">
											{playitStatus.publicAddress}
										</code>
										<div className="flex items-center gap-1">
											<Button
												size="sm"
												variant="ghost"
												onClick={() =>
													playitStatus.publicAddress &&
													handleCopyAddress(playitStatus.publicAddress)
												}
												className="size-7 p-0 text-zinc-400 hover:text-zinc-100"
											>
												{copied ? (
													<Check className="size-3.5 text-emerald-400" />
												) : (
													<Copy className="size-3.5" />
												)}
											</Button>
											<Button
												size="sm"
												variant="ghost"
												onClick={() => setIsQrOpen(true)}
												className="size-7 p-0 text-zinc-400 hover:text-zinc-100"
											>
												<QrCode className="size-3.5" />
											</Button>
										</div>
									</div>
								</div>
							) : playitStatus.status === "no_tunnel" && playitStatus.message ? (
								<div className="rounded-xl border border-amber-500/20 bg-amber-500/10 p-3 text-amber-200 text-xs">
									{playitStatus.message}
									<a
										href="https://playit.gg/account/tunnels"
										target="_blank"
										rel="noreferrer"
										className="mt-2 block font-semibold text-amber-300 underline"
									>
										Open playit.gg tunnels ↗
									</a>
								</div>
							) : (
								<div className="flex items-center gap-2 text-xs text-zinc-400">
									<RefreshCw className="size-3 animate-spin text-sky-400" />
									{playitStatus.message ??
										(playitStatus.status === "starting"
											? "Starting playit agent..."
											: "Connecting to playit.gg...")}
								</div>
							)}
						</div>
					)}
				</section>

				{/* Device Power & Background Persistence */}
				<section className="rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4">
					<div className="flex items-center gap-2.5">
						<div className="flex size-8 items-center justify-center rounded-lg border border-indigo-500/20 bg-indigo-500/10 text-indigo-400">
							<ShieldCheck className="size-4" />
						</div>
						<div>
							<h2 className="font-semibold text-sm">Background Protection</h2>
							<p className="text-[11px] text-zinc-400">Android Foreground Service & WakeLocks</p>
						</div>
					</div>

					<div className="mt-3 grid grid-cols-2 gap-2 text-xs">
						<div className="rounded-xl border border-zinc-800/60 bg-zinc-950 p-2.5">
							<span className="text-[10px] text-zinc-400">CPU Partial WakeLock</span>
							<p className="mt-0.5 font-semibold text-emerald-400">Active</p>
						</div>
						<div className="rounded-xl border border-zinc-800/60 bg-zinc-950 p-2.5">
							<span className="text-[10px] text-zinc-400">Wi-Fi High-Perf Lock</span>
							<p className="mt-0.5 font-semibold text-emerald-400">Active</p>
						</div>
					</div>

					<div className="mt-2.5 flex items-center justify-between px-1 text-[11px] text-zinc-400">
						<span>Auto-Sleep Timeout</span>
						<span className="font-medium text-zinc-200">10 Minutes (Idle)</span>
					</div>
				</section>

				{/* Quick Terminal Console Button */}
				<Button
					onClick={() => setIsConsoleOpen(true)}
					className="flex h-12 w-full items-center justify-between rounded-xl border border-zinc-800 bg-zinc-900 px-4 text-zinc-200 hover:bg-zinc-850"
				>
					<div className="flex items-center gap-2">
						<Terminal className="size-4 text-emerald-400" />
						<span className="font-semibold text-xs">Live Server Console</span>
					</div>
					<span className="text-[11px] text-zinc-400">View Logs & Commands →</span>
				</Button>
			</main>

			{/* QR Code Dialog */}
			<Dialog open={isQrOpen} onOpenChange={setIsQrOpen}>
				<DialogContent className="max-w-xs border-zinc-800 bg-zinc-950 p-6 text-center">
					<DialogHeader>
						<DialogTitle className="font-bold text-sm">Server Public Link</DialogTitle>
					</DialogHeader>
					<div className="mt-3 flex flex-col items-center">
						<div className="rounded-xl bg-white p-3">
							<img
								src={`https://api.qrserver.com/v1/create-qr-code/?size=180x180&data=${encodeURIComponent(
									playitStatus.publicAddress || "",
								)}`}
								alt="Server Address QR"
								className="size-44"
							/>
						</div>
						<code className="mt-3 font-bold font-mono text-emerald-400 text-xs">
							{playitStatus.publicAddress}
						</code>
					</div>
				</DialogContent>
			</Dialog>

			{/* Terminal Console Sheet Modal */}
			<MobileConsoleSheet
				serverId={currentServer.id}
				serverName={currentServer.name}
				isOpen={isConsoleOpen}
				onClose={() => setIsConsoleOpen(false)}
			/>

			{/* New Server Dialog */}
			<NewServerDialog open={isNewServerOpen} onOpenChange={setIsNewServerOpen} />
		</div>
	)
})

MobileServerDashboard.displayName = "MobileServerDashboard"
export default MobileServerDashboard
