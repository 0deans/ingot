import { createFileRoute } from "@tanstack/react-router"
import { FolderDown, Gamepad2, Layers, Plus, Square } from "lucide-react"
import { memo, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import * as v from "valibot"
import type { ModLoaderType, QuickPlayOptions, SyncConflictInfo } from "@/bindings"
import { ScrollArea } from "@/components/common/scroll-area"
import DeleteInstanceDialog from "@/components/instances/delete-instance-dialog"
import DuplicateInstanceDialog from "@/components/instances/duplicate-instance-dialog"
import ImportInstanceDialog from "@/components/instances/import-instance-dialog"
import InstanceCard from "@/components/instances/instance-card"
import InstanceSearchHeader from "@/components/instances/instance-search-header"
import InstanceSettingsDialog from "@/components/instances/instance-settings-dialog"
import NewInstanceDialog from "@/components/instances/new-instance-dialog"
import SyncConflictDialog from "@/components/instances/sync-conflict-dialog"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import {
	instanceService,
	useAllInstancesProgress,
	useInstances,
	useRunningInstances,
} from "@/services/instance-service"
import { settingsService, useMemorySettings } from "@/services/settings-service"

export const instancesActionSchema = v.picklist(["new", "import"])

export const instancesSearchSchema = v.object({
	q: v.optional(v.fallback(v.string(), ""), ""),
	action: v.optional(instancesActionSchema),
	edit: v.optional(v.string()),
	delete: v.optional(v.string()),
	duplicate: v.optional(v.string()),
})

export type InstancesSearchParams = v.InferOutput<typeof instancesSearchSchema>

const InstancesPage = () => {
	const { t } = useTranslation()
	const search = Route.useSearch()
	const navigate = Route.useNavigate()

	const searchQuery = search.q ?? ""
	const action = search.action
	const editId = search.edit
	const deleteId = search.delete
	const duplicateId = search.duplicate

	const [searchInput, setSearchInput] = useState(searchQuery)
	const [syncConflict, setSyncConflict] = useState<SyncConflictInfo | null>(null)

	const { instances, refresh } = useInstances()
	const { runningMap, runningList } = useRunningInstances()
	const progressMap = useAllInstancesProgress()
	const { memory } = useMemorySettings()

	// Keep input synced if URL search param changes
	useEffect(() => {
		setSearchInput(searchQuery)
	}, [searchQuery])

	// Debounce search input to URL query
	useEffect(() => {
		const timer = setTimeout(() => {
			if (searchInput !== searchQuery) {
				navigate({
					search: (prev) => ({ ...prev, q: searchInput }),
					replace: true,
				})
			}
		}, 300)

		return () => clearTimeout(timer)
	}, [searchInput, searchQuery, navigate])

	const isNewInstanceOpen = action === "new"
	const isImportOpen = action === "import"
	const editingInstance = editId ? (instances.find((i) => i.id === editId) ?? null) : null
	const deletingInstance = deleteId ? (instances.find((i) => i.id === deleteId) ?? null) : null
	const duplicatingInstance = duplicateId
		? (instances.find((i) => i.id === duplicateId) ?? null)
		: null

	const handleCreateInstance = async (
		name: string,
		gameVersion: string,
		loader: ModLoaderType,
		loaderVersion: string | null,
	) => {
		await instanceService.createInstance(name, gameVersion, loader, loaderVersion)
		await refresh()
	}

	const handlePlay = async (instanceId: string, quickPlay?: QuickPlayOptions) => {
		try {
			const conflict = await settingsService.checkSyncConflict(instanceId)
			if (conflict) {
				setSyncConflict(conflict)
				return
			}
			await instanceService.launchInstance(instanceId, quickPlay)
		} catch (e) {
			console.error("Failed to launch instance:", e)
			alert(t("errors.launchInstance", { error: String(e) }))
		}
	}

	const handleResolveConflict = async (
		resolution: "use_shared" | "use_instance" | "disable_sync",
	) => {
		if (!syncConflict) return
		const id = syncConflict.instanceId
		try {
			await settingsService.resolveSyncConflict(id, resolution)
			setSyncConflict(null)
			await instanceService.launchInstance(id)
		} catch (e) {
			console.error("Failed to resolve conflict and launch:", e)
			alert(t("errors.resolveConflict", { error: String(e) }))
		}
	}

	const handleStop = async (instanceId: string) => {
		try {
			await instanceService.killInstance(instanceId)
		} catch (e) {
			console.error("Failed to stop instance:", e)
		}
	}

	const handleConfirmDelete = async (instanceId: string) => {
		try {
			await instanceService.deleteInstance(instanceId)
			navigate({
				search: (prev) => ({ ...prev, delete: undefined }),
				replace: true,
			})
		} catch (e) {
			console.error("Failed to delete instance:", e)
			alert(t("errors.deleteInstance", { error: String(e) }))
		}
	}

	const handleCloseAction = () => {
		navigate({
			search: (prev) => ({
				...prev,
				action: undefined,
				edit: undefined,
				delete: undefined,
				duplicate: undefined,
			}),
			replace: true,
		})
	}

	const filteredInstances = useMemo(() => {
		if (!searchQuery.trim()) return instances
		const q = searchQuery.toLowerCase()
		return instances.filter((i) => {
			const name = i.name || ""
			const ver = i.gameVersion || ""
			const ldr = String(i.loader || "")
			return (
				name.toLowerCase().includes(q) ||
				ver.toLowerCase().includes(q) ||
				ldr.toLowerCase().includes(q)
			)
		})
	}, [instances, searchQuery])

	return (
		<ScrollArea className="size-full flex-1" scrollFade>
			<div className="flex flex-1 flex-col gap-6 p-4 pb-12 sm:p-5 lg:p-6">
				{/* Top Header */}
				<InstanceSearchHeader
					searchQuery={searchInput}
					onSearchChange={setSearchInput}
					onOpenNewInstance={() =>
						navigate({
							search: (prev) => ({ ...prev, action: "new" }),
							replace: true,
						})
					}
					onOpenImport={() =>
						navigate({
							search: (prev) => ({ ...prev, action: "import" }),
							replace: true,
						})
					}
				/>

				{/* Active Running Instances Multi-Banner */}
				{runningList.length > 0 && (
					<div className="flex flex-wrap items-center gap-3 rounded-2xl border border-primary/30 bg-primary/10 px-4 py-3 shadow-lg shadow-primary/10 backdrop-blur-md">
						<div className="flex items-center gap-2 font-medium text-primary text-xs">
							<span className="size-2 animate-ping rounded-full bg-primary" />
							{t("instances.activeInstances", { count: runningList.length })}
						</div>

						<div className="flex flex-wrap items-center gap-2">
							{runningList.map((proc) => {
								const inst = instances.find((i) => i.id === proc.instanceId)
								return (
									<div
										key={proc.instanceId}
										className="flex items-center gap-2 rounded-lg border border-primary/40 bg-primary/10 px-3 py-1 font-medium text-primary text-xs shadow-sm"
									>
										<span>
											{t("instances.activeEntry", { name: inst?.name ?? "", pid: proc.pid })}
										</span>
										<Tooltip>
											<TooltipTrigger
												render={
													<button
														type="button"
														onClick={() => handleStop(proc.instanceId)}
														className="rounded p-0.5 text-primary transition-colors hover:bg-destructive/20 hover:text-destructive"
														aria-label={t("instances.stopProcess")}
													/>
												}
											>
												<Square className="size-3 fill-current" />
											</TooltipTrigger>
											<TooltipContent>{t("instances.stopProcess")}</TooltipContent>
										</Tooltip>
									</div>
								)
							})}
						</div>
					</div>
				)}

				{/* Instances Section Header */}
				<div className="flex items-center justify-between border-border/30 border-b pb-2">
					<div className="flex items-center gap-2 font-semibold text-foreground text-sm">
						<Layers className="size-4 text-primary" />
						<span>{t("instances.allInstances")}</span>
						<span className="rounded-full bg-muted px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
							{instances.length}
						</span>
					</div>
				</div>

				{/* Instances Grid or Empty State */}
				{instances.length === 0 ? (
					<div className="flex flex-1 flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-card/10 p-12 text-center">
						<div className="flex size-12 items-center justify-center rounded-2xl border border-border bg-card/80 shadow-inner">
							<Gamepad2 className="size-6 text-primary" />
						</div>
						<h3 className="mt-4 font-semibold text-base text-foreground">
							{t("instances.emptyTitle")}
						</h3>
						<p className="mt-1.5 max-w-sm text-muted-foreground text-xs">
							{t("instances.emptySubtitle")}
						</p>
						<div className="mt-5 flex items-center gap-3">
							<Button
								onClick={() =>
									navigate({
										search: (prev) => ({ ...prev, action: "new" }),
										replace: true,
									})
								}
								className="gap-2"
								size="sm"
							>
								<Plus className="size-4" />
								{t("instances.createFirst")}
							</Button>
							<Button
								variant="outline"
								onClick={() =>
									navigate({
										search: (prev) => ({ ...prev, action: "import" }),
										replace: true,
									})
								}
								className="gap-2"
								size="sm"
							>
								<FolderDown className="size-4" />
								{t("instances.importFromLauncher")}
							</Button>
						</div>
					</div>
				) : filteredInstances.length > 0 ? (
					<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
						{filteredInstances.map((inst) => (
							<InstanceCard
								key={inst.id}
								instance={inst}
								runningInfo={runningMap.get(inst.id)}
								progress={progressMap.get(inst.id)}
								globalMaxRamMb={memory.maxRamMb}
								onPlay={(quickPlay) => handlePlay(inst.id, quickPlay)}
								onStop={() => handleStop(inst.id)}
								onSettings={() =>
									navigate({
										search: (prev) => ({ ...prev, edit: inst.id }),
										replace: true,
									})
								}
								onDelete={() =>
									navigate({
										search: (prev) => ({ ...prev, delete: inst.id }),
										replace: true,
									})
								}
								onDuplicate={() =>
									navigate({
										search: (prev) => ({ ...prev, duplicate: inst.id }),
										replace: true,
									})
								}
							/>
						))}

						{/* Create New Instance Card */}
						<button
							type="button"
							onClick={() =>
								navigate({
									search: (prev) => ({ ...prev, action: "new" }),
									replace: true,
								})
							}
							className="group flex min-h-[160px] flex-col items-center justify-center gap-3 rounded-2xl border border-border/50 border-dashed bg-background/30 p-6 text-muted-foreground transition-all duration-200 hover:border-primary/60 hover:bg-card/40 hover:text-foreground"
						>
							<div className="flex size-10 items-center justify-center rounded-xl border border-border bg-card shadow-inner transition-transform group-hover:scale-110">
								<Plus className="size-5 text-muted-foreground group-hover:text-primary" />
							</div>
							<div className="flex flex-col items-center gap-0.5 text-center">
								<span className="font-semibold text-foreground/80 text-xs group-hover:text-foreground">
									{t("instances.createCardTitle")}
								</span>
								<span className="text-[11px] text-muted-foreground">
									{t("instances.createCardSubtitle")}
								</span>
							</div>
						</button>
					</div>
				) : (
					<div className="flex flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-card/10 p-12 text-center">
						<p className="font-medium text-foreground text-sm">
							{t("instances.noMatchTitle", { query: searchQuery })}
						</p>
						<p className="mt-1 text-muted-foreground text-xs">{t("instances.noMatchSubtitle")}</p>
						<Button
							variant="outline"
							size="sm"
							onClick={() => {
								setSearchInput("")
								navigate({
									search: (prev) => ({ ...prev, q: "" }),
									replace: true,
								})
							}}
							className="mt-4 text-xs"
						>
							{t("common.clearSearch")}
						</Button>
					</div>
				)}

				{/* New Instance Dialog */}
				<NewInstanceDialog
					open={isNewInstanceOpen}
					onOpenChange={(open) => !open && handleCloseAction()}
					onCreateInstance={async (name, ver, loader, loaderVer) => {
						await handleCreateInstance(name, ver, loader, loaderVer)
						handleCloseAction()
					}}
				/>

				{/* Import Instance Dialog */}
				<ImportInstanceDialog
					open={isImportOpen}
					onOpenChange={(open) => !open && handleCloseAction()}
					onSuccess={async () => {
						await refresh()
						handleCloseAction()
					}}
				/>

				{/* Instance Settings Dialog */}
				<InstanceSettingsDialog
					instance={editingInstance}
					open={Boolean(editingInstance)}
					onOpenChange={(open) => !open && handleCloseAction()}
					onSave={async () => {
						await refresh()
					}}
				/>

				{/* Delete Instance Dialog */}
				<DeleteInstanceDialog
					instance={deletingInstance}
					open={Boolean(deletingInstance)}
					onOpenChange={(open) => !open && handleCloseAction()}
					onConfirm={handleConfirmDelete}
				/>

				{/* Duplicate Instance Dialog */}
				<DuplicateInstanceDialog
					instance={duplicatingInstance}
					open={Boolean(duplicatingInstance)}
					isRunning={duplicatingInstance ? runningMap.has(duplicatingInstance.id) : false}
					onOpenChange={(open) => !open && handleCloseAction()}
				/>

				{/* Sync Conflict Dialog */}
				<SyncConflictDialog
					conflict={syncConflict}
					open={Boolean(syncConflict)}
					onResolve={handleResolveConflict}
					onCancel={() => setSyncConflict(null)}
				/>
			</div>
		</ScrollArea>
	)
}

InstancesPage.displayName = "InstancesPage"

const MemoizedInstancesPage = memo(InstancesPage)

export const Route = createFileRoute("/")({
	validateSearch: instancesSearchSchema,
	component: MemoizedInstancesPage,
})
