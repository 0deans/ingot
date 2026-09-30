import { AlertTriangle, Trash2 } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
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
	const { t } = useTranslation()
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
			<DialogContent className="sm:max-w-md">
				<DialogHeader className="gap-2">
					<div className="flex size-11 items-center justify-center rounded-2xl border border-destructive/30 bg-destructive/10 text-destructive">
						<AlertTriangle className="size-5" />
					</div>
					<DialogTitle className="font-semibold text-foreground text-lg">
						{t("deleteInstance.title")}
					</DialogTitle>
					<DialogDescription className="text-muted-foreground text-xs">
						{t("deleteInstance.description")}
					</DialogDescription>
				</DialogHeader>

				<div className="flex items-center gap-3 rounded-xl border border-border bg-card/50 p-3.5">
					<div className="flex size-10 shrink-0 items-center justify-center rounded-lg border border-border bg-card">
						<LoaderIcon loader={instance.loader} size={22} />
					</div>
					<div className="flex min-w-0 flex-col">
						<span className="truncate font-semibold text-foreground text-sm">{instance.name}</span>
						<span className="text-2xs text-muted-foreground">
							{String(instance.loader).toUpperCase()} • {instance.gameVersion}
						</span>
					</div>
				</div>

				<p className="text-2xs text-muted-foreground">{t("deleteInstance.note")}</p>

				<DialogFooter>
					<Button
						type="button"
						variant="destructive"
						onClick={handleDelete}
						disabled={isDeleting}
						className="w-full gap-1.5 font-semibold"
					>
						<Trash2 className="size-3.5" />
						{isDeleting ? t("common.deleting") : t("deleteInstance.title")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default DeleteInstanceDialog
