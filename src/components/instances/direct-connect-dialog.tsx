import { ArrowRight, Clock, Globe, Play, Server, Trash2 } from "lucide-react"
import { useEffect, useState } from "react"
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
import { useServers } from "@/services/server-service"

export interface DirectConnectDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	instanceName: string
	onConnect: (serverAddress: string) => void
}

const RECENT_SERVERS_KEY = "ingot_recent_servers"

function getRecentServers(): string[] {
	try {
		const raw = localStorage.getItem(RECENT_SERVERS_KEY)
		if (!raw) return []
		const parsed = JSON.parse(raw)
		return Array.isArray(parsed) ? parsed.filter((s): s is string => typeof s === "string") : []
	} catch {
		return []
	}
}

function saveRecentServer(address: string) {
	try {
		const cleaned = address.trim()
		if (!cleaned) return
		const existing = getRecentServers().filter((s) => s.toLowerCase() !== cleaned.toLowerCase())
		const updated = [cleaned, ...existing].slice(0, 6)
		localStorage.setItem(RECENT_SERVERS_KEY, JSON.stringify(updated))
	} catch (e) {
		console.error("Failed to save recent server:", e)
	}
}

export function DirectConnectDialog({
	open,
	onOpenChange,
	instanceName,
	onConnect,
}: DirectConnectDialogProps) {
	const [address, setAddress] = useState("")
	const [recentServers, setRecentServers] = useState<string[]>([])
	const { servers } = useServers()

	useEffect(() => {
		if (open) {
			setRecentServers(getRecentServers())
		}
	}, [open])

	const handleLaunch = (targetAddress?: string) => {
		const finalAddress = (targetAddress || address).trim()
		if (!finalAddress) return
		saveRecentServer(finalAddress)
		onConnect(finalAddress)
		onOpenChange(false)
	}

	const handleRemoveRecent = (item: string, e: React.MouseEvent) => {
		e.stopPropagation()
		const filtered = recentServers.filter((s) => s !== item)
		setRecentServers(filtered)
		localStorage.setItem(RECENT_SERVERS_KEY, JSON.stringify(filtered))
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2 text-base">
						<Globe className="size-4 text-emerald-400" />
						<span>Direct Connect to Server</span>
					</DialogTitle>
					<DialogDescription className="text-xs">
						Launch <span className="font-semibold text-zinc-200">{instanceName}</span> directly into
						a multiplayer server, bypassing the main menu.
					</DialogDescription>
				</DialogHeader>

				<form
					onSubmit={(e) => {
						e.preventDefault()
						handleLaunch()
					}}
					className="space-y-4 pt-1"
				>
					<div className="space-y-1.5">
						<label htmlFor="server-address" className="font-medium text-xs text-zinc-300">
							Server Address
						</label>
						<div className="relative flex items-center">
							<Input
								id="server-address"
								autoFocus
								value={address}
								onChange={(e) => setAddress(e.target.value)}
								placeholder="e.g. mc.hypixel.net or localhost:25565"
								className="h-9 pr-8 font-mono text-xs"
							/>
						</div>
					</div>

					{/* Local Servers Section if any configured */}
					{servers.length > 0 && (
						<div className="space-y-1.5">
							<span className="font-medium text-[11px] text-zinc-400 uppercase tracking-wider">
								Configured Local Servers
							</span>
							<div className="grid max-h-32 grid-cols-1 gap-1.5 overflow-y-auto">
								{servers.map((srv) => (
									<button
										key={srv.id}
										type="button"
										onClick={() => {
											const addr = `127.0.0.1:${srv.port}`
											setAddress(addr)
											handleLaunch(addr)
										}}
										className="group flex items-center justify-between rounded-lg border border-border/50 bg-zinc-900/60 px-2.5 py-1.5 text-left text-xs transition-colors hover:border-emerald-500/50 hover:bg-zinc-800/80"
									>
										<div className="flex items-center gap-2 truncate">
											<Server className="size-3.5 shrink-0 text-emerald-400" />
											<span className="truncate font-medium text-zinc-200">{srv.name}</span>
											<span className="font-mono text-[11px] text-zinc-400">:{srv.port}</span>
										</div>
										<ArrowRight className="size-3 text-zinc-500 transition-transform group-hover:translate-x-0.5 group-hover:text-emerald-400" />
									</button>
								))}
							</div>
						</div>
					)}

					{/* Recent Servers Section */}
					{recentServers.length > 0 && (
						<div className="space-y-1.5">
							<span className="font-medium text-[11px] text-zinc-400 uppercase tracking-wider">
								Recent Servers
							</span>
							<div className="flex flex-wrap gap-1.5">
								{recentServers.map((item) => (
									<div
										key={item}
										className="group inline-flex items-center gap-1 rounded-md border border-zinc-800 bg-zinc-900/90 px-2 py-1 text-[11px] text-zinc-300 transition-colors hover:border-zinc-700 hover:text-white"
									>
										<button
											type="button"
											onClick={() => {
												setAddress(item)
												handleLaunch(item)
											}}
											className="flex items-center gap-1 font-mono hover:text-emerald-400"
										>
											<Clock className="size-2.5 opacity-60" />
											<span>{item}</span>
										</button>
										<button
											type="button"
											onClick={(e) => handleRemoveRecent(item, e)}
											className="ml-0.5 text-zinc-500 hover:text-rose-400"
											title="Remove from recents"
										>
											<Trash2 className="size-2.5" />
										</button>
									</div>
								))}
							</div>
						</div>
					)}

					<DialogFooter className="gap-2 pt-2 sm:gap-0">
						<Button
							type="button"
							variant="ghost"
							size="sm"
							onClick={() => onOpenChange(false)}
							className="text-xs"
						>
							Cancel
						</Button>
						<Button
							type="submit"
							size="sm"
							disabled={!address.trim()}
							className="gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
						>
							<Play className="size-3.5 fill-current" />
							<span>Launch & Connect</span>
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	)
}
