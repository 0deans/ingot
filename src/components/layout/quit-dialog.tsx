import { getCurrentWindow } from "@tauri-apps/api/window"
import { Gamepad2, Moon, Server, TriangleAlert } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { QuitRequest } from "@/bindings"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/common/dialog"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { type QuitPrompt, quitService } from "@/services/quit-service"
import { rpc } from "@/services/server-service"

/**
 * Asked when Ingot would close while servers run: their consoles live in Ingot, so
 * quitting would leave them running with no way to control them.
 */
export default function QuitDialog() {
	const { t } = useTranslation()
	const [prompt, setPrompt] = useState<QuitPrompt | null>(null)
	const [stopping, setStopping] = useState(false)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		const unsubscribe = quitService.subscribe((next) => {
			setPrompt(next)
			setError(null)
		})
		let unlisten: (() => void) | undefined
		let cancelled = false
		rpc.on_quit_requested
			.on((request: QuitRequest) => quitService.ask({ mode: "quit", request }))
			.then((stop) => {
				if (cancelled) stop()
				else unlisten = stop
			})
			.catch(console.error)
		return () => {
			cancelled = true
			unsubscribe()
			unlisten?.()
		}
	}, [])

	if (!prompt) return null
	const { mode, request } = prompt
	const running = request.servers.filter((s) => !s.sleeping)
	const count = request.servers.length

	const stopAndGo = async () => {
		setStopping(true)
		setError(null)
		try {
			await rpc.stop_all_servers()
			if (mode === "quit") {
				await rpc.quit_app()
			} else {
				setPrompt(null)
				await prompt.proceed?.()
			}
		} catch (e) {
			setError(String(e))
		} finally {
			setStopping(false)
		}
	}

	const keepInTray = async () => {
		try {
			await getCurrentWindow().hide()
			setPrompt(null)
		} catch (e) {
			setError(String(e))
		}
	}

	return (
		<Dialog open onOpenChange={(open) => !open && !stopping && setPrompt(null)}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-4 p-5 sm:max-w-md">
				<div className="flex items-start gap-3">
					<TriangleAlert className="mt-0.5 size-5 shrink-0 text-amber-400" />
					<div className="flex flex-col gap-1">
						<DialogTitle className="text-base">{t(`quitDialog.title_${mode}`)}</DialogTitle>
						<DialogDescription className="text-sm text-zinc-400 leading-relaxed">
							{running.length === 0
								? t("quitDialog.asleep", { count })
								: t("quitDialog.consoles", { count })}
						</DialogDescription>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					{request.servers.map((server) => (
						<div
							key={server.serverId}
							className="flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/40 px-3 py-2"
						>
							{server.sleeping ? (
								<Moon className="size-4 shrink-0 text-zinc-500" />
							) : (
								<Server className="size-4 shrink-0 text-emerald-400" />
							)}
							<p className="min-w-0 flex-1 truncate font-medium text-sm text-zinc-100">
								{server.name}
							</p>
							<span className="shrink-0 text-[11px] text-zinc-500">
								{server.sleeping ? t("serverStatus.sleeping") : t("serverStatus.running")}
							</span>
						</div>
					))}
				</div>

				{stopping ? (
					<p className="flex items-center gap-2 text-xs text-zinc-400">
						<Spinner className="size-3.5 shrink-0" />
						{running.length > 0
							? t("quitDialog.saving", { count: running.length })
							: t("quitDialog.stopping")}
					</p>
				) : (
					mode === "quit" &&
					request.games > 0 && (
						<p className="flex items-start gap-2 text-xs text-zinc-400 leading-relaxed">
							<Gamepad2 className="mt-px size-3.5 shrink-0" />
							{t("quitDialog.games", { count: request.games })}
						</p>
					)
				)}

				{error && (
					<Alert variant="destructive">
						<AlertDescription>{error}</AlertDescription>
					</Alert>
				)}

				<div className="flex flex-wrap justify-end gap-2">
					<Button
						variant="ghost"
						disabled={stopping}
						onClick={() => setPrompt(null)}
						className="rounded-xl"
					>
						{t("common.cancel")}
					</Button>
					{mode === "quit" && (
						<Button
							variant="outline"
							disabled={stopping}
							onClick={keepInTray}
							className="rounded-xl"
						>
							{t("quitDialog.keepInTray")}
						</Button>
					)}
					<Button
						disabled={stopping}
						onClick={stopAndGo}
						className="gap-1.5 rounded-xl bg-emerald-600 text-white hover:bg-emerald-500"
					>
						{stopping && <Spinner className="size-4" />}
						{t(`quitDialog.action_${mode}`)}
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	)
}
