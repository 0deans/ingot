import {
	AlertCircle,
	Check,
	Cpu,
	FolderOpen,
	Globe,
	Loader2,
	Plus,
	Save,
	Server,
	Settings,
	Shield,
	Signal,
	Swords,
	Trash2,
	Upload,
	UserCheck,
	Users,
} from "lucide-react"
import { useCallback, useEffect, useRef, useState } from "react"
import type {
	ServerConfig,
	ServerPingResponse,
	ServerPlayerSample,
	ServerProperties,
	WhitelistEntry,
} from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import Slider from "@/components/ui/slider"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { cn } from "@/lib/utils"
import { serverService } from "@/services/server-service"

export interface ServerSettingsDialogProps {
	server: ServerConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onSaved?: () => void
	isRunning?: boolean
}

// ─── MOTD color/format codes ─────────────────────────────────────────────────
const MC_COLORS: Record<string, string> = {
	"0": "#000000",
	"1": "#0000AA",
	"2": "#00AA00",
	"3": "#00AAAA",
	"4": "#AA0000",
	"5": "#AA00AA",
	"6": "#FFAA00",
	"7": "#AAAAAA",
	"8": "#555555",
	"9": "#5555FF",
	a: "#55FF55",
	b: "#55FFFF",
	c: "#FF5555",
	d: "#FF55FF",
	e: "#FFFF55",
	f: "#FFFFFF",
}

interface MotdSpan {
	key: string
	text: string
	color?: string
	bold?: boolean
	italic?: boolean
	underline?: boolean
	strikethrough?: boolean
}

interface MotdLine {
	key: string
	spans: MotdSpan[]
}

function parseMotd(motd: string): MotdSpan[] {
	const spans: MotdSpan[] = []
	let counter = 0
	let current: MotdSpan = { key: `span-${counter++}`, text: "" }
	let i = 0
	while (i < motd.length) {
		if ((motd[i] === "§" || motd[i] === "&") && i + 1 < motd.length) {
			if (current.text) {
				spans.push({ ...current })
			}
			const code = motd[i + 1].toLowerCase()
			if (code in MC_COLORS) {
				current = { key: `span-${counter++}`, text: "", color: MC_COLORS[code] }
			} else if (code === "l") {
				current = { ...current, key: `span-${counter++}`, text: "", bold: true }
			} else if (code === "o") {
				current = { ...current, key: `span-${counter++}`, text: "", italic: true }
			} else if (code === "n") {
				current = { ...current, key: `span-${counter++}`, text: "", underline: true }
			} else if (code === "m") {
				current = { ...current, key: `span-${counter++}`, text: "", strikethrough: true }
			} else if (code === "r") {
				current = { key: `span-${counter++}`, text: "" }
			}
			i += 2
		} else if (motd[i] === "\\n") {
			if (current.text) spans.push({ ...current })
			spans.push({ key: `span-${counter++}`, text: "\n" })
			current = { ...current, key: `span-${counter++}`, text: "" }
			i++
		} else {
			current.text += motd[i]
			i++
		}
	}
	if (current.text) spans.push({ ...current })
	return spans
}

function MotdPreview({
	motd,
	icon,
	serverName,
	maxPlayers = 20,
	onlinePlayers = 0,
	pingMs,
	isRunning = false,
}: {
	motd: string
	icon?: string | null
	serverName?: string
	maxPlayers?: number
	onlinePlayers?: number
	pingMs?: number
	isRunning?: boolean
}) {
	const spans = parseMotd(motd)
	// Split into lines (max 2 lines in Minecraft server list)
	const lines: MotdLine[] = [{ key: "line-0", spans: [] }]
	for (const span of spans) {
		if (span.text === "\n") {
			lines.push({ key: `line-${lines.length}`, spans: [] })
		} else {
			lines[lines.length - 1].spans.push(span)
		}
	}

	return (
		<div className="w-full min-w-0 max-w-full overflow-hidden rounded-lg border border-zinc-700 bg-[#1a1a2e] p-3 font-minecraft shadow-inner">
			{/* Fake server list row */}
			<div className="flex items-start gap-3 min-w-0 overflow-hidden">
				{/* Server icon */}
				<div className="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded border border-zinc-600 bg-zinc-800 text-2xl">
					{icon ? (
						<img
							src={icon}
							alt="Server Icon"
							className="size-full object-cover [image-rendering:pixelated]"
						/>
					) : (
						"🌿"
					)}
				</div>

				<div className="flex min-w-0 flex-1 flex-col gap-1 overflow-hidden">
					{/* Server Name + Ping & Player count header */}
					<div className="flex items-center justify-between gap-2 min-w-0 overflow-hidden">
						<span className="truncate font-semibold text-white text-xs">
							{serverName || "A Minecraft Server"}
						</span>
						<div className="flex shrink-0 items-center gap-2 text-[10px] text-zinc-400">
							<span className="flex items-center gap-1 font-mono">
								<Signal
									className={cn(
										"size-2.5",
										isRunning ? "text-emerald-400" : "text-zinc-500",
									)}
								/>
								<span>
									{isRunning && pingMs != null
										? `${pingMs}ms`
										: isRunning
											? "<1ms"
											: "Offline"}
								</span>
							</span>
							<span className="font-mono text-zinc-300">
								{onlinePlayers}/{maxPlayers}
							</span>
						</div>
					</div>

					{/* MOTD lines - max 2 lines with strict wrapping so text never overflows */}
					<div className="flex flex-col gap-0.5 overflow-hidden font-mono text-[11px] leading-snug break-words break-all [overflow-wrap:anywhere]">
						{lines.slice(0, 2).map((line) => (
							<div
								key={line.key}
								className={cn(
									"break-words break-all overflow-hidden [overflow-wrap:anywhere]",
									line.spans.length === 0 && "h-[1em]",
								)}
							>
								{line.spans.map((span) => (
									<span
										key={span.key}
										style={{
											color: span.color || "#AAAAAA",
											fontWeight: span.bold ? "bold" : undefined,
											fontStyle: span.italic ? "italic" : undefined,
											textDecoration:
												[span.underline && "underline", span.strikethrough && "line-through"]
													.filter(Boolean)
													.join(" ") || undefined,
										}}
									>
										{span.text}
									</span>
								))}
							</div>
						))}
						{lines.length === 1 && lines[0].spans.length === 0 && (
							<span className="text-[11px] text-zinc-500 italic">Empty MOTD</span>
						)}
					</div>
				</div>
			</div>
		</div>
	)
}

