import { useQueryClient } from "@tanstack/react-query"
import { AlertTriangle, Check, Package, Undo2 } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { CrashSuspect, VersionChangeCrash } from "@/bindings"
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/common/dialog"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { instanceService } from "@/services/instance-service"
import { rpc, serverService } from "@/services/server-service"

/**
 * Shown when the first start after a version change crashes: names the mods or plugins
 * the crash report blames and offers to turn them off or undo the whole change.
 * Nothing happens without a click.
 */
export default function VersionChangeCrashDialog() {
	const { t } = useTranslation()
	const queryClient = useQueryClient()
	const [crash, setCrash] = useState<VersionChangeCrash | null>(null)
	const [turnedOff, setTurnedOff] = useState<string[]>([])
	const [busy, setBusy] = useState<string | null>(null)
	const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null)

	useEffect(() => {
		let unlisten: (() => void) | undefined
		let cancelled = false
		rpc.on_version_change_crash
			.on((event: VersionChangeCrash) => {
				setCrash(event)
				setTurnedOff([])
				setMessage(null)
			})
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

	if (!crash) return null
	const isServer = crash.targetKind === "server"
	const target = isServer ? "server" : "game"
	// What the blamed files are: plugins, mods, or (nothing blamed on a server) either
	const addon =
		crash.suspects.every((s) => s.folder === "plugins") && crash.suspects.length > 0
			? "plugin"
			: crash.suspects.length > 0 || !isServer
				? "mod"
				: "either"

	const turnOff = async (suspect: CrashSuspect) => {
		setBusy(suspect.fileName)
		setMessage(null)
		try {
			await rpc.turn_off_after_version_change(
				crash.targetKind,
				crash.targetId,
				suspect.folder,
				suspect.fileName,
			)
			setTurnedOff((prev) => [...prev, suspect.fileName])
		} catch (e) {
			setMessage({ ok: false, text: String(e) })
		} finally {
			setBusy(null)
		}
	}

	const undo = async () => {
		setBusy("undo")
		setMessage(null)
		try {
			if (isServer) {
				await rpc.undo_server_version_change(crash.targetId)
				await serverService.refreshServers()
				await queryClient.invalidateQueries({ queryKey: ["server-version-backup", crash.targetId] })
			} else {
				await rpc.undo_instance_version_change(crash.targetId)
				await instanceService.refreshInstances()
			}
			setMessage({ ok: true, text: t("crashDialog.undone", { version: crash.fromGameVersion }) })
		} catch (e) {
			setMessage({ ok: false, text: String(e) })
		} finally {
			setBusy(null)
		}
	}

	const undone = message?.ok === true

	return (
		<Dialog open onOpenChange={(open) => !open && !busy && setCrash(null)}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-4 p-5 sm:max-w-lg">
				<div className="flex items-start gap-3">
					<AlertTriangle className="mt-0.5 size-5 shrink-0 text-warning" />
					<div className="flex flex-col gap-1">
						<DialogTitle className="text-base">
							{t("crashDialog.title", { name: crash.name, version: crash.toGameVersion })}
						</DialogTitle>
						<DialogDescription className="text-muted-foreground text-sm leading-relaxed">
							{crash.suspects.length > 0
								? t("crashDialog.suspects", { context: addon })
								: t("crashDialog.noSuspects", { context: addon })}
						</DialogDescription>
					</div>
				</div>

				{crash.suspects.length > 0 && (
					<div className="flex flex-col gap-1.5">
						{crash.suspects.map((suspect) => {
							const off = turnedOff.includes(suspect.fileName)
							return (
								<div
									key={`${suspect.folder}/${suspect.fileName}`}
									className="flex items-center gap-3 rounded-xl border border-border bg-card/40 px-3 py-2"
								>
									<Package className="size-4 shrink-0 text-muted-foreground" />
									<div className="min-w-0 flex-1">
										<p className="truncate font-medium text-foreground text-sm">{suspect.title}</p>
										<p className="truncate text-[11px] text-muted-foreground">{suspect.fileName}</p>
									</div>
									<Button
										size="sm"
										variant="outline"
										disabled={off || busy !== null || undone}
										onClick={() => turnOff(suspect)}
										className="h-8 gap-1.5 rounded-lg text-xs"
									>
										{busy === suspect.fileName ? (
											<Spinner className="size-3.5" />
										) : off ? (
											<Check className="size-3.5" />
										) : null}
										{off ? t("crashDialog.turnedOff") : t("crashDialog.turnOff")}
									</Button>
								</div>
							)
						})}
						{turnedOff.length > 0 && !undone && (
							<p className="text-muted-foreground text-xs">
								{t("crashDialog.tryAgain", { context: target, count: turnedOff.length })}
							</p>
						)}
					</div>
				)}

				{crash.excerpt && (
					<details className="rounded-xl border border-border bg-background/60 text-xs">
						<summary className="cursor-pointer px-3 py-2 text-muted-foreground">
							{t("crashDialog.details")}
						</summary>
						<pre className="max-h-48 overflow-auto whitespace-pre-wrap px-3 pb-3 font-mono text-[11px] text-muted-foreground">
							{crash.excerpt}
						</pre>
					</details>
				)}

				{message && (
					<p
						className={
							message.ok
								? "rounded-xl border border-primary/20 bg-primary/10 px-3 py-2 text-primary text-xs"
								: "rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-xs"
						}
					>
						{message.text}
					</p>
				)}

				<div className="flex flex-wrap justify-end gap-2">
					<Button
						variant="ghost"
						disabled={busy !== null}
						onClick={() => setCrash(null)}
						className="rounded-xl"
					>
						{undone ? t("common.close") : t("crashDialog.keepTrying")}
					</Button>
					{!undone && (
						<Button disabled={busy !== null} onClick={undo} className="gap-1.5 rounded-xl">
							{busy === "undo" ? <Spinner className="size-4" /> : <Undo2 className="size-4" />}
							{t("crashDialog.undo")}
						</Button>
					)}
				</div>
			</DialogContent>
		</Dialog>
	)
}
