import { AlertTriangle, Loader2, Trash2 } from "lucide-react"
import { useState } from "react"
import type { ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"

export interface DeleteServerDialogProps {
	server: ServerConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onConfirm: (serverId: string, deleteFiles: boolean) => Promise<void>
}

export default function DeleteServerDialog({
	server,
	open,
	onOpenChange,
	onConfirm,
}: DeleteServerDialogProps) {
	const [deleteFiles, setDeleteFiles] = useState(true)
	const [isDeleting, setIsDeleting] = useState(false)

	if (!server) return null

	const handleDelete = async () => {
		setIsDeleting(true)
		try {
			await onConfirm(server.id, deleteFiles)
			onOpenChange(false)
		} catch (err) {
			console.error("Failed to delete server:", err)
		} finally {
			setIsDeleting(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<div className="flex items-center gap-3">
						<div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-destructive/10 text-destructive">
							<AlertTriangle className="size-5" />
						</div>
						<div>
							<DialogTitle className="font-semibold text-base text-foreground">
								Delete Server
							</DialogTitle>
							<DialogDescription className="text-xs">
								Are you sure you want to delete <strong>{server.name}</strong>?
							</DialogDescription>
						</div>
					</div>
				</DialogHeader>

				<div className="flex flex-col gap-3 py-2">
					<div className="rounded-lg border border-border/40 bg-zinc-900/40 p-3 text-xs">
						<div className="font-medium text-foreground">Server Details:</div>
						<div className="mt-1 font-mono text-[11px] text-muted-foreground">
							Core: {server.core.toUpperCase()} {server.gameVersion} • Port: {server.port}
						</div>
					</div>

					<label
						htmlFor="delete-files-checkbox"
						className="flex cursor-pointer items-start gap-2.5 rounded-lg border border-border/40 bg-zinc-900/20 p-3 hover:bg-zinc-900/40"
					>
						<Checkbox
							id="delete-files-checkbox"
							checked={deleteFiles}
							onCheckedChange={(checked) => setDeleteFiles(Boolean(checked))}
							className="mt-0.5"
						/>
						<div className="flex flex-col gap-0.5">
							<span className="font-medium text-foreground text-xs">
								Permanently delete all server files
							</span>
							<span className="text-[11px] text-muted-foreground">
								Includes world maps, player data, plugins, and server logs from your computer disk.
							</span>
						</div>
					</label>
				</div>

				<DialogFooter className="gap-2 sm:gap-0">
					<Button
						variant="ghost"
						size="sm"
						onClick={() => onOpenChange(false)}
						disabled={isDeleting}
						className="text-xs"
					>
						Cancel
					</Button>
					<Button
						variant="destructive"
						size="sm"
						onClick={handleDelete}
						disabled={isDeleting}
						className="gap-1.5 text-xs"
					>
						{isDeleting ? (
							<>
								<Loader2 className="size-3.5 animate-spin" />
								<span>Deleting...</span>
							</>
						) : (
							<>
								<Trash2 className="size-3.5" />
								<span>Delete Server</span>
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
