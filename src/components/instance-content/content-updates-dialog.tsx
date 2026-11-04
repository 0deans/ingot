import { ArrowRight, CheckCircle2, RefreshCw } from "lucide-react"
import { memo, useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { ContentUpdateCandidate } from "@/bindings"
import { Badge } from "@/components/ui/badge"
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
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import { useApplyUpdatesMutation } from "@/services/instance-content-service"

export interface ContentUpdatesDialogProps {
	instanceId: string
	isOpen: boolean
	onClose: () => void
	candidates: ContentUpdateCandidate[]
	onApplied?: () => void
}

const ContentUpdatesDialog = ({
	instanceId,
	isOpen,
	onClose,
	candidates,
	onApplied,
}: ContentUpdatesDialogProps) => {
	// 1. Context & Global
	const { t } = useTranslation()

	// 2. Refs (none)

	// 3. State
	const [selectedFileNames, setSelectedFileNames] = useState<Set<string>>(() => {
		return new Set(
			candidates.map((candidate) => {
				return candidate.fileName
			}),
		)
	})

	// 4. Memoization & Mutations
	const applyMutation = useApplyUpdatesMutation(instanceId, "mod")

	const selectedCandidates = useMemo(() => {
		return candidates.filter((candidate) => {
			return selectedFileNames.has(candidate.fileName)
		})
	}, [candidates, selectedFileNames])

	const isAllSelected = candidates.length > 0 && selectedFileNames.size === candidates.length

	const handleToggleSelectAll = useCallback(() => {
		if (isAllSelected) {
			setSelectedFileNames(new Set())
		} else {
			setSelectedFileNames(
				new Set(
					candidates.map((c) => {
						return c.fileName
					}),
				),
			)
		}
	}, [isAllSelected, candidates])

	const handleToggleItem = useCallback((fileName: string) => {
		setSelectedFileNames((prev) => {
			const next = new Set(prev)
			if (next.has(fileName)) {
				next.delete(fileName)
			} else {
				next.add(fileName)
			}
			return next
		})
	}, [])

	const handleApply = useCallback(async () => {
		if (selectedCandidates.length === 0) {
			return
		}
		try {
			const report = await applyMutation.mutateAsync(selectedCandidates)
			if (report.failed.length > 0) {
				toast.error(`${t("instanceContent.updateFailed")}: ${report.failed.join(", ")}`)
			} else if (report.warnings.length > 0) {
				toast.warning(
					`${t("instanceContent.updateSuccess", { count: report.updated.length })} (${report.warnings.length} warnings)`,
				)
			} else {
				toast.success(t("instanceContent.updateSuccess", { count: report.updated.length }))
			}
			onApplied?.()
			onClose()
		} catch (error) {
			toast.error(error instanceof Error ? error.message : String(error))
		}
	}, [selectedCandidates, applyMutation, onApplied, onClose, t])

	const renderChannelBadge = useCallback(
		(channel: string) => {
			const lower = channel.toLowerCase()
			if (lower === "release") {
				return (
					<Badge
						variant="outline"
						className="border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
					>
						{t("instanceContent.channelRelease")}
					</Badge>
				)
			}
			if (lower === "beta") {
				return (
					<Badge
						variant="outline"
						className="border-blue-500/30 bg-blue-500/10 text-blue-600 dark:text-blue-400"
					>
						{t("instanceContent.channelBeta")}
					</Badge>
				)
			}
			return (
				<Badge
					variant="outline"
					className="border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-400"
				>
					{t("instanceContent.channelAlpha")}
				</Badge>
			)
		},
		[t],
	)

	return (
		<Dialog open={isOpen} onOpenChange={onClose}>
			<DialogContent className="flex max-h-[85vh] w-full flex-col sm:max-w-2xl">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<RefreshCw className="size-5 text-primary" />
						<span>{t("instanceContent.updatesDialogTitle")}</span>
					</DialogTitle>
					<DialogDescription>{t("instanceContent.updatesDialogDescription")}</DialogDescription>
				</DialogHeader>

				<div className="flex items-center justify-between border-b pb-2 text-xs">
					<Button
						variant="ghost"
						size="sm"
						onClick={handleToggleSelectAll}
						className="h-7 text-xs text-muted-foreground hover:text-foreground"
					>
						{isAllSelected ? t("instanceContent.deselectAll") : t("instanceContent.selectAll")}
					</Button>
					<span className="text-muted-foreground">
						{t("instanceContent.selectedCount", { count: selectedCandidates.length })}
					</span>
				</div>

				<ScrollArea className="flex-1 pr-3">
					<div className="flex flex-col divide-y divide-border/40">
						{candidates.map((candidate) => {
							const isSelected = selectedFileNames.has(candidate.fileName)
							const sourceName = candidate.source === "modrinth" ? "Modrinth" : "CurseForge"

							return (
								<div
									key={candidate.fileName}
									className="flex items-center gap-3 py-3 transition-colors hover:bg-muted/40"
								>
									<Checkbox
										checked={isSelected}
										onCheckedChange={() => {
											handleToggleItem(candidate.fileName)
										}}
									/>

									<div className="min-w-0 flex-1">
										<div className="flex items-center gap-2">
											<span className="truncate text-sm font-medium text-foreground">
												{candidate.title}
											</span>
											{renderChannelBadge(candidate.channel)}
											<Badge variant="outline" className="text-[10px]">
												{sourceName}
											</Badge>
										</div>

										<div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
											<span className="truncate">
												{candidate.currentVersion ?? candidate.fileName}
											</span>
											<ArrowRight className="size-3 shrink-0" />
											<span className="font-medium text-foreground">{candidate.latestVersion}</span>
										</div>
									</div>
								</div>
							)
						})}
					</div>
				</ScrollArea>

				<DialogFooter className="mt-2 flex items-center justify-between gap-2 sm:justify-between">
					<Button variant="outline" size="sm" onClick={onClose} disabled={applyMutation.isPending}>
						{t("common.cancel")}
					</Button>
					<Button
						size="sm"
						onClick={handleApply}
						disabled={selectedCandidates.length === 0 || applyMutation.isPending}
						className="gap-2"
					>
						{applyMutation.isPending ? (
							<>
								<Spinner className="size-4" />
								<span>{t("instanceContent.updating")}</span>
							</>
						) : (
							<>
								<CheckCircle2 className="size-4" />
								<span>
									{t("instanceContent.updateSelected", { count: selectedCandidates.length })}
								</span>
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

ContentUpdatesDialog.displayName = "ContentUpdatesDialog"

export default memo(ContentUpdatesDialog)
