import { AlertTriangle, RefreshCw } from "lucide-react"
import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { WorldDimension, WorldSummary } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Spinner } from "@/components/ui/spinner"
import { worldService } from "@/services/world-service"

interface DimensionResetDialogProps {
	world: WorldSummary | null
	dimension: WorldDimension | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onSuccess: () => void
}

const DimensionResetDialog = ({
	world,
	dimension,
	open,
	onOpenChange,
	onSuccess,
}: DimensionResetDialogProps) => {
	const { t } = useTranslation()

	const [isResetting, setIsResetting] = useState(false)

	const dimensionLabel = dimension === "nether" ? "Nether (DIM-1)" : "The End (DIM1)"

	const handleConfirm = useCallback(async () => {
		if (!world || !dimension) {
			return
		}
		try {
			setIsResetting(true)
			await worldService.resetDimension(world.instanceId, world.folderName, dimension)
			toast.success(t("worlds.resetSuccess"))
			onOpenChange(false)
			onSuccess()
		} catch (error) {
			console.error("Failed to reset dimension:", error)
			const message = error instanceof Error ? error.message : String(error)
			toast.error(message)
		} finally {
			setIsResetting(false)
		}
	}, [world, dimension, onOpenChange, onSuccess, t])

	const handleCancel = useCallback(() => {
		if (!isResetting) {
			onOpenChange(false)
		}
	}, [isResetting, onOpenChange])

	return (
		<Dialog open={open} onOpenChange={handleCancel}>
			<DialogContent className="max-w-md">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2 text-destructive">
						<AlertTriangle className="size-5 shrink-0" />
						{t("worlds.resetDimensionTitle")}
					</DialogTitle>
					<DialogDescription>
						{world &&
							t("worlds.resetDimensionConfirm", {
								dimension: dimensionLabel,
								name: world.displayName,
							})}
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-3 py-2 text-sm text-muted-foreground">
					<p>{t("worlds.resetDimensionWarning")}</p>
					<div className="rounded-lg border border-border/50 bg-muted/30 p-3 text-xs">
						{t("worlds.resetDimensionTrashNote")}
					</div>
				</div>

				<DialogFooter>
					<Button variant="outline" onClick={handleCancel} disabled={isResetting}>
						Cancel
					</Button>
					<Button
						variant="destructive"
						onClick={handleConfirm}
						disabled={isResetting || !world || !dimension}
					>
						{isResetting ? (
							<>
								<Spinner className="mr-2 size-4" />
								{t("worlds.resetting")}
							</>
						) : (
							<>
								<RefreshCw className="mr-2 size-4" />
								{t("worlds.resetDimensionTitle")}
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

DimensionResetDialog.displayName = "DimensionResetDialog"

export default DimensionResetDialog
