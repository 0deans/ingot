import { AlertCircle, Play, Plus, Server } from "lucide-react"
import { useEffect, useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { ServerConfig } from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
	FieldTitle,
} from "@/components/ui/field"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
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
							<span className="font-mono text-2xs text-muted-foreground">
								{t("quickJoin.target", { address: `127.0.0.1:${server.port}` })}
							</span>
						</div>
						<span className="rounded bg-primary/10 px-2 py-0.5 font-mono text-2xs text-primary">
							MC {server.gameVersion}
						</span>
					</div>

					<div className="space-y-2">
						<div className="flex items-center justify-between">
							<span className="font-medium text-2xs text-muted-foreground uppercase tracking-wider">
								{t("quickJoin.choose")}
							</span>
							{onCreateInstance && (
								<Button
									variant="link"
									size="xs"
									onClick={() => {
										onOpenChange(false)
										onCreateInstance()
									}}
									className="h-auto p-0"
								>
									<Plus />
									{t("instances.newInstance")}
								</Button>
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
								<p className="text-2xs text-muted-foreground">{t("quickJoin.noInstancesHint")}</p>
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
							<ScrollArea className="scroll-fade-y max-h-56 pr-1">
								<RadioGroup value={selectedId} onValueChange={(id) => setSelectedId(id as string)}>
									{instances.map((inst) => (
										<FieldLabel key={inst.id} htmlFor={`quick-join-${inst.id}`}>
											<Field orientation="horizontal" className="items-center">
												<LoaderIcon loader={inst.loader} size={16} />
												<FieldContent className="min-w-0">
													<FieldTitle className="truncate">{inst.name}</FieldTitle>
													<FieldDescription className="flex items-center gap-1.5 font-mono">
														{inst.gameVersion}
														{inst.gameVersion === server.gameVersion && (
															<Badge variant="outline" className="border-primary/30 text-primary">
																{t("quickJoin.matches")}
															</Badge>
														)}
													</FieldDescription>
												</FieldContent>
												<RadioGroupItem value={inst.id} id={`quick-join-${inst.id}`} />
											</Field>
										</FieldLabel>
									))}
								</RadioGroup>
							</ScrollArea>
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
