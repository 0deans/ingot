import { AlertTriangle, Trash2 } from "lucide-react"
import { useState } from "react"
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

interface DeleteInstanceDialogProps {
	instance: InstanceConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onConfirm: (instanceId: string) => Promise<void> | void
}

export const DeleteInstanceDialog = ({
	instance,
	open,
	onOpenChange,
	onConfirm,
}: DeleteInstanceDialogProps) => {
	const [isDeleting, setIsDeleting] = useState(false)

	if (!instance) return null

	const handleDelete = async () => {
		setIsDeleting(true)
		try {
			await onConfirm(instance.id)
			onOpenChange(false)
		} catch (e) {
			console.error("Failed to delete instance:", e)
		} finally {
			setIsDeleting(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-md border-border/60 bg-zinc-950 p-6 shadow-2xl backdrop-blur-2xl">
				<DialogHeader className="gap-2">
					<div className="flex size-11 items-center justify-center rounded-2xl border border-destructive/30 bg-destructive/10 text-destructive">
						<AlertTriangle className="size-5" />
					</div>
					<DialogTitle className="font-semibold text-lg text-white">Delete Instance</DialogTitle>
					<DialogDescription className="text-xs text-zinc-400">
						Are you sure you want to permanently delete this instance? This action cannot be undone.
					</DialogDescription>
				</DialogHeader>

				<div className="flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/50 p-3.5">
					<div className="flex size-10 shrink-0 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900">
						<LoaderIcon loader={instance.loader} size={22} />
					</div>
					<div className="flex min-w-0 flex-col">
						<span className="truncate font-semibold text-sm text-white">{instance.name}</span>
						<span className="text-[11px] text-zinc-400">
							{String(instance.loader).toUpperCase()} • {instance.gameVersion}
						</span>
					</div>
				</div>

				<p className="text-[11px] text-zinc-500">
					All saves, worlds, installed mods, resource packs, and configuration files stored in this
					instance folder will be deleted from disk.
				</p>

				<DialogFooter className="mt-2 flex items-center justify-end gap-2 border-border/30 border-t pt-4">
					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={() => onOpenChange(false)}
						disabled={isDeleting}
					>
						Cancel
					</Button>
					<Button
						type="button"
						variant="destructive"
						size="sm"
						onClick={handleDelete}
						disabled={isDeleting}
						className="gap-1.5 font-semibold"
					>
						<Trash2 className="size-3.5" />
						{isDeleting ? "Deleting..." : "Delete Instance"}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default DeleteInstanceDialog