// ─── Players & Whitelist Tab ──────────────────────────────────────────────────
function PlayersTab({
	server,
	isRunning,
	whitelistEnabled,
	onToggleWhitelist,
	pingInfo,
	isLoadingPing,
}: {
	server: ServerConfig | null
	isRunning: boolean
	whitelistEnabled: boolean
	onToggleWhitelist?: (enabled: boolean) => void
	pingInfo: ServerPingResponse | null
	isLoadingPing: boolean
}) {
	const [activeSubTab, setActiveSubTab] = useState<"online" | "whitelist">("online")

	// Whitelist state
	const [whitelist, setWhitelist] = useState<WhitelistEntry[]>([])
	const [isLoadingWhitelist, setIsLoadingWhitelist] = useState(false)
	const [newUsername, setNewUsername] = useState("")
	const [isAddingUser, setIsAddingUser] = useState(false)
	const [whitelistError, setWhitelistError] = useState<string | null>(null)

	const serverId = server?.id

	const loadWhitelist = useCallback(async () => {
		if (!serverId) return
		setIsLoadingWhitelist(true)
		setWhitelistError(null)
		try {
			const list = await serverService.getServerWhitelist(serverId)
			setWhitelist(list)
		} catch (err) {
			console.error("Failed to load whitelist:", err)
			setWhitelistError("Failed to load whitelist.json")
		} finally {
			setIsLoadingWhitelist(false)
		}
	}, [serverId])

	useEffect(() => {
		if (serverId) {
			loadWhitelist()
		}
	}, [serverId, loadWhitelist])

	const handleAddWhitelist = async (e?: React.FormEvent) => {
		if (e) e.preventDefault()
		const trimmed = newUsername.trim()
		if (!server || !trimmed) return

		if (whitelist.some((w) => w.name.toLowerCase() === trimmed.toLowerCase())) {
			setWhitelistError(`"${trimmed}" is already on the whitelist.`)
			return
		}

		setIsAddingUser(true)
		setWhitelistError(null)
		try {
			await serverService.addToServerWhitelist(server.id, trimmed)
			setNewUsername("")
			await loadWhitelist()
		} catch (err: unknown) {
			setWhitelistError(err instanceof Error ? err.message : "Failed to add user to whitelist")
		} finally {
			setIsAddingUser(false)
		}
	}

	const handleRemoveWhitelist = async (username: string) => {
		if (!server) return
		try {
			await serverService.removeFromServerWhitelist(server.id, username)
			await loadWhitelist()
		} catch (err: unknown) {
			setWhitelistError(
				err instanceof Error ? err.message : "Failed to remove user from whitelist",
			)
		}
	}

	const onlineSample: ServerPlayerSample[] = pingInfo?.players.sample || []

	return (
		<div className="flex flex-col gap-4">
			{/* Sub-tab switcher */}
			<div className="flex items-center justify-between border-border/30 border-b pb-2">
				<div className="flex items-center gap-1.5 rounded-lg border border-border/40 bg-zinc-900/50 p-0.5">
					<button
						type="button"
						onClick={() => setActiveSubTab("online")}
						className={cn(
							"flex items-center gap-1.5 rounded-md px-3 py-1 font-medium text-xs transition-colors",
							activeSubTab === "online"
								? "bg-zinc-800 text-foreground shadow-xs"
								: "text-muted-foreground hover:text-foreground",
						)}
					>
						<Users className="size-3.5" />
						<span>Online ({isRunning ? pingInfo?.players.online || 0 : 0})</span>
					</button>
					<button
						type="button"
						onClick={() => setActiveSubTab("whitelist")}
						className={cn(
							"flex items-center gap-1.5 rounded-md px-3 py-1 font-medium text-xs transition-colors",
							activeSubTab === "whitelist"
								? "bg-zinc-800 text-foreground shadow-xs"
								: "text-muted-foreground hover:text-foreground",
						)}
					>
						<Shield className="size-3.5" />
						<span>Whitelist ({whitelist.length})</span>
					</button>
				</div>

				{activeSubTab === "whitelist" && (
					<div className="flex items-center gap-2 text-xs">
						<span className="text-muted-foreground">Status:</span>
						{onToggleWhitelist ? (
							<button
								type="button"
								onClick={() => onToggleWhitelist(!whitelistEnabled)}
								className={cn(
									"flex cursor-pointer items-center gap-1.5 rounded-full px-2.5 py-0.5 font-medium text-[11px] transition-all",
									whitelistEnabled
										? "border border-emerald-500/40 bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30"
										: "border border-zinc-700 bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200",
								)}
								title={
									whitelistEnabled ? "Click to disable whitelist" : "Click to enable whitelist"
								}
							>
								<span
									className={cn(
										"size-1.5 rounded-full",
										whitelistEnabled
											? "bg-emerald-400 shadow-emerald-400 shadow-xs"
											: "bg-zinc-500",
									)}
								/>
								<span>{whitelistEnabled ? "Enforced (Active)" : "Disabled (Click to Enable)"}</span>
							</button>
						) : whitelistEnabled ? (
							<span className="flex items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-medium text-[11px] text-emerald-400">
								<span className="size-1.5 rounded-full bg-emerald-400" />
								Enforced
							</span>
						) : (
							<span className="flex items-center gap-1 rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 font-medium text-[11px] text-amber-400">
								Disabled in Game settings
							</span>
						)}
					</div>
				)}
			</div>

			{/* Sub-Tab 1: Online Players via SLP */}
			{activeSubTab === "online" && (
				<div className="flex flex-col gap-3">
					{!isRunning ? (
						<div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-border/40 border-dashed bg-zinc-900/20 p-8 text-center">
							<Users className="size-8 text-zinc-600" />
							<p className="font-medium text-sm text-zinc-400">Server is offline</p>
							<p className="text-muted-foreground text-xs">
								Start the server to query online players via Server List Ping (SLP).
							</p>
						</div>
					) : isLoadingPing && !pingInfo ? (
						<div className="flex items-center justify-center gap-2 py-8 text-muted-foreground">
							<Loader2 className="size-4 animate-spin text-emerald-400" />
							<span className="text-xs">Querying server via SLP protocol...</span>
						</div>
					) : (pingInfo?.players.online || 0) === 0 ? (
						<div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-border/40 border-dashed bg-zinc-900/20 p-8 text-center">
							<Users className="size-8 text-zinc-600" />
							<p className="font-medium text-sm text-zinc-400">Nobody online</p>
							<p className="text-muted-foreground text-xs">
								0 / {pingInfo?.players.max || 20} players connected. Ping: {pingInfo?.pingMs || 0}ms
							</p>
						</div>
					) : (
						<div className="flex flex-col gap-2">
							<div className="flex items-center justify-between px-1 text-xs text-muted-foreground">
								<span>
									Online Players ({pingInfo?.players.online} / {pingInfo?.players.max})
								</span>
								<span className="flex items-center gap-1 font-mono text-[11px]">
									<Signal className="size-2.5 text-emerald-400" />
									{pingInfo?.pingMs}ms
								</span>
							</div>
							<div className="flex flex-col divide-y divide-border/30 overflow-hidden rounded-xl border border-border/40 bg-zinc-900/30">
								{onlineSample.map((p) => (
									<div key={p.id || p.name} className="flex items-center justify-between px-3.5 py-2.5">
										<div className="flex items-center gap-3">
											<img
												src={`https://mc-heads.net/avatar/${encodeURIComponent(p.name)}/32`}
												alt={p.name}
												className="size-8 shrink-0 rounded-md border border-zinc-700 bg-zinc-800"
												onError={(e) => {
													;(e.currentTarget as HTMLImageElement).src =
														"https://mc-heads.net/avatar/Steve/32"
												}}
											/>
											<div className="flex flex-col">
												<span className="font-medium text-foreground text-xs">{p.name}</span>
												<span className="font-mono text-[10px] text-muted-foreground truncate max-w-[180px]">
													{p.id}
												</span>
											</div>
										</div>

										<div className="flex items-center gap-2">
											{!whitelist.some((w) => w.name.toLowerCase() === p.name.toLowerCase()) && (
												<Button
													variant="outline"
													size="sm"
													onClick={async () => {
														if (!server) return
														await serverService.addToServerWhitelist(server.id, p.name)
														await loadWhitelist()
													}}
													className="h-7 gap-1 border-emerald-500/30 text-[11px] text-emerald-300 hover:bg-emerald-500/10"
													title="Add to whitelist"
												>
													<Plus className="size-3" />
													<span>Whitelist</span>
												</Button>
											)}
											<span className="size-2 rounded-full bg-emerald-400 shadow-emerald-500/50 shadow-sm" />
										</div>
									</div>
								))}
							</div>
						</div>
					)}
				</div>
			)}

			{/* Sub-Tab 2: Whitelist Management */}
			{activeSubTab === "whitelist" && (
				<div className="flex flex-col gap-3">
					{/* Add user form */}
					<form onSubmit={handleAddWhitelist} className="flex items-center gap-2">
						<div className="relative flex-1">
							<UserCheck className="absolute top-1/2 left-3 size-3.5 -translate-y-1/2 text-muted-foreground" />
							<Input
								placeholder="Enter Minecraft username to whitelist..."
								value={newUsername}
								onChange={(e) => setNewUsername(e.target.value)}
								className="h-9 pl-8.5 font-medium text-xs"
							/>
						</div>
						<Button
							type="submit"
							size="sm"
							disabled={isAddingUser || !newUsername.trim()}
							className="h-9 shrink-0 gap-1.5 bg-emerald-600 text-white text-xs hover:bg-emerald-500"
						>
							{isAddingUser ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<Plus className="size-3.5" />
							)}
							<span>Add Player</span>
						</Button>
					</form>

					{whitelistError && (
						<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-2.5 text-rose-300 text-xs">
							<AlertCircle className="size-4 shrink-0 text-rose-400" />
							<span>{whitelistError}</span>
						</div>
					)}

					{/* Whitelist roster list */}
					{isLoadingWhitelist ? (
						<div className="flex items-center justify-center gap-2 py-8 text-muted-foreground">
							<Loader2 className="size-4 animate-spin text-emerald-400" />
							<span className="text-xs">Loading whitelist...</span>
						</div>
					) : whitelist.length === 0 ? (
						<div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-border/40 border-dashed bg-zinc-900/20 p-8 text-center">
							<Shield className="size-8 text-zinc-600" />
							<p className="font-medium text-sm text-zinc-400">Whitelist is empty</p>
							<p className="text-muted-foreground text-xs">
								Add usernames above. When Whitelist is enabled in Game Settings, only these players
								can join.
							</p>
						</div>
					) : (
						<div className="flex max-h-64 flex-col divide-y divide-border/30 overflow-hidden overflow-y-auto rounded-xl border border-border/40 bg-zinc-900/30">
							{whitelist.map((entry) => (
								<div key={entry.name} className="flex items-center justify-between px-3.5 py-2.5">
									<div className="flex items-center gap-3">
										<img
											src={`https://mc-heads.net/avatar/${encodeURIComponent(entry.name)}/32`}
											alt={entry.name}
											className="size-8 shrink-0 rounded-md border border-zinc-700 bg-zinc-800"
											onError={(e) => {
												;(e.currentTarget as HTMLImageElement).src =
													"https://mc-heads.net/avatar/Steve/32"
											}}
										/>
										<div className="flex flex-col">
											<span className="font-medium text-foreground text-xs">{entry.name}</span>
											<span className="max-w-[200px] truncate font-mono text-[10px] text-muted-foreground sm:max-w-xs">
												{entry.uuid || "Offline UUID"}
											</span>
										</div>
									</div>

									<Button
										variant="ghost"
										size="icon-xs"
										onClick={() => handleRemoveWhitelist(entry.name)}
										className="size-8 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
										title={`Remove ${entry.name} from whitelist`}
									>
										<Trash2 className="size-3.5" />
									</Button>
								</div>
							))}
						</div>
					)}
				</div>
			)}
		</div>
	)
}

