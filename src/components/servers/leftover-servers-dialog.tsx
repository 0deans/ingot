import { Check, Server, TriangleAlert } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { LeftoverServer } from "@/bindings"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/common/dialog"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { formatDateTime } from "@/lib/format"
import { rpc, serverService } from "@/services/server-service"

const since = (startedAt: number) =>
	formatDateTime(startedAt, { weekday: "short", hour: "2-digit", minute: "2-digit" })

/**
 * Servers still running from before Ingot last closed (a crash, or Task Manager): their
 * console is gone, so all Ingot can do is end them. Shown on open, and when starting
 * one of them runs into it.
 */
export default function LeftoverServersDialog() {
	const { t } = useTranslation()
	const [servers, setServers] = useState<LeftoverServer[]>([])
	const [stopped, setStopped] = useState<string[]>([])
	const [busy, setBusy] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		const show = (list: LeftoverServer[]) => {
			if (list.length === 0) return
			setServers(list)
			setStopped([])
			setError(null)
		}
		rpc.get_leftover_servers().then(show).catch(console.error)
		let unlisten: (() => void) | undefined
		let cancelled = false
		rpc.on_leftover_servers
			.on(show)
			.then((stop) => {
				if (cancelled) stop()
				else unlisten = stop
			})
			.catch(console.error)
		return () => {
			cancelled = true
			unlisten?.()
		}
	}, [])

	if (servers.length === 0) return null
	const allStopped = servers.every((s) => stopped.includes(s.serverId))

	const stop = async (server: LeftoverServer) => {
		setBusy(server.serverId)
		setError(null)
		try {
			await rpc.stop_leftover_server(server.serverId)
			setStopped((prev) => [...prev, server.serverId])
			await serverService.refreshServers()
		} catch (e) {
			setError(String(e))
		} finally {
			setBusy(null)
		}
	}

	return (
		<Dialog open onOpenChange={(open) => !open && !busy && setServers([])}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-4 p-5 sm:max-w-md">
				<div className="flex items-start gap-3">
					<TriangleAlert className="mt-0.5 size-5 shrink-0 text-warning" />
					<div className="flex flex-col gap-1">
						<DialogTitle className="text-base">
							{t("leftoverServers.title", { count: servers.length })}
						</DialogTitle>
						<DialogDescription className="text-muted-foreground text-sm leading-relaxed">
							{t("leftoverServers.description", { count: servers.length })}
						</DialogDescription>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					{servers.map((server) => {
						const done = stopped.includes(server.serverId)
						return (
							<div
								key={server.serverId}
								className="flex items-center gap-3 rounded-xl border border-border bg-card/40 px-3 py-2"
							>
								<Server className="size-4 shrink-0 text-muted-foreground" />
								<div className="min-w-0 flex-1">
									<p className="truncate font-medium text-foreground text-sm">{server.name}</p>
									<p className="truncate text-[11px] text-muted-foreground">
										{t("leftoverServers.runningSince", { time: since(server.startedAt) })}
									</p>
								</div>
								<Button
									size="sm"
									variant="outline"
									disabled={done || busy !== null}
									onClick={() => stop(server)}
									className="h-8 gap-1.5 rounded-lg text-xs"
								>
									{busy === server.serverId ? (
										<Spinner className="size-3.5" />
									) : done ? (
										<Check className="size-3.5" />
									) : null}
									{done ? t("leftoverServers.stopped") : t("common.stop")}
								</Button>
							</div>
						)
					})}
				</div>

				{!allStopped && (
					<p className="text-muted-foreground text-xs leading-relaxed">
						{t("leftoverServers.stopNote")}
					</p>
				)}

				{error && (
					<Alert variant="destructive">
						<AlertDescription>{error}</AlertDescription>
					</Alert>
				)}

				<div className="flex justify-end">
					<Button
						variant={allStopped ? "default" : "ghost"}
						disabled={busy !== null}
						onClick={() => setServers([])}
						className="rounded-xl"
					>
						{allStopped ? t("common.done") : t("leftoverServers.leaveRunning")}
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	)
}
