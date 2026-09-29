import { AlertCircle, Check, Play, Plus, Server } from "lucide-react"
import { useEffect, useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { ServerConfig } from "@/bindings"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/common/dialog"
import LoaderIcon from "@/components/instances/loader-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
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
	const { t } = useTranslation()
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
						<Server className="size-4 text-primary" />
						<span>{t("quickJoin.title")}</span>
					</DialogTitle>
					<DialogDescription className="text-xs">
						<Trans
							i18nKey="quickJoin.description"
							values={{ name: server.name, address: `127.0.0.1:${server.port}` }}
							components={{ b: <span className="font-semibold text-foreground" /> }}
						/>
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-4 pt-1">
					<div className="flex items-center justify-between rounded-lg border border-border bg-card/60 p-2.5 text-xs">
						<div className="flex flex-col">
							<span className="font-medium text-foreground/80">{server.name}</span>
							<span className="font-mono text-[11px] text-muted-foreground">
								{t("quickJoin.target", { address: `127.0.0.1:${server.port}` })}
							</span>
						</div>
						<span className="rounded bg-primary/10 px-2 py-0.5 font-mono text-[11px] text-primary">
							MC {server.gameVersion}
						</span>
					</div>

					<div className="space-y-2">
						<div className="flex items-center justify-between">
							<span className="font-medium text-[11px] text-muted-foreground uppercase tracking-wider">
								{t("quickJoin.choose")}
							</span>
							{onCreateInstance && (
								<button
									type="button"
									onClick={() => {
										onOpenChange(false)
										onCreateInstance()
									}}
									className="flex items-center gap-1 text-[11px] text-primary transition-colors hover:text-primary"
								>
									<Plus className="size-3" />
									<span>{t("instances.newInstance")}</span>
								</button>
							)}
						</div>

						{isLoading ? (
							<div className="flex items-center justify-center p-8 text-muted-foreground">
								<Spinner className="size-5" />
							</div>
						) : instances.length === 0 ? (
							<div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-border border-dashed p-6 text-center">
								<AlertCircle className="size-6 text-warning" />
								<p className="font-medium text-foreground/80 text-xs">
									{t("quickJoin.noInstances")}
								</p>
								<p className="text-[11px] text-muted-foreground">
									{t("quickJoin.noInstancesHint")}
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
										{t("quickJoin.createInstance")}
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
													? "border-primary/60 bg-primary/10 ring-1 ring-primary/30"
													: "border-border/50 bg-card/40 hover:border-input hover:bg-muted/60",
											)}
										>
											<div className="flex items-center gap-2.5 truncate">
												<div className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-card">
													<LoaderIcon loader={inst.loader} size={16} />
												</div>
												<div className="flex flex-col truncate">
													<span className="truncate font-medium text-foreground text-xs">
														{inst.name}
													</span>
													<div className="flex items-center gap-1.5 font-mono text-[10px] text-muted-foreground">
														<span>{inst.gameVersion}</span>
														{isVersionMatch && (
															<Badge variant="outline" className="border-primary/30 text-primary">
																{t("quickJoin.matches")}
															</Badge>
														)}
													</div>
												</div>
											</div>

											{isSelected && <Check className="size-4 shrink-0 text-primary" />}
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
							{t("common.cancel")}
						</Button>
						<Button
							type="button"
							size="sm"
							disabled={!selectedId}
							onClick={handleJoin}
							className="gap-1.5 font-medium text-xs"
						>
							<Play className="size-3.5 fill-current" />
							<span>{t("quickJoin.launch")}</span>
						</Button>
					</DialogFooter>
				</div>
			</DialogContent>
		</Dialog>
	)
}
