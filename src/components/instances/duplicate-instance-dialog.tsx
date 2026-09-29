import { Copy } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { InstanceConfig } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/common/dialog"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
import { instanceService, rpc } from "@/services/instance-service"

interface DuplicateInstanceDialogProps {
	instance: InstanceConfig | null
	open: boolean
	isRunning: boolean
	onOpenChange: (open: boolean) => void
}

/** Copies an instance into a new, independent one, e.g. to try a new version or modpack */
export const DuplicateInstanceDialog = ({
	instance,
	open,
	isRunning,
	onOpenChange,
}: DuplicateInstanceDialogProps) => {
	const { t } = useTranslation()
	const [name, setName] = useState("")
	const [includeWorlds, setIncludeWorlds] = useState(true)
	const [worldCount, setWorldCount] = useState<number | null>(null)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const instanceId = instance?.id
	const instanceName = instance?.name
	useEffect(() => {
		if (!open || !instanceId) return
		setName(t("duplicateInstance.defaultName", { name: instanceName }))
		setIncludeWorlds(true)
		setError(null)
		setWorldCount(null)
		rpc
			.get_instance_worlds(instanceId)
			.then((worlds) => setWorldCount(worlds.length))
			.catch(() => setWorldCount(null))
	}, [open, instanceId, instanceName, t])

	if (!instance) return null

	const copyingWorlds = includeWorlds && worldCount !== 0
	const blocked = copyingWorlds && isRunning

	const handleDuplicate = async () => {
		setBusy(true)
		setError(null)
		try {
			await instanceService.duplicateInstance(instance.id, name, includeWorlds)
			onOpenChange(false)
		} catch (e) {
			setError(String(e))
		} finally {
			setBusy(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={(next) => !busy && onOpenChange(next)}>
			<DialogContent className="border-border/60 bg-zinc-950 sm:max-w-md">
				<DialogHeader className="gap-2">
					<div className="flex size-11 items-center justify-center rounded-2xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-400">
						<Copy className="size-5" />
					</div>
					<DialogTitle className="font-semibold text-lg text-zinc-50">
						{t("duplicateInstance.title")}
					</DialogTitle>
					<DialogDescription className="text-xs text-zinc-400">
						{t("duplicateInstance.description")}
					</DialogDescription>
				</DialogHeader>

				<div className="flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/50 p-3.5">
					<div className="flex size-10 shrink-0 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900">
						<LoaderIcon loader={instance.loader} size={22} />
					</div>
					<div className="flex min-w-0 flex-col">
						<span className="truncate font-semibold text-sm text-zinc-50">{instance.name}</span>
						<span className="text-[11px] text-zinc-400">
							{String(instance.loader).toUpperCase()} • {instance.gameVersion}
						</span>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					<Label htmlFor="duplicate-name" className="font-medium text-xs text-zinc-300">
						{t("duplicateInstance.nameLabel")}
					</Label>
					<Input
						id="duplicate-name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === "Enter" && name.trim() && !busy && !blocked) handleDuplicate()
						}}
						disabled={busy}
						autoFocus
					/>
				</div>

				<Label className="flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3">
					<input
						type="checkbox"
						checked={includeWorlds}
						onChange={(e) => setIncludeWorlds(e.target.checked)}
						disabled={busy}
						className="mt-0.5 size-4 accent-emerald-500"
					/>
					<span className="flex flex-col gap-0.5">
						<span className="font-medium text-sm text-zinc-100">
							{t("duplicateInstance.copyWorlds")}
							{worldCount !== null && (
								<span className="font-normal text-zinc-500">
									{" "}
									({worldCount === 0 ? t("duplicateInstance.noWorldsYet") : worldCount})
								</span>
							)}
						</span>
						<span className="text-[11px] text-zinc-500">
							{t("duplicateInstance.copyWorldsDesc")}
						</span>
					</span>
				</Label>

				<p className="text-[11px] text-zinc-500">{t("duplicateInstance.note")}</p>

				{blocked && (
					<Alert className={alertTone.warning}>
						<AlertDescription>{t("duplicateInstance.running")}</AlertDescription>
					</Alert>
				)}
				{error && (
					<Alert variant="destructive">
						<AlertDescription>{error}</AlertDescription>
					</Alert>
				)}

				<DialogFooter>
					<Button
						type="button"
						onClick={handleDuplicate}
						disabled={busy || blocked || !name.trim()}
						className="w-full gap-1.5 bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
					>
						{busy ? <Spinner className="size-4" /> : <Copy className="size-4" />}
						{busy ? t("duplicateInstance.copying") : t("duplicateInstance.action")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default DuplicateInstanceDialog
