import { AlertCircle, Check, Loader2, Play, Plus, Server } from "lucide-react"
import { useEffect, useState } from "react"
import type { ServerConfig } from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { cn } from "@/lib/utils"
import { useInstances } from "@/services/instance-service"

export interface QuickJoinDialogProps {
	server: ServerConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onLaunch: (instanceId: string, serverAddress: string) => void
	onCreateInstance?: () => void
}

const SERVER_INSTANCE_MAP_KEY = "ingot_server_instance_map"

function getLastSelectedInstance(serverId: string): string | null {
	try {
		const raw = localStorage.getItem(SERVER_INSTANCE_MAP_KEY)
		if (!raw) return null
		const map = JSON.parse(raw)
		return map[serverId] || null
	} catch {
		return null
	}
}

function setLastSelectedInstance(serverId: string, instanceId: string) {
	try {
		const raw = localStorage.getItem(SERVER_INSTANCE_MAP_KEY)
		const map = raw ? JSON.parse(raw) : {}
		map[serverId] = instanceId
		localStorage.setItem(SERVER_INSTANCE_MAP_KEY, JSON.stringify(map))
	} catch (e) {
		console.error("Failed to save last selected instance:", e)
	}
}

export function QuickJoinDialog({
	server,
	open,
	onOpenChange,
	onLaunch,
	onCreateInstance,
}: QuickJoinDialogProps) {
	const { instances, isLoading } = useInstances()
	const [selectedId, setSelectedId] = useState<string | null>(null)

	useEffect(() => {
		if (!open || !server || instances.length === 0) return

		const lastId = getLastSelectedInstance(server.id)
		if (lastId && instances.some((i) => i.id === lastId)) {
			setSelectedId(lastId)
			return
		}

		// Otherwise prefer an instance matching the game version
		const matching = instances.find((i) => i.gameVersion === server.gameVersion)
		if (matching) {
			setSelectedId(matching.id)
		} else {
			setSelectedId(instances[0].id)
		}
	}, [open, server, instances])

	if (!server) return null

	const handleJoin = () => {
		if (!selectedId) return
		setLastSelectedInstance(server.id, selectedId)
		onLaunch(selectedId, `127.0.0.1:${server.port}`)
		onOpenChange(false)
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2 text-base">
						<Server className="size-4 text-emerald-400" />
						<span>Quick Play — Join Server</span>
					</DialogTitle>
					<DialogDescription className="text-xs">
						Launch Minecraft and join{" "}
						<span className="font-semibold text-zinc-200">{server.name}</span> (127.0.0.1:
						{server.port}) directly.
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-4 pt-1">
					<div className="flex items-center justify-between rounded-lg border border-zinc-800 bg-zinc-900/60 p-2.5 text-xs">
						<div className="flex flex-col">
							<span className="font-medium text-zinc-300">{server.name}</span>
							<span className="font-mono text-[11px] text-muted-foreground">
								Target: 127.0.0.1:{server.port}
							</span>
						</div>
						<span className="rounded bg-emerald-500/10 px-2 py-0.5 font-mono text-[11px] text-emerald-400">
							MC {server.gameVersion}
						</span>
					</div>

					<div className="space-y-2">
						<div className="flex items-center justify-between">
							<span className="font-medium text-[11px] text-zinc-400 uppercase tracking-wider">
								Choose Client Instance
							</span>
							{onCreateInstance && (
								<button
									type="button"
									onClick={() => {
										onOpenChange(false)
										onCreateInstance()
									}}
									className="flex items-center gap-1 text-[11px] text-emerald-400 transition-colors hover:text-emerald-300"
								>
									<Plus className="size-3" />
									<span>New Instance</span>
								</button>
							)}
						</div>

						{isLoading ? (
							<div className="flex items-center justify-center p-8 text-muted-foreground">
								<Loader2 className="size-5 animate-spin" />
							</div>
						) : instances.length === 0 ? (
							<div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-zinc-800 border-dashed p-6 text-center">
								<AlertCircle className="size-6 text-amber-400" />
								<p className="font-medium text-xs text-zinc-300">No Minecraft instances found</p>
								<p className="text-[11px] text-zinc-500">
									Create a client instance first to join your server.
								</p>
								{onCreateInstance && (
									<Button
										size="sm"
										variant="outline"
										onClick={() => {
											onOpenChange(false)
											onCreateInstance()
										}}
										className="mt-2 text-xs"
									>
										Create Instance
									</Button>
								)}
							</div>
						) : (
							<div className="max-h-56 space-y-1.5 overflow-y-auto pr-1">
								{instances.map((inst) => {
									const isSelected = inst.id === selectedId
									const isVersionMatch = inst.gameVersion === server.gameVersion

									return (
										<button
											key={inst.id}
											type="button"
											onClick={() => setSelectedId(inst.id)}
											className={cn(
												"flex w-full items-center justify-between rounded-lg border p-2.5 text-left transition-all",
												isSelected
													? "border-emerald-500/60 bg-emerald-950/20 ring-1 ring-emerald-500/30"
													: "border-border/50 bg-zinc-900/40 hover:border-zinc-700 hover:bg-zinc-800/60",
											)}
										>
											<div className="flex items-center gap-2.5 truncate">
												<div className="flex size-7 shrink-0 items-center justify-center rounded-md border border-zinc-800 bg-zinc-900">
													<LoaderIcon loader={inst.loader} size={16} />
												</div>
												<div className="flex flex-col truncate">
													<span className="truncate font-medium text-xs text-zinc-200">
														{inst.name}
													</span>
													<div className="flex items-center gap-1.5 font-mono text-[10px] text-zinc-400">
														<span>{inst.gameVersion}</span>
														{isVersionMatch && (
															<span className="rounded bg-emerald-500/20 px-1 py-0.2 text-[9px] text-emerald-300">
																Matches Server
															</span>
														)}
													</div>
												</div>
											</div>

											{isSelected && <Check className="size-4 shrink-0 text-emerald-400" />}
										</button>
									)
								})}
							</div>
						)}
					</div>

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
							type="button"
							size="sm"
							disabled={!selectedId}
							onClick={handleJoin}
							className="gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
						>
							<Play className="size-3.5 fill-current" />
							<span>Launch & Join</span>
						</Button>
					</DialogFooter>
				</div>
			</DialogContent>
		</Dialog>
	)
}
