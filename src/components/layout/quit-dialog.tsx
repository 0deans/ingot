import { getCurrentWindow } from "@tauri-apps/api/window"
import { Gamepad2, Moon, Server, TriangleAlert } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { QuitRequest } from "@/bindings"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog"
import { Spinner } from "@/components/ui/spinner"
import { quitService, useQuitPrompt } from "@/services/quit-service"
import { rpc } from "@/services/server-service"

/**
 * Asked when Ingot would close while servers run: their consoles live in Ingot, so
 * quitting would leave them running with no way to control them.
 */
export default function QuitDialog() {
	const { t } = useTranslation()
	const prompt = useQuitPrompt()
	const [stopping, setStopping] = useState(false)
	const [error, setError] = useState<string | null>(null)

	// A new prompt starts without the previous one's error
	// biome-ignore lint/correctness/useExhaustiveDependencies: reset on every new prompt
	useEffect(() => setError(null), [prompt])

	useEffect(() => {
		let unlisten: (() => void) | undefined
		let cancelled = false
		rpc.events.on_quit_requested
			.on((request: QuitRequest) => quitService.ask({ mode: "quit", request }))
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
				quitService.dismiss()
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
			quitService.dismiss()
		} catch (e) {
			setError(String(e))
		}
	}

	return (
		<Dialog open onOpenChange={(open) => !open && !stopping && quitService.dismiss()}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-4 p-5 sm:max-w-md">
				<div className="flex items-start gap-3">
					<TriangleAlert className="mt-0.5 size-5 shrink-0 text-warning" />
					<div className="flex flex-col gap-1">
						<DialogTitle className="text-base">{t(`quitDialog.title_${mode}`)}</DialogTitle>
						<DialogDescription className="text-muted-foreground text-sm leading-relaxed">
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
							className="flex items-center gap-3 rounded-xl border border-border bg-card/40 px-3 py-2"
						>
							{server.sleeping ? (
								<Moon className="size-4 shrink-0 text-muted-foreground" />
							) : (
								<Server className="size-4 shrink-0 text-primary" />
							)}
							<p className="min-w-0 flex-1 truncate font-medium text-foreground text-sm">
								{server.name}
							</p>
							<span className="shrink-0 text-2xs text-muted-foreground">
								{server.sleeping ? t("serverStatus.sleeping") : t("serverStatus.running")}
							</span>
						</div>
					))}
				</div>

				{stopping ? (
					<p className="flex items-center gap-2 text-muted-foreground text-xs">
						<Spinner className="size-3.5 shrink-0" />
						{running.length > 0
							? t("quitDialog.saving", { count: running.length })
							: t("quitDialog.stopping")}
					</p>
				) : (
					mode === "quit" &&
					request.games > 0 && (
						<p className="flex items-start gap-2 text-muted-foreground text-xs leading-relaxed">
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
						onClick={() => quitService.dismiss()}
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
					<Button disabled={stopping} onClick={stopAndGo} className="gap-1.5 rounded-xl">
						{stopping && <Spinner className="size-4" />}
						{t(`quitDialog.action_${mode}`)}
					</Button>
				</div>
			</DialogContent>
		</Dialog>
	)
}