// ─── Main Dialog ──────────────────────────────────────────────────────────────
export default function ServerSettingsDialog({
	server,
	open,
	onOpenChange,
	onSaved,
	isRunning = false,
}: ServerSettingsDialogProps) {
	const serverId = server?.id || null

	// General tab state (ServerConfig fields)
	const [editName, setEditName] = useState("")
	const [editRamMb, setEditRamMb] = useState(4096)
	const [serverIcon, setServerIcon] = useState<string | null>(null)
	const [isUploadingIcon, setIsUploadingIcon] = useState(false)
	const fileInputRef = useRef<HTMLInputElement | null>(null)
	const [isSavingGeneral, setIsSavingGeneral] = useState(false)
	const [generalError, setGeneralError] = useState<string | null>(null)
	const [generalSuccess, setGeneralSuccess] = useState(false)

	// Game tab state (server.properties)
	const [properties, setProperties] = useState<ServerProperties | null>(null)
	const [isLoadingProps, setIsLoadingProps] = useState(false)
	const [isSavingProps, setIsSavingProps] = useState(false)
	const [propsError, setPropsError] = useState<string | null>(null)
	const [propsSuccess, setPropsSuccess] = useState(false)

	// Live SLP state
	const [pingInfo, setPingInfo] = useState<ServerPingResponse | null>(null)
	const [isLoadingPing, setIsLoadingPing] = useState(false)

	useEffect(() => {
		if (!open || !server) return
		// Prefill General tab
		setEditName(server.name)
		setEditRamMb(server.memoryMaxMb)
		setGeneralError(null)
		setGeneralSuccess(false)
		setPropsError(null)
		setPropsSuccess(false)
	}, [open, server])

	// SLP native query
	const serverPort = server?.port
	useEffect(() => {
		if (!open || !serverPort || !isRunning) {
			setPingInfo(null)
			return
		}

		let isMounted = true
		const fetchSlp = async () => {
			try {
				setIsLoadingPing(true)
				const info = await serverService.pingServer(serverPort)
				if (isMounted) setPingInfo(info)
			} catch {
				// Server may be booting or offline
			} finally {
				if (isMounted) setIsLoadingPing(false)
			}
		}

		fetchSlp()
		const interval = setInterval(fetchSlp, 4000)

		return () => {
			isMounted = false
			clearInterval(interval)
		}
	}, [open, serverPort, isRunning])

	// Load server icon
	useEffect(() => {
		if (!open || !serverId) {
			setServerIcon(null)
			return
		}
		let isMounted = true
		serverService
			.getServerIcon(serverId)
			.then((icon) => {
				if (isMounted) setServerIcon(icon)
			})
			.catch((err) => {
				console.error("Failed to load server icon:", err)
			})
		return () => {
			isMounted = false
		}
	}, [open, serverId])

	const handleIconFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
		const file = e.target.files?.[0]
		if (!file || !serverId) return
		setIsUploadingIcon(true)
		try {
			const reader = new FileReader()
			reader.onload = async () => {
				try {
					const base64 = reader.result as string
					await serverService.setServerIcon(serverId, base64)
					setServerIcon(base64)
					onSaved?.()
				} catch (err: unknown) {
					console.error("Failed to save server icon:", err)
					setGeneralError(err instanceof Error ? err.message : "Failed to upload icon")
				} finally {
					setIsUploadingIcon(false)
					if (fileInputRef.current) fileInputRef.current.value = ""
				}
			}
			reader.readAsDataURL(file)
		} catch (err) {
			console.error("Failed to read file:", err)
			setIsUploadingIcon(false)
		}
	}

	// Load server.properties when opening Game tab (we load eagerly)
	useEffect(() => {
		if (!open || !serverId) return
		let isMounted = true
		setIsLoadingProps(true)
		setPropsError(null)

		serverService
			.getServerProperties(serverId)
			.then((props) => {
				if (isMounted) {
					setProperties(props)
					setIsLoadingProps(false)
				}
			})
			.catch((err) => {
				if (isMounted) {
					console.error("Failed to load server.properties:", err)
					setPropsError("Failed to load server.properties")
					setIsLoadingProps(false)
				}
			})

		return () => {
			isMounted = false
		}
	}, [open, serverId])

	const isGamePortConflict = Boolean(
		properties && serverService.isPortInUse(properties.serverPort, server?.id),
	)

	const handleSaveGeneral = async () => {
		if (!server || !editName.trim()) {
			setGeneralError("Server name cannot be empty.")
			return
		}
		setIsSavingGeneral(true)
		setGeneralError(null)
		try {
			const updated: ServerConfig = {
				...server,
				name: editName.trim(),
				memoryMaxMb: editRamMb,
				memoryMinMb: Math.min(server.memoryMinMb, Math.floor(editRamMb / 2)),
			}
			await serverService.updateServer(updated)
			setGeneralSuccess(true)
			onSaved?.()
			setTimeout(() => setGeneralSuccess(false), 2000)
		} catch (err: unknown) {
			setGeneralError(err instanceof Error ? err.message : "Failed to save settings")
		} finally {
			setIsSavingGeneral(false)
		}
	}

	const handleSaveProps = async () => {
		if (!serverId || !properties) return
		if (isGamePortConflict) {
			setPropsError(`Port ${properties.serverPort} is already in use by another server.`)
			return
		}
		setIsSavingProps(true)
		setPropsError(null)
		try {
			await serverService.setServerProperties(serverId, properties)
			// Also sync port in ServerConfig if changed
			if (server && server.port !== properties.serverPort) {
				await serverService.updateServer({
					...server,
					port: properties.serverPort,
				})
			}
			setPropsSuccess(true)
			onSaved?.()
			setTimeout(() => setPropsSuccess(false), 2000)
		} catch (err: unknown) {
			setPropsError(err instanceof Error ? err.message : "Failed to save properties")
		} finally {
			setIsSavingProps(false)
		}
	}

	const handleOpenFolder = async () => {
		if (!serverId) return
		try {
			await serverService.openServerFolder(serverId)
		} catch (err) {
			console.error("Failed to open server folder:", err)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="flex max-h-[92vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl">
				{/* Header */}
				<DialogHeader className="flex flex-row items-center justify-between gap-3 border-border/30 border-b px-5 py-4">
					<div className="flex items-center gap-2.5">
						<div className="flex size-9 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-400">
							<Settings className="size-4" />
						</div>
						<div>
							<DialogTitle className="font-semibold text-base text-foreground">
								{server?.name || "Server Settings"}
							</DialogTitle>
							<p className="text-muted-foreground text-xs">
								{server?.core?.toUpperCase()} {server?.gameVersion}
							</p>
						</div>
					</div>
					<Button
						variant="outline"
						size="sm"
						onClick={handleOpenFolder}
						className="h-8 shrink-0 gap-1.5 text-xs"
						title="Open server directory in File Explorer"
					>
						<FolderOpen className="size-3.5" />
						<span className="hidden sm:inline">Open Folder</span>
					</Button>
				</DialogHeader>

				{/* Tabs */}
				<Tabs defaultValue="general" className="flex min-h-0 flex-1 flex-col">
					<div className="px-5 pt-3">
						<TabsList className="w-full justify-start border border-border/40 bg-zinc-900/50">
							<TabsTrigger value="general" className="gap-1.5">
								<Server className="size-3.5" />
								General
							</TabsTrigger>
							<TabsTrigger value="game" className="gap-1.5">
								<Settings className="size-3.5" />
								Game
							</TabsTrigger>
							<TabsTrigger value="players" className="gap-1.5">
								<Users className="size-3.5" />
								Players
							</TabsTrigger>
						</TabsList>
					</div>

					{/* ── General Tab ── */}
					<TabsContent value="general" className="overflow-y-auto px-5 pt-3 pb-5">
						<div className="flex flex-col gap-4">
							{/* Server Icon */}
							<div className="flex items-center gap-4 rounded-xl border border-border/40 bg-zinc-900/30 p-3.5">
								<div className="relative flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border/50 bg-zinc-800/80 shadow-inner">
									{serverIcon ? (
										<img
											src={serverIcon}
											alt="Server Icon"
											className="size-full object-cover [image-rendering:pixelated]"
										/>
									) : (
										<Server className="size-6 text-zinc-500" />
									)}
								</div>
								<div className="flex flex-1 flex-col gap-1">
									<span className="font-medium text-foreground text-xs">Server Picture (Icon)</span>
									<p className="text-[11px] text-muted-foreground">
										Upload a custom server icon (64x64 PNG format). Shown in server list and multiplayer ping.
									</p>
									<div className="mt-1 flex items-center gap-2">
										<input
											type="file"
											ref={fileInputRef}
											accept="image/png,image/jpeg,image/webp"
											onChange={handleIconFileSelect}
											className="hidden"
										/>
										<Button
											type="button"
											variant="outline"
											size="sm"
											disabled={isUploadingIcon}
											onClick={() => fileInputRef.current?.click()}
											className="h-7 gap-1.5 text-xs"
										>
											{isUploadingIcon ? (
												<Loader2 className="size-3 animate-spin" />
											) : (
												<Upload className="size-3" />
											)}
											<span>{serverIcon ? "Change Picture" : "Upload Picture"}</span>
										</Button>
									</div>
								</div>
							</div>

							{/* Server Name */}
							<div className="flex flex-col gap-1.5">
								<label htmlFor="edit-name" className="font-medium text-foreground text-xs">
									Server Display Name
								</label>
								<Input
									id="edit-name"
									value={editName}
									onChange={(e) => setEditName(e.target.value)}
									placeholder="My Minecraft Server"
									className="h-9 text-xs"
								/>
							</div>

							{/* RAM Slider */}
							<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/30 p-3.5">
								<div className="flex items-center justify-between">
									<div className="flex items-center gap-1.5 font-medium text-foreground text-xs">
										<Cpu className="size-3.5 text-emerald-400" />
										<span>Memory Allocation (RAM)</span>
									</div>
									<span className="font-bold font-mono text-emerald-400 text-xs">
										{(editRamMb / 1024).toFixed(1)} GB ({editRamMb} MB)
									</span>
								</div>
								<Slider
									min={1024}
									max={16384}
									step={512}
									value={[editRamMb]}
									onValueChange={(val: number | readonly number[]) =>
										setEditRamMb(Array.isArray(val) ? val[0] : (val as number))
									}
									className="my-1"
								/>
								<p className="text-[11px] text-muted-foreground">
									Aikar&apos;s optimized G1GC flags will automatically be applied.
								</p>
							</div>

							{/* Read-only info */}
							<div className="grid grid-cols-2 gap-3">
								<div className="flex flex-col gap-1 rounded-lg border border-border/30 bg-zinc-900/20 px-3 py-2">
									<span className="text-[10px] text-muted-foreground uppercase tracking-wider">
										Core
									</span>
									<span className="font-medium text-foreground text-xs capitalize">
										{server?.core}
									</span>
								</div>
								<div className="flex flex-col gap-1 rounded-lg border border-border/30 bg-zinc-900/20 px-3 py-2">
									<span className="text-[10px] text-muted-foreground uppercase tracking-wider">
										Version
									</span>
									<span className="font-medium font-mono text-foreground text-xs">
										{server?.gameVersion}
									</span>
								</div>
							</div>

							{generalError && (
								<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-2.5 text-rose-300 text-xs">
									<AlertCircle className="size-4 shrink-0 text-rose-400" />
									<span>{generalError}</span>
								</div>
							)}

							<Button
								size="sm"
								onClick={handleSaveGeneral}
								disabled={isSavingGeneral || !editName.trim()}
								className="gap-1.5 self-end bg-emerald-600 text-white text-xs hover:bg-emerald-500"
							>
								{isSavingGeneral ? (
									<>
										<Loader2 className="size-3.5 animate-spin" />
										<span>Saving...</span>
									</>
								) : generalSuccess ? (
									<>
										<Check className="size-3.5" />
										<span>Saved!</span>
									</>
								) : (
									<>
										<Save className="size-3.5" />
										<span>Save Changes</span>
									</>
								)}
							</Button>
						</div>
					</TabsContent>

					{/* ── Game Tab ── */}
					<TabsContent value="game" className="overflow-y-auto px-5 pt-3 pb-5">
						{isLoadingProps ? (
							<div className="flex min-h-[200px] flex-col items-center justify-center gap-2 text-muted-foreground">
								<Loader2 className="size-6 animate-spin text-emerald-400" />
								<span className="text-xs">Loading server configuration...</span>
							</div>
						) : properties ? (
							<div className="flex flex-col gap-4">
								{/* Running Warning Banner */}
								{isRunning && (
									<div className="flex items-start gap-2.5 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-amber-300 text-xs">
										<AlertCircle className="mt-0.5 size-4 shrink-0 text-amber-400" />
										<div className="flex flex-col gap-0.5">
											<span className="font-semibold text-amber-200">Server is currently running</span>
											<p className="text-[11px] text-amber-300/80">
												Changes to{" "}
												<code className="rounded bg-amber-500/20 px-1 py-0.5 font-mono text-[10px] text-amber-200">
													server.properties
												</code>{" "}
												will take effect the next time the server is restarted.
											</p>
										</div>
									</div>
								)}

								{/* MOTD + Preview */}
								<div className="flex flex-col gap-1.5">
									<label htmlFor="motd-input" className="font-medium text-foreground text-xs">
										Server MOTD (Message of the Day)
									</label>
									<Input
										id="motd-input"
										value={properties.motd}
										onChange={(e) =>
											setProperties((p) => (p ? { ...p, motd: e.target.value } : null))
										}
										placeholder="§6§lMy Server§r §7- Welcome!"
										className="h-9 font-mono text-xs w-full min-w-0"
										maxLength={256}
									/>
									<p className="text-[11px] text-muted-foreground">
										Use §-codes for colors: §6 gold, §c red, §a green, §l bold, §r reset
									</p>
									<MotdPreview
										motd={properties.motd}
										icon={serverIcon}
										serverName={editName || server?.name}
										maxPlayers={properties.maxPlayers}
										onlinePlayers={isRunning ? (pingInfo?.players.online ?? 0) : 0}
										pingMs={isRunning ? pingInfo?.pingMs : undefined}
										isRunning={isRunning}
									/>
								</div>

								{/* Port & Max Players */}
								<div className="grid grid-cols-2 gap-3">
									<div className="flex flex-col gap-1.5">
										<div className="flex items-center justify-between">
											<label htmlFor="port-input" className="font-medium text-foreground text-xs">
												Server Port
											</label>
											{isGamePortConflict && (
												<span className="font-medium text-[11px] text-rose-400">
													Port already in use
												</span>
											)}
										</div>
										<Input
											id="port-input"
											type="number"
											min={1024}
											max={65535}
											value={properties.serverPort}
											onChange={(e) =>
												setProperties((p) =>
													p ? { ...p, serverPort: Number(e.target.value) || 25565 } : null,
												)
											}
											className={cn(
												"h-9 font-mono text-xs",
												isGamePortConflict && "border-rose-500 focus-visible:ring-rose-500/50",
											)}
										/>
									</div>
									<div className="flex flex-col gap-1.5">
										<label
											htmlFor="max-players-input"
											className="font-medium text-foreground text-xs"
										>
											Max Players
										</label>
										<Input
											id="max-players-input"
											type="number"
											min={1}
											max={9999}
											value={properties.maxPlayers}
											onChange={(e) =>
												setProperties((p) =>
													p ? { ...p, maxPlayers: Number(e.target.value) || 20 } : null,
												)
											}
											className="h-9 font-mono text-xs"
										/>
									</div>
								</div>

								{/* Gamemode & Difficulty */}
								<div className="grid grid-cols-2 gap-3">
									<div className="flex flex-col gap-1.5">
										<span className="font-medium text-foreground text-xs">Default Gamemode</span>
										<Select
											value={properties.gamemode}
											onValueChange={(val) => {
												if (val) setProperties((p) => (p ? { ...p, gamemode: val } : null))
											}}
										>
											<SelectTrigger className="h-9 text-xs">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="survival">Survival</SelectItem>
												<SelectItem value="creative">Creative</SelectItem>
												<SelectItem value="adventure">Adventure</SelectItem>
												<SelectItem value="spectator">Spectator</SelectItem>
											</SelectContent>
										</Select>
									</div>
									<div className="flex flex-col gap-1.5">
										<span className="font-medium text-foreground text-xs">Difficulty</span>
										<Select
											value={properties.difficulty}
											onValueChange={(val) => {
												if (val) setProperties((p) => (p ? { ...p, difficulty: val } : null))
											}}
										>
											<SelectTrigger className="h-9 text-xs">
												<SelectValue />
											</SelectTrigger>
											<SelectContent>
												<SelectItem value="peaceful">Peaceful</SelectItem>
												<SelectItem value="easy">Easy</SelectItem>
												<SelectItem value="normal">Normal</SelectItem>
												<SelectItem value="hard">Hard</SelectItem>
											</SelectContent>
										</Select>
									</div>
								</div>

								{/* View + Sim Distance */}
								<div className="grid grid-cols-2 gap-3">
									<div className="flex flex-col gap-1.5">
										<label
											htmlFor="view-distance-input"
											className="font-medium text-foreground text-xs"
										>
											View Distance ({properties.viewDistance} chunks)
										</label>
										<Input
											id="view-distance-input"
											type="number"
											min={2}
											max={32}
											value={properties.viewDistance}
											onChange={(e) =>
												setProperties((p) =>
													p ? { ...p, viewDistance: Number(e.target.value) || 10 } : null,
												)
											}
											className="h-9 font-mono text-xs"
										/>
									</div>
									<div className="flex flex-col gap-1.5">
										<label
											htmlFor="sim-distance-input"
											className="font-medium text-foreground text-xs"
										>
											Simulation Distance ({properties.simulationDistance} chunks)
										</label>
										<Input
											id="sim-distance-input"
											type="number"
											min={2}
											max={32}
											value={properties.simulationDistance}
											onChange={(e) =>
												setProperties((p) =>
													p ? { ...p, simulationDistance: Number(e.target.value) || 8 } : null,
												)
											}
											className="h-9 font-mono text-xs"
										/>
									</div>
								</div>

								{/* Toggles */}
								<div className="flex flex-col divide-y divide-border/30 rounded-xl border border-border/40 bg-zinc-900/30">
									<div className="flex items-center justify-between p-3">
										<div className="flex flex-col gap-0.5">
											<div className="flex items-center gap-2 font-medium text-foreground text-xs">
												<Globe className="size-3.5 text-emerald-400" />
												<span>Online Mode (Mojang Auth)</span>
											</div>
											<p className="text-[11px] text-muted-foreground">
												Turn OFF to allow Ely.by, offline, or custom client accounts to connect.
											</p>
										</div>
										<Switch
											checked={properties.onlineMode}
											onCheckedChange={(checked) =>
												setProperties((p) => (p ? { ...p, onlineMode: checked } : null))
											}
										/>
									</div>
									<div className="flex items-center justify-between p-3">
										<div className="flex flex-col gap-0.5">
											<div className="flex items-center gap-2 font-medium text-foreground text-xs">
												<Swords className="size-3.5 text-rose-400" />
												<span>Player vs Player (PvP)</span>
											</div>
											<p className="text-[11px] text-muted-foreground">
												Allow players to damage each other.
											</p>
										</div>
										<Switch
											checked={properties.pvp}
											onCheckedChange={(checked) =>
												setProperties((p) => (p ? { ...p, pvp: checked } : null))
											}
										/>
									</div>
									<div className="flex items-center justify-between p-3">
										<div className="flex flex-col gap-0.5">
											<div className="flex items-center gap-2 font-medium text-foreground text-xs">
												<Shield className="size-3.5 text-amber-400" />
												<span>Enable Whitelist</span>
											</div>
											<p className="text-[11px] text-muted-foreground">
												Only explicitly whitelisted usernames can join this server. Manage roster in
												Players & Whitelist tab.
											</p>
										</div>
										<Switch
											checked={properties.whiteList}
											onCheckedChange={(checked) =>
												setProperties((p) => (p ? { ...p, whiteList: checked } : null))
											}
										/>
									</div>
									<div className="flex items-center justify-between p-3">
										<div className="flex flex-col gap-0.5">
											<span className="font-medium text-foreground text-xs">Allow Flight</span>
											<p className="text-[11px] text-muted-foreground">
												Disable server kicks for flying (useful for creative or modpacks).
											</p>
										</div>
										<Switch
											checked={properties.allowFlight}
											onCheckedChange={(checked) =>
												setProperties((p) => (p ? { ...p, allowFlight: checked } : null))
											}
										/>
									</div>
								</div>

								{propsError && (
									<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-2.5 text-rose-300 text-xs">
										<AlertCircle className="size-4 shrink-0 text-rose-400" />
										<span>{propsError}</span>
									</div>
								)}

								<Button
									size="sm"
									onClick={handleSaveProps}
									disabled={isSavingProps || !properties || isGamePortConflict}
									className="gap-1.5 self-end bg-emerald-600 text-white text-xs hover:bg-emerald-500"
								>
									{isSavingProps ? (
										<>
											<Loader2 className="size-3.5 animate-spin" />
											<span>Saving...</span>
										</>
									) : propsSuccess ? (
										<>
											<Check className="size-3.5" />
											<span>Saved!</span>
										</>
									) : (
										<>
											<Save className="size-3.5" />
											<span>Save Changes</span>
										</>
									)}
								</Button>
							</div>
						) : null}
					</TabsContent>

					{/* ── Players & Whitelist Tab ── */}
					<TabsContent value="players" className="overflow-y-auto px-5 pt-3 pb-5">
						<PlayersTab
							server={server}
							isRunning={isRunning}
							whitelistEnabled={Boolean(properties?.whiteList)}
							pingInfo={pingInfo}
							isLoadingPing={isLoadingPing}
							onToggleWhitelist={async (enabled) => {
								if (!serverId || !properties) return
								const updated = { ...properties, whiteList: enabled }
								setProperties(updated)
								try {
									await serverService.setServerProperties(serverId, updated)
									onSaved?.()
								} catch (err) {
									console.error("Failed to toggle whitelist:", err)
								}
							}}
						/>
					</TabsContent>
				</Tabs>
			</DialogContent>
		</Dialog>
	)
}
