import {
	AlertCircle,
	Check,
	FolderOpen,
	Globe,
	Loader2,
	Save,
	Settings,
	Shield,
	Swords,
} from "lucide-react"
import { useEffect, useState } from "react"
import type { ServerConfig, ServerProperties } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { serverService } from "@/services/server-service"

export interface ServerSettingsDialogProps {
	server: ServerConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onSaved?: () => void
}

export default function ServerSettingsDialog({
	server,
	open,
	onOpenChange,
	onSaved,
}: ServerSettingsDialogProps) {
	const serverId = server?.id || null
	const [properties, setProperties] = useState<ServerProperties | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [isSaving, setIsSaving] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [saveSuccess, setSaveSuccess] = useState(false)

	useEffect(() => {
		if (!open || !serverId) return
		let isMounted = true
		setIsLoading(true)
		setError(null)
		setSaveSuccess(false)

		serverService
			.getServerProperties(serverId)
			.then((props) => {
				if (isMounted) {
					setProperties(props)
					setIsLoading(false)
				}
			})
			.catch((err) => {
				if (isMounted) {
					console.error("Failed to load server.properties:", err)
					setError("Failed to load server.properties")
					setIsLoading(false)
				}
			})

		return () => {
			isMounted = false
		}
	}, [open, serverId])

	const handleSave = async () => {
		if (!serverId || !properties) return
		setIsSaving(true)
		setError(null)

		try {
			await serverService.setServerProperties(serverId, properties)
			setSaveSuccess(true)
			onSaved?.()
			setTimeout(() => {
				onOpenChange(false)
			}, 600)
		} catch (err: unknown) {
			console.error("Failed to save server properties:", err)
			setError(err instanceof Error ? err.message : "Failed to save properties")
		} finally {
			setIsSaving(false)
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
			<DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
				<DialogHeader>
					<div className="flex items-center justify-between">
						<div className="flex items-center gap-2.5">
							<div className="flex size-9 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-400">
								<Settings className="size-4" />
							</div>
							<div>
								<DialogTitle className="font-semibold text-base text-foreground">
									Server Settings
								</DialogTitle>
								<DialogDescription className="text-xs">
									{server?.name} ({server?.core?.toUpperCase()} {server?.gameVersion})
								</DialogDescription>
							</div>
						</div>

						<Button
							variant="outline"
							size="sm"
							onClick={handleOpenFolder}
							className="h-8 gap-1.5 text-xs"
							title="Open server directory in File Explorer"
						>
							<FolderOpen className="size-3.5" />
							<span className="hidden sm:inline">Open Folder</span>
						</Button>
					</div>
				</DialogHeader>

				{isLoading ? (
					<div className="flex min-h-[300px] flex-col items-center justify-center gap-2 text-muted-foreground">
						<Loader2 className="size-6 animate-spin text-emerald-400" />
						<span className="text-xs">Loading server configuration...</span>
					</div>
				) : properties ? (
					<div className="flex flex-col gap-4 py-2">
						{/* Server MOTD */}
						<div className="flex flex-col gap-1.5">
							<label htmlFor="motd-input" className="font-medium text-foreground text-xs">
								Server MOTD (Message of the Day)
							</label>
							<Input
								id="motd-input"
								value={properties.motd}
								onChange={(e) => setProperties((p) => (p ? { ...p, motd: e.target.value } : null))}
								className="h-9 text-xs"
							/>
						</div>

						{/* Port & Max Players */}
						<div className="grid grid-cols-2 gap-3">
							<div className="flex flex-col gap-1.5">
								<label htmlFor="port-input" className="font-medium text-foreground text-xs">
									Server Port
								</label>
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
									className="h-9 font-mono text-xs"
								/>
							</div>

							<div className="flex flex-col gap-1.5">
								<label htmlFor="max-players-input" className="font-medium text-foreground text-xs">
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

						{/* Distance Settings */}
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
								<label htmlFor="sim-distance-input" className="font-medium text-foreground text-xs">
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

						{/* Gameplay Toggles */}
						<div className="flex flex-col divide-y divide-border/30 rounded-xl border border-border/40 bg-zinc-900/30">
							{/* Online Mode / Cracked */}
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

							{/* PvP */}
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

							{/* Whitelist */}
							<div className="flex items-center justify-between p-3">
								<div className="flex flex-col gap-0.5">
									<div className="flex items-center gap-2 font-medium text-foreground text-xs">
										<Shield className="size-3.5 text-amber-400" />
										<span>Enable Whitelist</span>
									</div>
									<p className="text-[11px] text-muted-foreground">
										Only explicitly whitelisted usernames can join this server.
									</p>
								</div>
								<Switch
									checked={properties.whiteList}
									onCheckedChange={(checked) =>
										setProperties((p) => (p ? { ...p, whiteList: checked } : null))
									}
								/>
							</div>

							{/* Allow Flight */}
							<div className="flex items-center justify-between p-3">
								<div className="flex flex-col gap-0.5">
									<span className="font-medium text-foreground text-xs">Allow Flight</span>
									<p className="text-[11px] text-muted-foreground">
										Disable server kicks for flying (useful for modpacks or creative building).
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

						{/* Error */}
						{error && (
							<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-2.5 text-rose-300 text-xs">
								<AlertCircle className="size-4 shrink-0 text-rose-400" />
								<span>{error}</span>
							</div>
						)}
					</div>
				) : null}

				<DialogFooter className="gap-2 sm:gap-0">
					<Button
						variant="ghost"
						size="sm"
						onClick={() => onOpenChange(false)}
						disabled={isSaving}
						className="text-xs"
					>
						Cancel
					</Button>
					<Button
						size="sm"
						onClick={handleSave}
						disabled={isSaving || !properties}
						className="gap-1.5 bg-emerald-600 text-white text-xs hover:bg-emerald-500"
					>
						{isSaving ? (
							<>
								<Loader2 className="size-3.5 animate-spin" />
								<span>Saving...</span>
							</>
						) : saveSuccess ? (
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
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
