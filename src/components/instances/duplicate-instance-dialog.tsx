import { Copy, Loader2 } from "lucide-react"
import { useEffect, useState } from "react"
import type { InstanceConfig } from "@/bindings"
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
import { Input } from "@/components/ui/input"
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
	const [name, setName] = useState("")
	const [includeWorlds, setIncludeWorlds] = useState(true)
	const [worldCount, setWorldCount] = useState<number | null>(null)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const instanceId = instance?.id
	const instanceName = instance?.name
	useEffect(() => {
		if (!open || !instanceId) return
		setName(`${instanceName} (copy)`)
		setIncludeWorlds(true)
		setError(null)
		setWorldCount(null)
		rpc
			.get_instance_worlds(instanceId)
			.then((worlds) => setWorldCount(worlds.length))
			.catch(() => setWorldCount(null))
	}, [open, instanceId, instanceName])

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
						Duplicate Instance
					</DialogTitle>
					<DialogDescription className="text-xs text-zinc-400">
						Makes a separate copy with the same mods, configs, resource packs and settings. Changes
						to one don't touch the other, so the copy is a safe place to try a new version or mods.
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
					<label htmlFor="duplicate-name" className="font-medium text-xs text-zinc-300">
						Name of the copy
					</label>
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

				<label className="flex cursor-pointer items-start gap-3 rounded-xl border border-zinc-800 bg-zinc-900/40 p-3">
					<input
						type="checkbox"
						checked={includeWorlds}
						onChange={(e) => setIncludeWorlds(e.target.checked)}
						disabled={busy}
						className="mt-0.5 size-4 accent-emerald-500"
					/>
					<span className="flex flex-col gap-0.5">
						<span className="font-medium text-sm text-zinc-100">
							Copy worlds
							{worldCount !== null && (
								<span className="font-normal text-zinc-500">
									{" "}
									({worldCount === 0 ? "none yet" : worldCount})
								</span>
							)}
						</span>
						<span className="text-[11px] text-zinc-500">
							Without them the copy starts with no singleplayer worlds.
						</span>
					</span>
				</label>

				<p className="text-[11px] text-zinc-500">
					Playtime starts at zero. Logs, crash reports and screenshots stay with the original.
				</p>

				{blocked && (
					<p className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-amber-200 text-xs">
						The game is running. Close it first so the worlds are copied as they were saved, or copy
						without worlds.
					</p>
				)}
				{error && (
					<p className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-red-200 text-xs">
						{error}
					</p>
				)}

				<DialogFooter>
					<Button
						type="button"
						onClick={handleDuplicate}
						disabled={busy || blocked || !name.trim()}
						className="w-full gap-1.5 bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
					>
						{busy ? <Loader2 className="size-4 animate-spin" /> : <Copy className="size-4" />}
						{busy ? "Copying…" : "Duplicate"}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default DuplicateInstanceDialog
