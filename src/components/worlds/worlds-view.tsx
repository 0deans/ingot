import { getRouteApi } from "@tanstack/react-router"
import {
	ArrowUpDown,
	Check,
	ChevronDown,
	Filter,
	Folder,
	FolderClosed,
	Globe,
	HardDrive,
	RefreshCw,
	Search,
	Upload,
	X,
} from "lucide-react"
import { memo, useCallback, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type * as v from "valibot"
import type { InstanceConfig, WorldDimension, WorldSummary } from "@/bindings"
import InstanceIcon, { getColorTagDefinition } from "@/components/instances/instance-icon"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupInput,
} from "@/components/ui/input-group"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import type { worldSortSchema } from "@/routes/worlds"
import { useCachedInstances } from "@/services/instance-service"
import { useWorlds } from "@/services/world-service"
import DimensionResetDialog from "./dimension-reset-dialog"
import ImportWorldDialog from "./import-world-dialog"
import WorldBackupsDialog from "./world-backups-dialog"
import WorldCard from "./world-card"

const routeApi = getRouteApi("/worlds")

type SortOption = v.InferOutput<typeof worldSortSchema>

const SORT_OPTIONS: SortOption[] = [
	"lastPlayed-desc",
	"lastPlayed-asc",
	"name-asc",
	"name-desc",
	"size-desc",
	"size-asc",
]

