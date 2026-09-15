import { createFileRoute } from "@tanstack/react-router"
import { FolderDown, Gamepad2, Layers, Plus, Square } from "lucide-react"
import { memo, useMemo, useState } from "react"
import type { InstanceConfig, ModLoaderType, SyncConflictInfo } from "@/bindings"
import DeleteInstanceDialog from "@/components/instances/delete-instance-dialog"
import ImportInstanceDialog from "@/components/instances/import-instance-dialog"
import InstanceCard from "@/components/instances/instance-card"
import InstanceSearchHeader from "@/components/instances/instance-search-header"
import InstanceSettingsDialog from "@/components/instances/instance-settings-dialog"
import NewInstanceDialog from "@/components/instances/new-instance-dialog"
import SyncConflictDialog from "@/components/instances/sync-conflict-dialog"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	instanceService,
	useAllInstancesProgress,
	useInstances,
	useRunningInstances,
} from "@/services/instance-service"
import { settingsService, useMemorySettings } from "@/services/settings-service"

const InstancesPage = () => {
	const [searchQuery, setSearchQuery] = useState("")
	const [isNewInstanceOpen, setIsNewInstanceOpen] = useState(false)
	const [isImportOpen, setIsImportOpen] = useState(false)
	const [editingInstance, setEditingInstance] = useState<InstanceConfig | null>(null)
	const [deletingInstance, setDeletingInstance] = useState<InstanceConfig | null>(null)
	const [syncConflict, setSyncConflict] = useState<SyncConflictInfo | null>(null)

	const { instances, refresh } = useInstances()
	const { runningMap, runningList } = useRunningInstances()
	const progressMap = useAllInstancesProgress()
	const { memory } = useMemorySettings()

	const handleCreateInstance = async (
		name: string,
		gameVersion: string,
		loader: ModLoaderType,
		loaderVersion: string | null,
	) => {
		await instanceService.createInstance(name, gameVersion, loader, loaderVersion)
		await refresh()
	}

	const handlePlay = async (instanceId: string) => {
		try {
			const conflict = await settingsService.checkSyncConflict(instanceId)
			if (conflict) {
				setSyncConflict(conflict)
				return
			}
			await instanceService.launchInstance(instanceId)
		} catch (e) {
			console.error("Failed to launch instance:", e)
			alert(`Failed to launch instance: ${e}`)
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
			alert(`Failed to resolve conflict: ${e}`)
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
		} catch (e) {
			console.error("Failed to delete instance:", e)
			alert(`Failed to delete instance: ${e}`)
		}
	}

	const filteredInstances = useMemo(() => {
		if (!searchQuery.trim()) return instances
		const q = searchQuery.toLowerCase()
		return instances.filter((i) => {
			const name = i.name || ""
			const ver = i.gameVersion || (i as unknown as Record<string, string>).game_version || ""
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
					searchQuery={searchQuery}
					onSearchChange={setSearchQuery}
					onOpenNewInstance={() => setIsNewInstanceOpen(true)}
					onOpenImport={() => setIsImportOpen(true)}
				/>

				{/* Active Running Instances Multi-Banner */}
				{runningList.length > 0 && (
					<div className="flex flex-wrap items-center gap-3 rounded-2xl border border-emerald-500/30 bg-emerald-950/20 px-4 py-3 shadow-emerald-950/10 shadow-lg backdrop-blur-md">
						<div className="flex items-center gap-2 font-medium text-emerald-400 text-xs">
							<span className="size-2 animate-ping rounded-full bg-emerald-400" />
							Active Instances ({runningList.length}):
						</div>

						<div className="flex flex-wrap items-center gap-2">
							{runningList.map((proc) => {
								const inst = instances.find((i) => i.id === proc.instanceId)
								return (
									<div
										key={proc.instanceId}
										className="flex items-center gap-2 rounded-lg border border-emerald-500/40 bg-emerald-900/30 px-3 py-1 font-medium text-emerald-200 text-xs shadow-sm"
									>
										<span>
											{inst?.name || "Instance"} (PID {proc.pid})
										</span>
										<button
											type="button"
											onClick={() => handleStop(proc.instanceId)}
											className="rounded p-0.5 text-emerald-400 transition-colors hover:bg-destructive/20 hover:text-destructive"
											title="Stop process"
										>
											<Square className="size-3 fill-current" />
										</button>
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
						<span>All Instances</span>
						<span className="rounded-full bg-zinc-800 px-2 py-0.5 font-mono text-[11px] text-zinc-400">
							{instances.length}
						</span>
					</div>
				</div>

				{/* Instances Grid or Empty State */}
				{instances.length === 0 ? (
					<div className="flex flex-1 flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-zinc-900/10 p-12 text-center">
						<div className="flex size-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 shadow-inner">
							<Gamepad2 className="size-6 text-primary" />
						</div>
						<h3 className="mt-4 font-semibold text-base text-foreground">
							No Instances Created Yet
						</h3>
						<p className="mt-1.5 max-w-sm text-muted-foreground text-xs">
							Create your first Minecraft instance to start playing. You can select Vanilla or mod
							loaders like Fabric, Quilt, or Forge.
						</p>
						<div className="mt-5 flex items-center gap-3">
							<Button onClick={() => setIsNewInstanceOpen(true)} className="gap-2" size="sm">
								<Plus className="size-4" />
								Create Your First Instance
							</Button>
							<Button
								variant="outline"
								onClick={() => setIsImportOpen(true)}
								className="gap-2"
								size="sm"
							>
								<FolderDown className="size-4" />
								Import from Launcher
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
								onPlay={() => handlePlay(inst.id)}
								onStop={() => handleStop(inst.id)}
								onSettings={() => setEditingInstance(inst)}
								onDelete={() => setDeletingInstance(inst)}
							/>
						))}

						{/* Create New Instance Card */}
						<button
							type="button"
							onClick={() => setIsNewInstanceOpen(true)}
							className="group flex min-h-[160px] flex-col items-center justify-center gap-3 rounded-2xl border border-border/50 border-dashed bg-zinc-950/30 p-6 text-muted-foreground transition-all duration-200 hover:border-primary/60 hover:bg-zinc-900/40 hover:text-white"
						>
							<div className="flex size-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 shadow-inner transition-transform group-hover:scale-110">
								<Plus className="size-5 text-zinc-400 group-hover:text-primary" />
							</div>
							<div className="flex flex-col items-center gap-0.5 text-center">
								<span className="font-semibold text-xs text-zinc-300 group-hover:text-white">
									Create New Instance
								</span>
								<span className="text-[11px] text-zinc-500">Choose any version or mod loader</span>
							</div>
						</button>
					</div>
				) : (
					<div className="flex flex-col items-center justify-center rounded-2xl border border-border/40 border-dashed bg-zinc-900/10 p-12 text-center">
						<p className="font-medium text-foreground text-sm">
							No instances found matching "{searchQuery}"
						</p>
						<p className="mt-1 text-muted-foreground text-xs">
							Try searching for another name, version, or mod loader.
						</p>
						<Button
							variant="outline"
							size="sm"
							onClick={() => setSearchQuery("")}
							className="mt-4 text-xs"
						>
							Clear Search
						</Button>
					</div>
				)}

				{/* New Instance Dialog */}
				<NewInstanceDialog
					open={isNewInstanceOpen}
					onOpenChange={setIsNewInstanceOpen}
					onCreateInstance={handleCreateInstance}
				/>

				{/* Import Instance Dialog */}
				<ImportInstanceDialog
					open={isImportOpen}
					onOpenChange={setIsImportOpen}
					onSuccess={async () => {
						await refresh()
					}}
				/>

				{/* Instance Settings Dialog */}
				<InstanceSettingsDialog
					instance={editingInstance}
					open={Boolean(editingInstance)}
					onOpenChange={(open) => !open && setEditingInstance(null)}
					onSave={async () => {
						await refresh()
					}}
				/>

				{/* Delete Instance Dialog */}
				<DeleteInstanceDialog
					instance={deletingInstance}
					open={Boolean(deletingInstance)}
					onOpenChange={(open) => !open && setDeletingInstance(null)}
					onConfirm={handleConfirmDelete}
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
	component: MemoizedInstancesPage,
})
