import { AlertTriangle, Trash2 } from "lucide-react"
import { useState } from "react"
import { Trans, useTranslation } from "react-i18next"
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
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"

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
	const { t } = useTranslation()
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
								{t("deleteServer.title")}
							</DialogTitle>
							<DialogDescription className="text-xs">
								<Trans
									i18nKey="deleteServer.confirm"
									values={{ name: server.name }}
									components={{ b: <strong /> }}
								/>
							</DialogDescription>
						</div>
					</div>
				</DialogHeader>

				<div className="flex flex-col gap-3 py-2">
					<div className="rounded-lg border border-border/40 bg-card/40 p-3 text-xs">
						<div className="font-medium text-foreground">{t("deleteServer.details")}</div>
						<div className="mt-1 font-mono text-2xs text-muted-foreground">
							{t("deleteServer.detailsLine", {
								core: server.core.toUpperCase(),
								version: server.gameVersion,
								port: server.port,
							})}
						</div>
					</div>

					<Label
						htmlFor="delete-files-checkbox"
						className="flex cursor-pointer items-start gap-2.5 rounded-lg border border-border/40 bg-card/20 p-3 hover:bg-card/40"
					>
						<Checkbox
							id="delete-files-checkbox"
							checked={deleteFiles}
							onCheckedChange={(checked) => setDeleteFiles(Boolean(checked))}
							className="mt-0.5"
						/>
						<div className="flex flex-col gap-0.5">
							<span className="font-medium text-foreground text-xs">
								{t("deleteServer.deleteFiles")}
							</span>
							<span className="text-2xs text-muted-foreground">
								{t("deleteServer.deleteFilesDesc")}
							</span>
						</div>
					</Label>
				</div>

				<DialogFooter className="gap-2 sm:gap-0">
					<Button
						variant="ghost"
						size="sm"
						onClick={() => onOpenChange(false)}
						disabled={isDeleting}
						className="text-xs"
					>
						{t("common.cancel")}
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
								<Spinner className="size-3.5" />
								<span>{t("common.deleting")}</span>
							</>
						) : (
							<>
								<Trash2 className="size-3.5" />
								<span>{t("deleteServer.title")}</span>
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
