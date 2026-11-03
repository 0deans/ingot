import { useQuery, useQueryClient } from "@tanstack/react-query"
import { Archive, Clock, HardDrive, Trash2 } from "lucide-react"
import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { WorldBackupInfo, WorldSummary } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import { formatBytes, formatDateTime } from "@/lib/format"
import { worldService } from "@/services/world-service"

interface WorldBackupsDialogProps {
	world: WorldSummary | null
	open: boolean
	onOpenChange: (open: boolean) => void
}

const WorldBackupsDialog = ({ world, open, onOpenChange }: WorldBackupsDialogProps) => {
	const { t } = useTranslation()
	const queryClient = useQueryClient()

	const [deletingFile, setDeletingFile] = useState<string | null>(null)

	const backupsQuery = useQuery<WorldBackupInfo[]>({
		queryKey: ["world-backups", world?.instanceId, world?.folderName],
		queryFn: () => {
			if (!world) {
				return []
			}
			return worldService.listBackups(world.instanceId, world.folderName)
		},
		enabled: open && !!world,
	})

	const backups = backupsQuery.data ?? []
	const isLoading = backupsQuery.isLoading

	const handleDelete = useCallback(
		async (fileName: string) => {
			if (!world) {
				return
			}
			try {
				setDeletingFile(fileName)
				await worldService.deleteBackup(world.instanceId, fileName)
				toast.success(t("worlds.deleteSuccess"))
				await queryClient.invalidateQueries({
					queryKey: ["world-backups", world.instanceId, world.folderName],
				})
			} catch (error) {
				console.error("Failed to delete backup:", error)
				const message = error instanceof Error ? error.message : String(error)
				toast.error(message)
			} finally {
				setDeletingFile(null)
			}
		},
		[world, t, queryClient],
	)

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-lg">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<Archive className="size-5 shrink-0 text-primary" />
						{world && t("worlds.backupsTitle", { name: world.displayName })}
					</DialogTitle>
					<DialogDescription>{world?.instanceName}</DialogDescription>
				</DialogHeader>

				<div className="py-2">
					{isLoading ? (
						<div className="flex h-36 items-center justify-center">
							<Spinner className="size-6 text-muted-foreground" />
						</div>
					) : backups.length === 0 ? (
						<div className="flex h-36 flex-col items-center justify-center gap-2 text-center text-sm text-muted-foreground">
							<Archive className="size-8 stroke-[1.5] text-muted-foreground/60" />
							<p>{t("worlds.noBackups")}</p>
						</div>
					) : (
						<ScrollArea className="max-h-72">
							<div className="space-y-2 pr-3">
								{backups.map((backup) => {
									const isDeleting = deletingFile === backup.fileName
									return (
										<div
											key={backup.fileName}
											className="flex items-center justify-between rounded-lg border border-border/40 bg-muted/20 p-3 transition-colors hover:bg-muted/40"
										>
											<div className="min-w-0 flex-1 space-y-1">
												<p className="truncate font-mono text-xs font-medium text-foreground">
													{backup.fileName}
												</p>
												<div className="flex items-center gap-3 text-xs text-muted-foreground">
													<span className="flex items-center gap-1">
														<Clock className="size-3" />
														{backup.createdAt > 0
															? formatDateTime(backup.createdAt * 1000)
															: "Unknown"}
													</span>
													<span className="flex items-center gap-1">
														<HardDrive className="size-3" />
														{formatBytes(backup.sizeBytes)}
													</span>
												</div>
											</div>

											<Button
												size="icon-sm"
												variant="ghost"
												className="text-muted-foreground hover:text-destructive"
												onClick={() => {
													return handleDelete(backup.fileName)
												}}
												disabled={isDeleting}
												title={t("worlds.deleteBackup")}
											>
												{isDeleting ? (
													<Spinner className="size-3.5" />
												) : (
													<Trash2 className="size-3.5" />
												)}
											</Button>
										</div>
									)
								})}
							</div>
						</ScrollArea>
					)}
				</div>

				<DialogFooter>
					<Button
						variant="outline"
						onClick={() => {
							return onOpenChange(false)
						}}
					>
						Close
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

WorldBackupsDialog.displayName = "WorldBackupsDialog"

export default WorldBackupsDialog
