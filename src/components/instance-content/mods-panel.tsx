import { AlertTriangle, FolderOpen, PackagePlus, Search, Trash2 } from "lucide-react"
import { memo, useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { ContentListResult, InstalledContent } from "@/bindings"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import {
	instanceContentService,
	useBulkToggleContentMutation,
	useDeleteContentMutation,
	useToggleContentMutation,
} from "@/services/instance-content-service"
import ContentDetailsSheet from "./content-details-sheet"
import ContentRow from "./content-row"

export interface ModsPanelProps {
	instanceId: string
	data?: ContentListResult
	isLoading: boolean
	isInstanceRunning: boolean
	onOpenDiscoverMods?: () => void
}

type FilterState = "all" | "enabled" | "disabled" | "issues"

const ModsPanel = ({
	instanceId,
	data,
	isLoading,
	isInstanceRunning,
	onOpenDiscoverMods,
}: ModsPanelProps) => {
	// 1. Context & Global
	const { t } = useTranslation()

	// 2. Refs (none)

	// 3. State
	const [searchQuery, setSearchQuery] = useState("")
	const [filterState, setFilterState] = useState<FilterState>("all")
	const [selectedFileNames, setSelectedFileNames] = useState<Set<string>>(new Set())
	const [inspectingItem, setInspectingItem] = useState<InstalledContent | null>(null)
	const [deletingFileNames, setDeletingFileNames] = useState<string[] | null>(null)

	// 4. Mutations & Memoization
	const toggleMutation = useToggleContentMutation(instanceId, "mod")
	const bulkToggleMutation = useBulkToggleContentMutation(instanceId, "mod")
	const deleteMutation = useDeleteContentMutation(instanceId, "mod")

	const isBusy =
		toggleMutation.isPending || bulkToggleMutation.isPending || deleteMutation.isPending

	const items = useMemo(() => {
		return data?.items ?? []
	}, [data?.items])

	const dependencyIssues = data?.dependencyIssues
	const issuesMap = useMemo(() => {
		const map = new Map<string, string>()
		if (!dependencyIssues) {
			return map
		}
		for (const issue of dependencyIssues) {
			const text = issue.disabled
				? t("instanceContent.dependencyDisabled", { dep: issue.missingDependencyId })
				: t("instanceContent.dependencyMissing", { dep: issue.missingDependencyId })
			map.set(issue.modId.toLowerCase(), text)
		}
		return map
	}, [dependencyIssues, t])

	const filteredItems = useMemo(() => {
		const q = searchQuery.trim().toLowerCase()
		return items.filter((item) => {
			if (filterState === "enabled" && item.state !== "enabled") {
				return false
			}
			if (filterState === "disabled" && item.state !== "disabled") {
				return false
			}
			if (filterState === "issues") {
				const id = item.metadata.modId?.toLowerCase()
				if (!id || !issuesMap.has(id)) {
					return false
				}
			}
			if (q) {
				const nameMatch = item.metadata.name.toLowerCase().includes(q)
				const fileMatch = item.fileName.toLowerCase().includes(q)
				const idMatch = item.metadata.modId?.toLowerCase().includes(q)
				return nameMatch || fileMatch || Boolean(idMatch)
			}
			return true
		})
	}, [items, searchQuery, filterState, issuesMap])

	const allSelected = useMemo(() => {
		return (
			filteredItems.length > 0 &&
			filteredItems.every((item) => selectedFileNames.has(item.fileName))
		)
	}, [filteredItems, selectedFileNames])

	const handleToggleSelectAll = useCallback(() => {
		if (allSelected) {
			setSelectedFileNames(new Set())
		} else {
			const next = new Set<string>()
			for (const item of filteredItems) {
				next.add(item.fileName)
			}
			setSelectedFileNames(next)
		}
	}, [allSelected, filteredItems])

	const handleToggleSelect = useCallback((fileName: string) => {
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

	const handleToggleEnabled = useCallback(
		async (fileName: string, enabled: boolean) => {
			try {
				await toggleMutation.mutateAsync({ fileName, enabled })
			} catch (error) {
				toast.error(error instanceof Error ? error.message : String(error))
			}
		},
		[toggleMutation],
	)

	const handleBulkEnable = useCallback(
		async (enabled: boolean) => {
			const fileNames = Array.from(selectedFileNames)
			if (fileNames.length === 0) {
				return
			}
			try {
				await bulkToggleMutation.mutateAsync({ fileNames, enabled })
				toast.success(
					enabled
						? t("instanceContent.bulkEnabledSuccess", { count: fileNames.length })
						: t("instanceContent.bulkDisabledSuccess", { count: fileNames.length }),
				)
			} catch (error) {
				toast.error(error instanceof Error ? error.message : String(error))
			}
		},
		[selectedFileNames, bulkToggleMutation, t],
	)

	const handleRequestDelete = useCallback((fileName: string) => {
		setDeletingFileNames([fileName])
	}, [])

	const handleRequestDeleteSelected = useCallback(() => {
		const fileNames = Array.from(selectedFileNames)
		if (fileNames.length > 0) {
			setDeletingFileNames(fileNames)
		}
	}, [selectedFileNames])

	const handleConfirmDelete = useCallback(async () => {
		if (!deletingFileNames || deletingFileNames.length === 0) {
			return
		}
		try {
			await deleteMutation.mutateAsync(deletingFileNames)
			toast.success(t("instanceContent.deleteSuccess", { count: deletingFileNames.length }))
			setSelectedFileNames((prev) => {
				const next = new Set(prev)
				for (const f of deletingFileNames) {
					next.delete(f)
				}
				return next
			})
			setDeletingFileNames(null)
		} catch (error) {
			toast.error(error instanceof Error ? error.message : String(error))
		}
	}, [deletingFileNames, deleteMutation, t])

	const handleOpenFolder = useCallback(async () => {
		try {
			await instanceContentService.openContentFolder(instanceId, "mod")
		} catch (error) {
			toast.error(error instanceof Error ? error.message : String(error))
		}
	}, [instanceId])

	return (
		<div className="flex flex-1 flex-col gap-4">
			{/* Controls Header */}
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div className="flex flex-1 flex-wrap items-center gap-2">
					<div className="relative min-w-48 flex-1 sm:max-w-xs">
						<Search className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
						<Input
							placeholder={t("instanceContent.searchMods")}
							value={searchQuery}
							onChange={(e) => setSearchQuery(e.target.value)}
							className="pl-8 text-xs"
						/>
					</div>

					<Select value={filterState} onValueChange={(val) => setFilterState(val as FilterState)}>
						<SelectTrigger className="w-36 text-xs">
							<SelectValue placeholder={t("instanceContent.filterAll")} />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="all">{t("instanceContent.filterAll")}</SelectItem>
							<SelectItem value="enabled">{t("instanceContent.filterEnabled")}</SelectItem>
							<SelectItem value="disabled">{t("instanceContent.filterDisabled")}</SelectItem>
							<SelectItem value="issues">{t("instanceContent.filterIssues")}</SelectItem>
						</SelectContent>
					</Select>
				</div>

				<div className="flex items-center gap-2">
					<Button variant="outline" size="sm" onClick={handleOpenFolder} className="gap-2 text-xs">
						<FolderOpen className="size-4" />
						<span>{t("instanceContent.openFolder")}</span>
					</Button>

					{onOpenDiscoverMods && (
						<Button size="sm" onClick={onOpenDiscoverMods} className="gap-2 text-xs">
							<PackagePlus className="size-4" />
							<span>{t("instanceContent.discoverMods")}</span>
						</Button>
					)}
				</div>
			</div>

			{/* Bulk Actions Banner */}
			{selectedFileNames.size > 0 && (
				<div className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-primary/30 bg-primary/5 p-2.5 px-3">
					<span className="text-xs font-medium text-foreground">
						{t("instanceContent.selectedCount", { count: selectedFileNames.size })}
					</span>
					<div className="flex items-center gap-2">
						<Button
							variant="outline"
							size="sm"
							onClick={() => handleBulkEnable(true)}
							disabled={isBusy || isInstanceRunning}
							className="text-xs"
						>
							{t("instanceContent.enableSelected")}
						</Button>
						<Button
							variant="outline"
							size="sm"
							onClick={() => handleBulkEnable(false)}
							disabled={isBusy || isInstanceRunning}
							className="text-xs"
						>
							{t("instanceContent.disableSelected")}
						</Button>
						<Button
							variant="destructive"
							size="sm"
							onClick={handleRequestDeleteSelected}
							disabled={isBusy || isInstanceRunning}
							className="gap-1.5 text-xs"
						>
							<Trash2 className="size-3.5" />
							<span>{t("common.delete")}</span>
						</Button>
					</div>
				</div>
			)}

			{/* Dependency Issues Alert Banner */}
			{data?.dependencyIssues && data.dependencyIssues.length > 0 && (
				<div className="flex items-center gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-xs text-amber-600 dark:text-amber-400">
					<AlertTriangle className="size-4 shrink-0" />
					<div className="flex-1">
						<span className="font-semibold">
							{t("instanceContent.dependencyIssuesDetected", {
								count: data.dependencyIssues.length,
							})}
						</span>
						<span className="ml-1 text-muted-foreground">
							{t("instanceContent.dependencyIssuesHelp")}
						</span>
					</div>
				</div>
			)}

			{/* Mod List Content */}
			{isLoading ? (
				<div className="flex flex-1 items-center justify-center p-12">
					<Spinner className="size-8" />
				</div>
			) : filteredItems.length === 0 ? (
				<Empty className="flex-1 border border-dashed border-border/60 p-8">
					<EmptyMedia variant="icon">
						<PackagePlus className="size-8 text-muted-foreground" />
					</EmptyMedia>
					<EmptyHeader>
						<EmptyTitle>{t("instanceContent.noModsFound")}</EmptyTitle>
						<EmptyDescription>
							{searchQuery
								? t("instanceContent.noModsMatchingFilter")
								: t("instanceContent.noModsInstalledDescription")}
						</EmptyDescription>
					</EmptyHeader>
				</Empty>
			) : (
				<div className="flex flex-col gap-2">
					<div className="flex items-center justify-between px-1 text-xs text-muted-foreground">
						<button type="button" onClick={handleToggleSelectAll} className="hover:text-foreground">
							{allSelected ? t("instanceContent.deselectAll") : t("instanceContent.selectAll")}
						</button>
						<span>{t("instanceContent.itemsCount", { count: filteredItems.length })}</span>
					</div>

					<ScrollArea className="flex-1">
						<div className="flex flex-col gap-2 pr-2">
							{filteredItems.map((item) => {
								const modId = item.metadata.modId?.toLowerCase()
								const issueText = modId ? issuesMap.get(modId) : undefined

								return (
									<ContentRow
										key={item.fileName}
										item={item}
										isSelected={selectedFileNames.has(item.fileName)}
										isBusy={isBusy}
										isInstanceRunning={isInstanceRunning}
										hasDependencyIssue={Boolean(issueText)}
										dependencyIssueText={issueText}
										onToggleSelect={handleToggleSelect}
										onToggleEnabled={handleToggleEnabled}
										onOpenDetails={setInspectingItem}
										onDelete={handleRequestDelete}
									/>
								)
							})}
						</div>
					</ScrollArea>
				</div>
			)}

			{/* Details Sheet */}
			<ContentDetailsSheet
				item={inspectingItem}
				isOpen={Boolean(inspectingItem)}
				onClose={() => setInspectingItem(null)}
			/>

			{/* Delete Confirmation Dialog */}
			<Dialog
				open={Boolean(deletingFileNames)}
				onOpenChange={(open) => {
					if (!open) {
						setDeletingFileNames(null)
					}
				}}
			>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>{t("instanceContent.deleteConfirmTitle")}</DialogTitle>
						<DialogDescription>
							{t("instanceContent.deleteConfirmDescription", {
								count: deletingFileNames?.length ?? 0,
							})}
						</DialogDescription>
					</DialogHeader>

					{deletingFileNames && (
						<div className="max-h-40 overflow-y-auto rounded-md border border-border/50 bg-muted/30 p-2 font-mono text-xs">
							{deletingFileNames.map((name) => {
								return <div key={name}>{name}</div>
							})}
						</div>
					)}

					<DialogFooter>
						<Button variant="outline" onClick={() => setDeletingFileNames(null)} disabled={isBusy}>
							{t("common.cancel")}
						</Button>
						<Button variant="destructive" onClick={handleConfirmDelete} disabled={isBusy}>
							{t("common.delete")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	)
}

ModsPanel.displayName = "ModsPanel"

export default memo(ModsPanel)
