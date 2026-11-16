import { getRouteApi } from "@tanstack/react-router"
import { Search } from "lucide-react"
import { useCallback, useMemo, useState, type MouseEvent } from "react"
import { useShallow } from "zustand/react/shallow"
import InstanceContextMenu, {
	type ContextMenuState,
} from "@/features/play/components/InstanceContextMenu"
import InstanceGroup from "@/features/play/components/InstanceGroup"
import PlayToolbar from "@/features/play/components/PlayToolbar"
import QuickCreateModal from "@/features/play/components/QuickCreateModal"
import { useFilteredInstances } from "@/features/play/hooks/useFilteredInstances"
import type {
	CreateInstanceInput,
	EngineFilter,
	GameInstance,
	GroupByOption,
	TagFilter,
} from "@/features/play/types"
import { selectActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import { useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { findProcessByName, useProcessStore } from "@/shared/stores/useProcessStore"

const playRouteApi = getRouteApi("/")

const PlayPage = () => {
	const search = playRouteApi.useSearch()
	const navigate = playRouteApi.useNavigate()

	const { instances, selectedId, selectInstance, cycleTag, addInstance } = useInstancesStore(
		useShallow((s) => {
			return {
				instances: s.instances,
				selectedId: s.selectedId,
				selectInstance: s.selectInstance,
				cycleTag: s.cycleTag,
				addInstance: s.addInstance,
			}
		}),
	)
	const { runningProcesses, launchInstance, stopProcess } = useProcessStore(
		useShallow((s) => {
			return {
				runningProcesses: s.runningProcesses,
				launchInstance: s.launchInstance,
				stopProcess: s.stopProcess,
			}
		}),
	)
	const activeAccount = useAccountStore(selectActiveAccount)
	const ramGb = useLauncherPrefsStore((s) => {
		return s.ramGb
	})

	const [createOpen, setCreateOpen] = useState(false)
	const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null)

	const sections = useFilteredInstances({
		instances,
		query: search.q,
		engine: search.engine,
		tag: search.tag,
		groupBy: search.groupBy,
	})

	const runningInstanceNames = useMemo(() => {
		const names = runningProcesses.map((proc) => {
			return proc.name
		})

		return new Set(names)
	}, [runningProcesses])

	const totalVisible = useMemo(() => {
		return sections.reduce((sum, section) => {
			return sum + section.items.length
		}, 0)
	}, [sections])

	const handleQueryChange = useCallback(
		(nextQuery: string) => {
			void navigate({
				search: (prev) => {
					return { ...prev, q: nextQuery }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleEngineChange = useCallback(
		(nextEngine: EngineFilter) => {
			void navigate({
				search: (prev) => {
					return { ...prev, engine: nextEngine }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleTagChange = useCallback(
		(nextTag: TagFilter) => {
			void navigate({
				search: (prev) => {
					return { ...prev, tag: nextTag }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleGroupByChange = useCallback(
		(nextGroupBy: GroupByOption) => {
			void navigate({
				search: (prev) => {
					return { ...prev, groupBy: nextGroupBy }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleToggleCreate = useCallback(() => {
		setCreateOpen((prev) => {
			return !prev
		})
	}, [])

	const handleCloseCreate = useCallback(() => {
		setCreateOpen(false)
	}, [])

	const handleCreateInstance = useCallback(
		(input: CreateInstanceInput) => {
			addInstance(input)
			setCreateOpen(false)
		},
		[addInstance],
	)

	const handleOpenDetails = useCallback(
		(instanceId: string) => {
			void navigate({
				to: "/instances/$instanceId/mods",
				params: { instanceId },
			})
		},
		[navigate],
	)

	const handleContextMenu = useCallback(
		(instance: GameInstance, event: MouseEvent<HTMLDivElement>) => {
			setContextMenu({
				instance,
				x: event.clientX,
				y: event.clientY,
			})
		},
		[],
	)

	const handleCloseContextMenu = useCallback(() => {
		setContextMenu(null)
	}, [])

	const handleLaunchOrStop = useCallback(
		(instance: GameInstance) => {
			const runningProc = findProcessByName(runningProcesses, instance.name)
			if (runningProc) {
				stopProcess(runningProc.id)
				return
			}
			launchInstance({
				instance,
				username: activeAccount.username,
				ramGb,
			})
		},
		[activeAccount.username, launchInstance, ramGb, runningProcesses, stopProcess],
	)

	const handleCloneInstance = useCallback(
		(instance: GameInstance) => {
			addInstance({
				name: `${instance.name} Copy`,
				engine: instance.engine,
				loader: instance.loader,
				version: instance.version,
				colorTag: instance.colorTag,
			})
		},
		[addInstance],
	)

	const handleResetFilters = useCallback(() => {
		void navigate({
			search: () => {
				return {
					q: "",
					engine: "all",
					tag: "all",
					groupBy: "tag",
				}
			},
			replace: true,
		})
	}, [navigate])

	return (
		<div className="flex flex-col gap-3">
			<PlayToolbar
				query={search.q}
				engine={search.engine}
				tag={search.tag}
				groupBy={search.groupBy}
				onQueryChange={handleQueryChange}
				onEngineChange={handleEngineChange}
				onTagChange={handleTagChange}
				onGroupByChange={handleGroupByChange}
				onToggleCreate={handleToggleCreate}
			/>

			<QuickCreateModal
				open={createOpen}
				onClose={handleCloseCreate}
				onCreate={handleCreateInstance}
			/>

			{totalVisible === 0 ? (
				<div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-border bg-card/50 px-6 py-12 text-center">
					<div className="mb-2.5 flex size-10 items-center justify-center rounded-xl bg-foreground/6 text-muted-foreground">
						<Search className="size-4.5" />
					</div>
					<p className="text-[13.5px] font-medium text-foreground">
						No instances match your filters
					</p>
					<p className="mt-1 max-w-sm text-[12px] text-muted-foreground">
						Try clearing the search query or switching engine/tag filters.
					</p>
					<button
						type="button"
						onClick={handleResetFilters}
						className="mt-3 rounded-lg bg-foreground/8 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/14"
					>
						Reset Filters
					</button>
				</div>
			) : (
				<div className="flex flex-col gap-5 pb-2">
					{sections.map((section) => {
						return (
							<InstanceGroup
								key={section.key}
								section={section}
								selectedId={selectedId}
								runningNames={runningInstanceNames}
								onSelect={selectInstance}
								onCycleTag={cycleTag}
								onOpenDetails={handleOpenDetails}
								onContextMenu={handleContextMenu}
							/>
						)
					})}
				</div>
			)}

			<InstanceContextMenu
				state={contextMenu}
				isRunning={contextMenu ? runningInstanceNames.has(contextMenu.instance.name) : false}
				onClose={handleCloseContextMenu}
				onOpenDetails={handleOpenDetails}
				onLaunchOrStop={handleLaunchOrStop}
				onCycleTag={cycleTag}
				onClone={handleCloneInstance}
			/>
		</div>
	)
}

export default PlayPage
