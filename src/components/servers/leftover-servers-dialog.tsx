import { Check, Loader2, Server, TriangleAlert } from "lucide-react"
import { useEffect, useState } from "react"
import type { LeftoverServer } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { rpc, serverService } from "@/services/server-service"

const since = (startedAt: number) =>
	new Date(startedAt * 1000).toLocaleString(undefined, {
		weekday: "short",
		hour: "2-digit",
		minute: "2-digit",
	})

/**
 * Servers still running from before Ingot last closed (a crash, or Task Manager): their
 * console is gone, so all Ingot can do is end them. Shown on open, and when starting
 * one of them runs into it.
 */
export default function LeftoverServersDialog() {
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
	const one = servers.length === 1
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
					<TriangleAlert className="mt-0.5 size-5 shrink-0 text-amber-400" />
					<div className="flex flex-col gap-1">
						<DialogTitle className="text-base">
							{one ? "A server is" : "Servers are"} still running from last time
						</DialogTitle>
						<DialogDescription className="text-sm text-zinc-400 leading-relaxed">
							Ingot closed without stopping {one ? "it" : "them"}, so{" "}
							{one ? "its console is" : "their consoles are"} gone: Ingot can't send commands or
							save {one ? "its world" : "their worlds"}. {one ? "It keeps its" : "They keep their"}{" "}
							port and world busy until stopped.
						</DialogDescription>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					{servers.map((server) => {
						const done = stopped.includes(server.serverId)
						return (
							<div
								key={server.serverId}
								className="flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/40 px-3 py-2"
							>
								<Server className="size-4 shrink-0 text-zinc-500" />
								<div className="min-w-0 flex-1">
									<p className="truncate font-medium text-sm text-zinc-100">{server.name}</p>
									<p className="truncate text-[11px] text-zinc-500">
										Running since {since(server.startedAt)}
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
										<Loader2 className="size-3.5 animate-spin" />
									) : done ? (
										<Check className="size-3.5" />
									) : null}
									{done ? "Stopped" : "Stop"}
								</Button>
							</div>
						)
					})}
				</div>

				{!allStopped && (
					<p className="text-xs text-zinc-400 leading-relaxed">
						Stopping ends it right away. Anything since the last autosave (a few minutes at most) is
						lost, and the world itself stays fine.
					</p>
				)}

				{error && (
					<p className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-red-200 text-xs">
						{error}
					</p>
				)}

				<div className="flex justify-end">
					<Button
						variant={allStopped ? "default" : "ghost"}
						disabled={busy !== null}
						onClick={() => setServers([])}
						className="rounded-xl"
					>
						{allStopped ? "Done" : "Leave running"}
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	)
}
