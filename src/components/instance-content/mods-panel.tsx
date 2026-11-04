import {
	AlertTriangle,
	Compass,
	Layers,
	PackagePlus,
	RefreshCw,
	Search,
	Sparkles,
	Trash2,
	X,
} from "lucide-react"
import { memo, useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { ContentListResult, ContentUpdateCandidate, InstalledContent } from "@/bindings"
import { supportsMods } from "@/components/content/install-dialog"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import {
	useBulkToggleContentMutation,
	useCheckUpdatesMutation,
	useDeleteContentMutation,
	useIdentifyContentMutation,
	useToggleContentMutation,
} from "@/services/instance-content-service"
import ContentDetailsSheet from "./content-details-sheet"
import ContentRow from "./content-row"
import ContentUpdatesDialog from "./content-updates-dialog"

export interface ModsPanelProps {
	instanceId: string
	loader?: string
	data?: ContentListResult
	isLoading: boolean
	isInstanceRunning: boolean
	onOpenDiscoverMods?: () => void
	onSwitchTab?: (tab: "mods" | "resourcepacks" | "shaders" | "datapacks") => void
}

type FilterState = "all" | "enabled" | "disabled" | "issues"

const ModsPanel = ({
	instanceId,
	loader,
	data,
	isLoading,
	isInstanceRunning,
	onOpenDiscoverMods,
	onSwitchTab,
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
	const [updateCandidates, setUpdateCandidates] = useState<ContentUpdateCandidate[] | null>(null)
	const [isUpdatesDialogOpen, setIsUpdatesDialogOpen] = useState(false)

	// 4. Mutations & Memoization
	const toggleMutation = useToggleContentMutation(instanceId, "mod")
	const bulkToggleMutation = useBulkToggleContentMutation(instanceId, "mod")
	const deleteMutation = useDeleteContentMutation(instanceId, "mod")
	const identifyMutation = useIdentifyContentMutation(instanceId, "mod")
	const checkUpdatesMutation = useCheckUpdatesMutation(instanceId, "mod")

	const isBusy =
		toggleMutation.isPending ||
		bulkToggleMutation.isPending ||
		deleteMutation.isPending ||
		identifyMutation.isPending ||
		checkUpdatesMutation.isPending

	const items = useMemo(() => {
		return data?.items ?? []
	}, [data?.items])

	const dependencyIssues = data?.dependencyIssues
	const isLoaderSupported = loader ? supportsMods(loader) : true
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

	const counts = useMemo(() => {
		let enabledCount = 0
		let disabledCount = 0
		let issuesCount = 0
		for (const item of items) {
			if (item.state === "enabled") {
				enabledCount += 1
			} else if (item.state === "disabled") {
				disabledCount += 1
			}
			const id = item.metadata.modId?.toLowerCase()
			if (id && issuesMap.has(id)) {
				issuesCount += 1
			}
		}
		return {
			all: items.length,
			enabled: enabledCount,
			disabled: disabledCount,
			issues: issuesCount,
		}
	}, [items, issuesMap])

	const filterLabels: Record<FilterState, string> = useMemo(() => {
		return {
			all: t("instanceContent.filterAll"),
			enabled: t("instanceContent.filterEnabled"),
			disabled: t("instanceContent.filterDisabled"),
			issues: t("instanceContent.filterIssues"),
		}
	}, [t])

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

	const handleIdentify = useCallback(async () => {
		try {
			const result = await identifyMutation.mutateAsync()
			if (result.identifiedCount > 0) {
				toast.success(
					t("instanceContent.identifySuccess", {
						count: result.identifiedCount,
						total: result.totalCount,
					}),
				)
			} else {
				toast.info(t("instanceContent.identifyNoNew"))
			}
		} catch (error) {
			toast.error(error instanceof Error ? error.message : String(error))
		}
	}, [identifyMutation, t])

	const handleCheckUpdates = useCallback(async () => {
		try {
			const result = await checkUpdatesMutation.mutateAsync()
			if (result.updates.length === 0) {
				toast.info(t("instanceContent.noUpdatesFound"))
			} else {
				setUpdateCandidates(result.updates)
				setIsUpdatesDialogOpen(true)
			}
		} catch (error) {
			toast.error(error instanceof Error ? error.message : String(error))
		}
	}, [checkUpdatesMutation, t])

	if (!isLoaderSupported) {
		return (
			<div className="flex min-h-0 flex-1 items-center justify-center p-6">
				<div className="flex max-w-md flex-col items-center justify-center text-center">
					<div className="mb-4 flex size-14 items-center justify-center rounded-2xl border border-amber-500/30 bg-amber-500/10 text-amber-500 shadow-xs">
						<AlertTriangle className="size-7" />
					</div>
					<h2 className="text-base font-semibold text-foreground">
						{t("instanceContent.unsupportedLoaderTitle")}
					</h2>
					<p className="mt-2 text-xs leading-relaxed text-muted-foreground">
						{t("instanceContent.unsupportedLoaderDescription", { loader: loader ?? "" })}
					</p>
					{onSwitchTab && (
						<div className="mt-5 flex flex-wrap items-center justify-center gap-2">
							<Button
								variant="outline"
								size="sm"
								onClick={() => {
									onSwitchTab("resourcepacks")
								}}
								className="h-8 gap-1.5 text-xs"
							>
								<Layers className="size-3.5" />
								<span>{t("instanceContent.tabs.resourcePacks")}</span>
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => {
									onSwitchTab("datapacks")
								}}
								className="h-8 gap-1.5 text-xs"
							>
								<Compass className="size-3.5" />
								<span>{t("instanceContent.tabs.datapacks")}</span>
							</Button>
						</div>
					)}
				</div>
			</div>
		)
	}

	return (
		<div className="flex min-h-0 flex-1 flex-col">
			{/* Controls Toolbar: Row 1 Actions / Bulk & Row 2 Filter Pills + Search */}
			<div className="flex shrink-0 flex-col gap-2 border-b border-border/40 bg-background/40 px-4 py-2 sm:px-6">
				{/* Row 1: Bulk Actions or Count on left, Actions on right */}
				<div className="flex items-center justify-between gap-2">
					{selectedFileNames.size > 0 ? (
						<div className="flex flex-wrap items-center gap-1.5">
							<span className="text-xs font-medium text-foreground">
								{t("instanceContent.selectedCount", { count: selectedFileNames.size })}
							</span>
							<Button
								variant="outline"
								size="sm"
								onClick={() => {
									void handleBulkEnable(true)
								}}
								disabled={isBusy || isInstanceRunning}
								className="h-7 px-2 text-2xs"
							>
								{t("instanceContent.enableSelected")}
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => {
									void handleBulkEnable(false)
								}}
								disabled={isBusy || isInstanceRunning}
								className="h-7 px-2 text-2xs"
							>
								{t("instanceContent.disableSelected")}
							</Button>
							<Button
								variant="destructive"
								size="sm"
								onClick={handleRequestDeleteSelected}
								disabled={isBusy || isInstanceRunning}
								className="h-7 gap-1 px-2 text-2xs"
							>
								<Trash2 className="size-3" />
								<span className="hidden sm:inline">{t("common.delete")}</span>
							</Button>
						</div>
					) : (
						<div className="flex items-center gap-1.5 text-xs text-muted-foreground">
							<span>{t("instanceContent.itemsCount", { count: items.length })}</span>
						</div>
					)}

					{/* Action Buttons */}
					<div className="flex shrink-0 items-center gap-1.5">
						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="outline"
										size="sm"
										onClick={handleIdentify}
										disabled={isBusy || isInstanceRunning || items.length === 0}
										className="h-8 gap-1.5 px-2.5 text-xs"
										aria-label={t("instanceContent.identify")}
									>
										{identifyMutation.isPending ? (
											<Spinner className="size-3.5" />
										) : (
											<Sparkles className="size-3.5 text-primary" />
										)}
										<span className="hidden xl:inline">{t("instanceContent.identify")}</span>
									</Button>
								}
							/>
							<TooltipContent>{t("instanceContent.identify")}</TooltipContent>
						</Tooltip>

						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="outline"
										size="sm"
										onClick={handleCheckUpdates}
										disabled={isBusy || isInstanceRunning || items.length === 0}
										className="h-8 gap-1.5 px-2.5 text-xs"
										aria-label={t("instanceContent.checkForUpdates")}
									>
										{checkUpdatesMutation.isPending ? (
											<Spinner className="size-3.5" />
										) : (
											<RefreshCw className="size-3.5" />
										)}
										<span className="hidden xl:inline">{t("instanceContent.checkForUpdates")}</span>
									</Button>
								}
							/>
							<TooltipContent>{t("instanceContent.checkForUpdates")}</TooltipContent>
						</Tooltip>

						{onOpenDiscoverMods && (
							<Button
								size="sm"
								onClick={onOpenDiscoverMods}
								className="h-8 gap-1.5 px-2.5 text-xs font-medium"
							>
								<PackagePlus className="size-3.5" />
								<span>{t("instanceContent.discoverMods")}</span>
							</Button>
						)}
					</div>
				</div>

				{/* Row 2: Severity/State Filter Pills on left, Search Box on right */}
				<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
					{/* Filter Pills */}
					<div className="flex items-center gap-0.5 overflow-x-auto rounded-lg border border-border/60 bg-card/30 p-0.5">
						{(["all", "enabled", "disabled", "issues"] as const).map((filter) => {
							const active = filterState === filter
							const count = counts[filter]
							const isIssuesWithCount = filter === "issues" && count > 0

							return (
								<button
									key={filter}
									type="button"
									onClick={() => {
										setFilterState(filter)
									}}
									className={cn(
										"inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-2xs font-medium transition-colors",
										active
											? "bg-primary text-primary-foreground shadow-xs"
											: isIssuesWithCount
												? "text-amber-500 hover:bg-amber-500/10"
												: "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
									)}
								>
									<span>{filterLabels[filter]}</span>
									<span
										className={cn(
											"rounded px-1 font-mono text-3xs",
											active
												? "bg-primary-foreground/20 text-primary-foreground"
												: isIssuesWithCount
													? "bg-amber-500/20 font-semibold text-amber-500"
													: "bg-muted text-muted-foreground",
										)}
									>
										{count}
									</span>
								</button>
							)
						})}
					</div>

					{/* Search Box */}
					<div className="relative w-full shrink-0 sm:w-56 md:w-64">
						<Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
						<Input
							placeholder={t("instanceContent.searchMods")}
							value={searchQuery}
							onChange={(e) => {
								setSearchQuery(e.target.value)
							}}
							className="h-8 pr-7 pl-8 text-xs"
						/>
						{searchQuery && (
							<button
								type="button"
								onClick={() => {
									setSearchQuery("")
								}}
								className="absolute top-1/2 right-2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
								aria-label={t("common.clearSearch")}
							>
								<X className="size-3.5" />
							</button>
						)}
					</div>
				</div>
			</div>

			{/* Dependency Issues Alert Banner */}
			{data?.dependencyIssues && data.dependencyIssues.length > 0 && (
				<div className="mx-4 mt-2.5 flex shrink-0 items-center gap-2.5 rounded-lg border border-amber-500/40 bg-amber-500/10 p-2.5 text-xs text-amber-600 sm:mx-6 dark:text-amber-400">
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

			{/* Mod List Content Body */}
			<div className="flex min-h-0 flex-1 flex-col overflow-hidden px-4 py-3 sm:px-6">
				{isLoading ? (
					<div className="flex flex-1 items-center justify-center p-12">
						<Spinner className="size-8" />
					</div>
				) : filteredItems.length === 0 ? (
					<div className="flex min-h-0 flex-1 items-center justify-center p-4">
						<Empty className="max-w-md border-0 p-0">
							<EmptyMedia variant="icon">
								<PackagePlus className="size-6 text-muted-foreground" />
							</EmptyMedia>
							<EmptyHeader>
								<EmptyTitle>{t("instanceContent.noModsFound")}</EmptyTitle>
								<EmptyDescription>
									{searchQuery
										? t("instanceContent.noModsMatchingFilter")
										: t("instanceContent.noModsInstalledDescription")}
								</EmptyDescription>
							</EmptyHeader>
							{!searchQuery && onOpenDiscoverMods && (
								<EmptyContent className="mt-2">
									<Button
										size="sm"
										onClick={onOpenDiscoverMods}
										className="h-8 gap-1.5 px-3 text-xs"
									>
										<PackagePlus className="size-3.5" />
										<span>{t("instanceContent.discoverMods")}</span>
									</Button>
								</EmptyContent>
							)}
						</Empty>
					</div>
				) : (
					<div className="flex min-h-0 flex-1 flex-col gap-2">
						<div className="flex shrink-0 items-center justify-between px-1 text-xs text-muted-foreground">
							<button
								type="button"
								onClick={handleToggleSelectAll}
								className="hover:text-foreground"
							>
								{allSelected ? t("instanceContent.deselectAll") : t("instanceContent.selectAll")}
							</button>
							{filteredItems.length !== items.length ? (
								<span>
									{filteredItems.length} / {items.length} mods
								</span>
							) : (
								<span>{t("instanceContent.itemsCount", { count: filteredItems.length })}</span>
							)}
						</div>

						<ScrollArea className="min-h-0 flex-1">
							<div className="flex flex-col gap-2 pr-3">
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
			</div>

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

			{updateCandidates && (
				<ContentUpdatesDialog
					instanceId={instanceId}
					isOpen={isUpdatesDialogOpen}
					onClose={() => {
						setIsUpdatesDialogOpen(false)
					}}
					candidates={updateCandidates}
					onApplied={() => {
						setUpdateCandidates(null)
					}}
				/>
			)}
		</div>
	)
}

ModsPanel.displayName = "ModsPanel"

export default memo(ModsPanel)