const WorldsView = () => {
	const { t } = useTranslation()
	const instances = useCachedInstances()
	const search = routeApi.useSearch()
	const navigate = routeApi.useNavigate()

	const searchQuery = search.q ?? ""
	const selectedInstanceId = search.instance ?? "all"
	const sortBy = search.sort ?? "lastPlayed-desc"

	const { worlds, isLoading, isFetching, refetch, invalidate } = useWorlds(selectedInstanceId)

	const [searchInput, setSearchInput] = useState(searchQuery)
	const [resetTarget, setResetTarget] = useState<{
		world: WorldSummary
		dimension: WorldDimension
	} | null>(null)
	const [backupsTargetWorld, setBackupsTargetWorld] = useState<WorldSummary | null>(null)
	const [isImportOpen, setIsImportOpen] = useState(false)

	const instanceMap = useMemo(() => {
		return new Map(instances.map((i) => [i.id, i]))
	}, [instances])

	const activeInstance = useMemo(() => {
		return instanceMap.get(selectedInstanceId)
	}, [instanceMap, selectedInstanceId])

	const activeColorDef = useMemo(() => {
		return getColorTagDefinition(activeInstance?.colorTag)
	}, [activeInstance?.colorTag])

	const worldCountByInstanceId = useMemo(() => {
		const map = new Map<string, number>()
		for (const w of worlds) {
			map.set(w.instanceId, (map.get(w.instanceId) ?? 0) + 1)
		}
		return map
	}, [worlds])

	const instanceSections = useMemo(() => {
		const namedGroups = new Map<string, InstanceConfig[]>()
		const ungrouped: InstanceConfig[] = []

		for (const inst of instances) {
			const groupName = inst.group?.trim()
			if (groupName) {
				const list = namedGroups.get(groupName) ?? []
				list.push(inst)
				namedGroups.set(groupName, list)
			} else {
				ungrouped.push(inst)
			}
		}

		const sortedGroupNames = Array.from(namedGroups.keys()).sort((a, b) => {
			return a.localeCompare(b)
		})

		const sections: {
			key: string
			title: string | null
			isUngrouped: boolean
			items: InstanceConfig[]
		}[] = []

		if (sortedGroupNames.length === 0) {
			return [
				{
					key: "all",
					title: null,
					isUngrouped: true,
					items: instances,
				},
			]
		}

		for (const groupName of sortedGroupNames) {
			sections.push({
				key: `group:${groupName}`,
				title: groupName,
				isUngrouped: false,
				items: namedGroups.get(groupName) ?? [],
			})
		}

		if (ungrouped.length > 0) {
			sections.push({
				key: "__ungrouped__",
				title: t("instances.groups.ungrouped", { defaultValue: "Ungrouped" }),
				isUngrouped: true,
				items: ungrouped,
			})
		}

		return sections
	}, [instances, t])

	const instanceSelectorLabel = useMemo(() => {
		if (activeInstance) {
			return `${activeInstance.name} (${activeInstance.loader} ${activeInstance.gameVersion})`
		}
		return t("worlds.allInstances")
	}, [activeInstance, t])

	const handleOpenResetDialog = useCallback((world: WorldSummary, dimension: WorldDimension) => {
		setResetTarget({ world, dimension })
	}, [])

	const handleOpenBackupsDialog = useCallback((world: WorldSummary) => {
		setBackupsTargetWorld(world)
	}, [])

	const handleInstanceSelect = useCallback(
		(instanceId: string) => {
			navigate({
				search: (prev) => {
					return { ...prev, instance: instanceId }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleSortSelect = useCallback(
		(sort: SortOption) => {
			navigate({
				search: (prev) => {
					return { ...prev, sort }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleClearSearch = useCallback(() => {
		setSearchInput("")
		navigate({
			search: (prev) => {
				return { ...prev, q: "" }
			},
			replace: true,
		})
	}, [navigate])

	const getSortLabel = useCallback(
		(sort: SortOption): string => {
			switch (sort) {
				case "lastPlayed-desc":
					return t("worlds.sorts.lastPlayed-desc")
				case "lastPlayed-asc":
					return t("worlds.sorts.lastPlayed-asc")
				case "name-asc":
					return t("worlds.sorts.name-asc")
				case "name-desc":
					return t("worlds.sorts.name-desc")
				case "size-desc":
					return t("worlds.sorts.size-desc")
				case "size-asc":
					return t("worlds.sorts.size-asc")
			}
		},
		[t],
	)

	const sortLabel = useMemo(() => {
		return getSortLabel(sortBy)
	}, [getSortLabel, sortBy])

	const filteredWorlds = useMemo(() => {
		let list = [...worlds]
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase()
			list = list.filter((w) => {
				const inst = instanceMap.get(w.instanceId)
				const matchesGroup = inst?.group ? inst.group.toLowerCase().includes(q) : false
				return (
					w.displayName.toLowerCase().includes(q) ||
					w.folderName.toLowerCase().includes(q) ||
					w.instanceName.toLowerCase().includes(q) ||
					matchesGroup ||
					(w.seed ? w.seed.toLowerCase().includes(q) : false)
				)
			})
		}

		list.sort((a, b) => {
			switch (sortBy) {
				case "lastPlayed-desc":
					return (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0)
				case "lastPlayed-asc":
					return (a.lastPlayed ?? 0) - (b.lastPlayed ?? 0)
				case "name-asc":
					return a.displayName.localeCompare(b.displayName)
				case "name-desc":
					return b.displayName.localeCompare(a.displayName)
				case "size-desc":
					return b.sizeBytes - a.sizeBytes
				case "size-asc":
					return a.sizeBytes - b.sizeBytes
				default:
					return 0
			}
		})
		return list
	}, [worlds, searchQuery, sortBy, instanceMap])

	const renderInstanceMenuItem = useCallback(
		(inst: InstanceConfig) => {
			const count = worldCountByInstanceId.get(inst.id) ?? 0
			const isSelected = selectedInstanceId === inst.id
			const colorDef = getColorTagDefinition(inst.colorTag)

			return (
				<DropdownMenuItem
					key={inst.id}
					className={`flex items-center gap-2.5 py-2 text-xs ${
						isSelected ? "font-semibold text-primary" : ""
					}`}
					onClick={() => {
						return handleInstanceSelect(inst.id)
					}}
				>
					<InstanceIcon
						icon={inst.icon}
						loader={inst.loader}
						size={18}
						className="size-4.5 shrink-0 rounded"
					/>
					<div className="flex min-w-0 flex-1 flex-col">
						<div className="flex min-w-0 items-center gap-1.5">
							{colorDef && (
								<span
									className="size-2 shrink-0 rounded-full"
									style={{ backgroundColor: colorDef.hex }}
									title={colorDef.defaultLabel}
								/>
							)}
							<span className="truncate text-xs font-medium text-foreground">{inst.name}</span>
						</div>
						<span className="font-mono text-[10px] text-muted-foreground">
							{inst.loader} {inst.gameVersion}
						</span>
					</div>
					{count > 0 && (
						<span className="rounded-full bg-muted/60 px-1.5 py-0.5 text-3xs font-medium text-muted-foreground">
							{count}
						</span>
					)}
					{isSelected && <Check className="size-3.5 shrink-0 text-primary" />}
				</DropdownMenuItem>
			)
		},
		[worldCountByInstanceId, selectedInstanceId, handleInstanceSelect],
	)

	useEffect(() => {
		const timer = setTimeout(() => {
			if (searchInput !== searchQuery) {
				navigate({
					search: (prev) => {
						return { ...prev, q: searchInput }
					},
					replace: true,
				})
			}
		}, 300)
		return () => {
			clearTimeout(timer)
		}
	}, [searchInput, searchQuery, navigate])

	return (
		<div className="flex flex-1 flex-col overflow-hidden bg-background">
			{/* Top Bar Header */}
			<header className="flex flex-wrap items-center justify-between gap-4 border-b border-border/40 bg-background/50 px-6 py-4 backdrop-blur-md">
				<div className="flex items-center gap-3">
					<div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
						<Globe className="size-5" />
					</div>
					<div>
						<h1 className="text-xl font-bold tracking-tight text-foreground">
							{t("worlds.title")}
						</h1>
						<p className="text-xs text-muted-foreground">{t("worlds.subtitle")}</p>
					</div>
				</div>

				{/* Header Actions */}
				<div className="flex items-center gap-2">
					<Button
						variant="outline"
						size="sm"
						className="h-9 gap-2 text-xs"
						onClick={() => {
							return setIsImportOpen(true)
						}}
					>
						<Upload className="size-3.5 text-primary" />
						<span>{t("worlds.importWorld")}</span>
					</Button>

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="outline"
									size="icon"
									className="size-9 shrink-0"
									onClick={() => {
										return refetch()
									}}
									disabled={isFetching}
									aria-label={t("worlds.refreshTooltip")}
								>
									<RefreshCw
										className={`size-4 ${isFetching ? "animate-spin text-primary" : ""}`}
									/>
								</Button>
							}
						/>
						<TooltipContent side="bottom" align="end">
							{t("worlds.refreshTooltip")}
						</TooltipContent>
					</Tooltip>
				</div>
			</header>

			{/* Filter & Toolbar */}
			<div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-b border-border/40 bg-background/20 px-6 py-3">
				<div className="flex max-w-sm min-w-[200px] flex-1 items-center gap-2.5">
					<InputGroup className="h-9 w-full">
						<InputGroupAddon>
							<Search className="size-4 text-muted-foreground" />
						</InputGroupAddon>
						<InputGroupInput
							placeholder={t("worlds.searchPlaceholder")}
							value={searchInput}
							onChange={(e) => {
								return setSearchInput(e.target.value)
							}}
							className="text-xs"
						/>
						{searchInput && (
							<InputGroupAddon align="inline-end">
								<InputGroupButton
									size="icon-xs"
									variant="ghost"
									onClick={handleClearSearch}
									aria-label="Clear search"
								>
									<X className="size-3.5" />
								</InputGroupButton>
							</InputGroupAddon>
						)}
					</InputGroup>
				</div>

				<div className="flex items-center gap-2">
					{/* Instance Filter Dropdown */}
					<DropdownMenu>
						<DropdownMenuTrigger
							render={
								<Button variant="outline" size="sm" className="h-9 max-w-[260px] gap-2 text-xs">
									{activeInstance ? (
										<>
											<InstanceIcon
												icon={activeInstance.icon}
												loader={activeInstance.loader}
												size={16}
												className="size-4 shrink-0 rounded"
											/>
											{activeColorDef && (
												<span
													className="size-2 shrink-0 rounded-full"
													style={{ backgroundColor: activeColorDef.hex }}
													title={activeColorDef.defaultLabel}
												/>
											)}
											<span className="truncate">{activeInstance.name}</span>
										</>
									) : (
										<>
											<Filter className="size-3.5 shrink-0 text-muted-foreground" />
											<span className="truncate">{instanceSelectorLabel}</span>
										</>
									)}
									<ChevronDown className="ml-auto size-3.5 shrink-0 opacity-60" />
								</Button>
							}
						/>
						<DropdownMenuContent align="end" className="max-h-80 w-72 overflow-y-auto">
							<DropdownMenuItem
								className={`flex items-center gap-2.5 py-2 text-xs ${
									selectedInstanceId === "all" ? "font-semibold text-primary" : ""
								}`}
								onClick={() => {
									return handleInstanceSelect("all")
								}}
							>
								<Globe className="size-4 shrink-0 text-muted-foreground" />
								<span className="flex-1 truncate">{t("worlds.allInstances")}</span>
								<span className="rounded-full bg-muted/60 px-1.5 py-0.5 text-3xs font-medium text-muted-foreground">
									{worlds.length}
								</span>
								{selectedInstanceId === "all" && (
									<Check className="size-3.5 shrink-0 text-primary" />
								)}
							</DropdownMenuItem>

							<DropdownMenuSeparator />

							{instanceSections.map((section, idx) => {
								return (
									<DropdownMenuGroup key={section.key}>
										{section.title && (
											<>
												{idx > 0 && <DropdownMenuSeparator />}
												<DropdownMenuLabel className="flex items-center gap-1.5 px-2 py-1 text-3xs font-semibold tracking-wider text-muted-foreground/70 uppercase">
													{section.isUngrouped ? (
														<FolderClosed className="size-3" />
													) : (
														<Folder className="size-3" />
													)}
													<span>{section.title}</span>
													<span className="ml-auto text-3xs font-normal text-muted-foreground/50 lowercase">
														({section.items.length})
													</span>
												</DropdownMenuLabel>
											</>
										)}
										{section.items.map(renderInstanceMenuItem)}
									</DropdownMenuGroup>
								)
							})}
						</DropdownMenuContent>
					</DropdownMenu>

					{/* Sort Dropdown */}
					<DropdownMenu>
						<DropdownMenuTrigger
							render={
								<Button variant="outline" size="sm" className="h-9 gap-2 text-xs">
									<ArrowUpDown className="size-3.5 text-muted-foreground" />
									<span>{sortLabel}</span>
									<ChevronDown className="size-3.5 opacity-60" />
								</Button>
							}
						/>
						<DropdownMenuContent align="end" className="w-52">
							{SORT_OPTIONS.map((option) => {
								return (
									<DropdownMenuItem
										key={option}
										className={`text-xs ${sortBy === option ? "font-semibold text-primary" : ""}`}
										onClick={() => {
											return handleSortSelect(option)
										}}
									>
										{getSortLabel(option)}
									</DropdownMenuItem>
								)
							})}
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</div>

			{/* Main Grid View */}
			<ScrollArea className="flex-1 px-6 py-6">
				{isLoading ? (
					<div className="flex h-64 items-center justify-center">
						<Spinner className="size-8 text-primary" />
					</div>
				) : filteredWorlds.length === 0 ? (
					<div className="flex h-64 flex-col items-center justify-center gap-3 text-center">
						<div className="flex size-14 items-center justify-center rounded-2xl bg-muted/40 text-muted-foreground">
							<HardDrive className="size-7 stroke-[1.5]" />
						</div>
						<div className="space-y-1">
							<h3 className="text-base font-semibold text-foreground">
								{searchQuery ? t("worlds.noMatchingWorlds") : t("worlds.noSavedWorlds")}
							</h3>
							<p className="max-w-sm text-xs text-muted-foreground">
								{searchQuery ? "Try clearing your filters" : t("worlds.noSavedWorldsDesc")}
							</p>
						</div>
					</div>
				) : (
					<div className="grid grid-cols-1 gap-4 pb-8 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
						{filteredWorlds.map((world) => {
							return (
								<WorldCard
									key={`${world.instanceId}_${world.folderName}`}
									world={world}
									instance={instanceMap.get(world.instanceId)}
									onResetDimension={handleOpenResetDialog}
									onViewBackups={handleOpenBackupsDialog}
									onRefetch={invalidate}
								/>
							)
						})}
					</div>
				)}
			</ScrollArea>

			{/* Modals */}
			<DimensionResetDialog
				open={resetTarget !== null}
				onOpenChange={(open) => {
					if (!open) {
						setResetTarget(null)
					}
				}}
				world={resetTarget?.world ?? null}
				dimension={resetTarget?.dimension ?? null}
				onSuccess={invalidate}
			/>

			<WorldBackupsDialog
				open={backupsTargetWorld !== null}
				onOpenChange={(open) => {
					if (!open) {
						setBackupsTargetWorld(null)
					}
				}}
				world={backupsTargetWorld}
			/>

			<ImportWorldDialog
				open={isImportOpen}
				onOpenChange={setIsImportOpen}
				initialInstanceId={selectedInstanceId}
				onSuccess={invalidate}
			/>
		</div>
	)
}

WorldsView.displayName = "WorldsView"

export default memo(WorldsView)
