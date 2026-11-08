import { useNavigate } from "@tanstack/react-router"
import {
	Check,
	ChevronDown,
	Clock,
	Compass,
	Copy,
	Folder,
	FolderOpen,
	Gamepad2,
	Globe,
	HardDrive,
	LayoutGrid,
	Monitor,
	MoreVertical,
	Package,
	Palette,
	Play,
	Settings,
	Share2,
	Square,
	Terminal,
	Trash2,
	X,
} from "lucide-react"
import { motion } from "motion/react"
import { memo, useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type {
	InstanceConfig,
	InstanceWorldSummary,
	LaunchProgressEvent,
	QuickPlayOptions,
	RunningInstanceSummary,
	ShortcutLocation,
} from "@/bindings"
import { DirectConnectDialog } from "@/components/instances/direct-connect-dialog"
import InstanceIcon, {
	COLOR_TAGS,
	getColorTagDefinition,
	type InstanceColorTag,
} from "@/components/instances/instance-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Progress } from "@/components/ui/progress"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { translateLaunchDetail, translateLaunchPhase } from "@/lib/backend-text"
import {
	formatDuration as formatLocalDuration,
	formatMegabytes,
	formatPercent,
	formatRelative,
} from "@/lib/format"
import { instanceService } from "@/services/instance-service"

interface InstanceCardProps {
	instance: InstanceConfig
	runningInfo?: RunningInstanceSummary
	progress?: LaunchProgressEvent
	globalMaxRamMb: number
	existingGroups?: string[]
	onPlay: (quickPlay?: QuickPlayOptions) => void
	onStop: () => void
	onLogs: () => void
	onSettings: () => void
	onDelete: () => void
	onDuplicate: () => void
	onQuickUpdate?: (updated: InstanceConfig) => Promise<void> | void
}

function isVersionAtLeast(versionStr: string, targetMajor: number, targetMinor: number): boolean {
	const parts = versionStr.split(".")
	if (parts.length >= 2) {
		const major = Number.parseInt(parts[0], 10)
		const minor = Number.parseInt(parts[1], 10)
		if (!Number.isNaN(major) && !Number.isNaN(minor)) {
			if (major > targetMajor) return true
			if (major === targetMajor && minor >= targetMinor) return true
			return false
		}
	}
	const firstTwo = Number.parseInt(versionStr.slice(0, 2), 10)
	if (!Number.isNaN(firstTwo) && firstTwo >= 23) {
		return true
	}
	return false
}

function formatDuration(seconds: number): string {
	return formatLocalDuration(seconds, seconds < 3600)
}

export const InstanceCard = ({
	instance,
	runningInfo,
	progress,
	globalMaxRamMb,
	existingGroups = [],
	onPlay,
	onStop,
	onLogs,
	onSettings,
	onDelete,
	onDuplicate,
	onQuickUpdate,
}: InstanceCardProps) => {
	const { t } = useTranslation()
	const navigate = useNavigate()
	const isRunning = !!runningInfo
	const isDownloading = !!progress && !isRunning
	const customColorInputRef = useRef<HTMLInputElement | null>(null)

	const [elapsedSeconds, setElapsedSeconds] = useState(0)
	const [worlds, setWorlds] = useState<InstanceWorldSummary[]>([])
	const [loadingWorlds, setLoadingWorlds] = useState(false)
	const [directConnectOpen, setDirectConnectOpen] = useState(false)

	const supportsSingleplayerQP = isVersionAtLeast(instance.gameVersion, 1, 20)

	const formatLastPlayed = (timestamp?: number | null): string => {
		if (!timestamp) return t("instances.card.neverPlayed")
		return formatRelative(timestamp)
	}

	const loadWorlds = async () => {
		if (!supportsSingleplayerQP) return
		try {
			setLoadingWorlds(true)
			const list = await instanceService.getInstanceWorlds(instance.id)
			setWorlds(list)
		} catch (e) {
			console.error("Failed to load instance worlds:", e)
		} finally {
			setLoadingWorlds(false)
		}
	}

	const getShortcutSuccessMessage = (location: ShortcutLocation): string => {
		switch (location) {
			case "desktop": {
				return t("instances.card.shortcutCreatedDesktop", { name: instance.name })
			}
			case "applicationsMenu": {
				return t("instances.card.shortcutCreatedAppMenu", { name: instance.name })
			}
			case "steam": {
				return t("instances.card.shortcutCreatedSteam", { name: instance.name })
			}
		}
	}

	const handleCreateShortcut = async (location: ShortcutLocation) => {
		try {
			await instanceService.createShortcut(instance.id, location)
			toast.success(getShortcutSuccessMessage(location))
		} catch (error) {
			toast.error(
				t("instances.card.shortcutFailed", {
					error: error instanceof Error ? error.message : String(error),
				}),
			)
		}
	}

	useEffect(() => {
		if (!isRunning || !runningInfo) {
			return
		}

		const startedAt = runningInfo.startedAt
		const update = () => {
			const now = Math.floor(Date.now() / 1000)
			setElapsedSeconds(Math.max(0, now - startedAt))
		}
		update()
		const timer = setInterval(update, 1000)
		return () => {
			clearInterval(timer)
		}
	}, [isRunning, runningInfo])

	const handleSetColorTag = async (colorTag: InstanceColorTag | null) => {
		const updated: InstanceConfig = {
			...instance,
			colorTag,
		}
		try {
			await instanceService.updateInstance(updated)
			await onQuickUpdate?.(updated)
		} catch (e) {
			console.error("Failed to update instance color tag:", e)
		}
	}

	const handleSetGroup = async (group: string | null) => {
		const normalizedGroup = group?.trim() ? group.trim() : null
		const updated: InstanceConfig = {
			...instance,
			group: normalizedGroup,
		}
		try {
			await instanceService.updateInstance(updated)
			await onQuickUpdate?.(updated)
		} catch (e) {
			console.error("Failed to update instance group:", e)
		}
	}

	const loaderType = instance.loader || "vanilla"
	const loaderName =
		loaderType === "vanilla"
			? t("versionChange.vanilla").toLocaleUpperCase()
			: String(loaderType).toUpperCase()
	const versionStr = instance.gameVersion || ""

	const hasCustomRam = instance.memoryMaxMb != null
	const effectiveRamMb = instance.memoryMaxMb ?? globalMaxRamMb
	const colorTagDef = getColorTagDefinition(instance.colorTag)
	const liveTotalPlaytimeSeconds = instance.totalPlayTimeSeconds + (isRunning ? elapsedSeconds : 0)
	const groupLabel = instance.group?.trim() || null

	let cardStateClasses = "border-border/50"
	if (isRunning) {
		cardStateClasses = "border-primary/40 ring-1 ring-primary/20"
	} else if (isDownloading) {
		cardStateClasses = "border-warning/40 ring-1 ring-warning/20"
	} else if (colorTagDef && !colorTagDef.isCustom) {
		cardStateClasses = colorTagDef.borderClass
	}

	const customCardStyle =
		!isRunning && !isDownloading && colorTagDef?.isCustom
			? { borderColor: `${colorTagDef.hex}59` }
			: undefined
	const customGlowStyle = colorTagDef?.isCustom
		? {
				background: `linear-gradient(135deg, ${colorTagDef.hex}1f 0%, ${colorTagDef.hex}0a 50%, transparent 100%)`,
			}
		: undefined

	return (
		<motion.div
			style={customCardStyle}
			className={`group relative flex flex-col justify-between overflow-hidden rounded-2xl border bg-background/60 p-5 transition-[border-color,background-color,box-shadow] duration-200 hover:border-input/80 hover:bg-card/50 hover:shadow-xl ${cardStateClasses}`}
		>
			<input
				ref={customColorInputRef}
				type="color"
				value={colorTagDef?.hex ?? "#3b82f6"}
				onChange={(e) => {
					return void handleSetColorTag(e.target.value)
				}}
				className="pointer-events-none absolute right-5 bottom-4 size-6 opacity-0"
				tabIndex={-1}
				aria-hidden="true"
			/>
			{colorTagDef && (
				<div
					style={customGlowStyle}
					className={`pointer-events-none absolute inset-0 ${colorTagDef.isCustom ? "" : `bg-gradient-to-br ${colorTagDef.glowClass}`}`}
				/>
			)}

			{/* Top Bar: Loader Badge & Status */}
			<div className="flex items-start justify-between gap-3">
				<div className="flex items-center gap-2.5">
					<div className="flex size-9 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-border bg-card shadow-inner">
						<InstanceIcon icon={instance.icon} loader={loaderType} size={22} />
					</div>
					<div className="flex flex-col">
						<span className="text-2xs font-semibold tracking-wider text-muted-foreground uppercase">
							{loaderName}
						</span>
						<span className="font-mono text-xs text-foreground/80">{versionStr}</span>
					</div>
				</div>

				{/* Status Pill */}
				<div>
					{isRunning ? (
						<Badge variant="outline" className="border-primary/30 text-primary">
							<span className="size-1.5 animate-pulse rounded-full bg-primary" />
							{formatDuration(elapsedSeconds)}
						</Badge>
					) : isDownloading ? (
						<Badge variant="outline" className="border-warning/30 text-warning">
							<Spinner />
							{progress.percentage != null
								? formatPercent(progress.percentage)
								: t("instances.card.preparing")}
						</Badge>
					) : (
						<Badge variant="outline" className="text-muted-foreground">
							{formatLastPlayed(instance.lastPlayed)}
						</Badge>
					)}
				</div>
			</div>

			{/* Main Info */}
			<div className="my-4 flex flex-col gap-2">
				<h3
					className="truncate text-base font-semibold tracking-tight text-foreground"
					title={instance.name}
				>
					{instance.name}
				</h3>

				<div className="flex flex-wrap items-center gap-2 text-2xs text-muted-foreground">
					<span
						className="inline-flex items-center gap-1 rounded-md border border-border/80 bg-card/60 px-2 py-0.5"
						title={
							hasCustomRam
								? t("instances.card.customRamTooltip")
								: t("instances.card.globalRamTooltip")
						}
					>
						<HardDrive className="size-3 text-info" />
						<span>
							{t("instances.card.ram", { size: formatMegabytes(effectiveRamMb) })}
							{hasCustomRam && (
								<span className="ml-1 text-3xs font-medium text-info">
									{t("instances.card.customRam")}
								</span>
							)}
						</span>
					</span>

					{liveTotalPlaytimeSeconds > 0 && (
						<span
							className="inline-flex items-center gap-1 rounded-md border border-border/80 bg-card/60 px-2 py-0.5"
							title={t("instanceSettings.playtimeTitle", { defaultValue: "Cumulative Playtime" })}
						>
							<Clock className="size-3 text-primary" />
							<span>
								{t("instances.card.playedDuration", {
									duration: formatDuration(liveTotalPlaytimeSeconds),
								})}
							</span>
						</span>
					)}
				</div>

				{/* Progress Track if downloading */}
				{isDownloading && (
					<div className="mt-1 flex flex-col gap-1.5 rounded-lg border border-warning/20 bg-background/60 p-2.5 text-2xs">
						<div className="flex items-center justify-between font-medium text-warning">
							<span className="truncate">{translateLaunchPhase(progress.phase)}</span>
							<span className="font-mono">
								{progress.percentage != null ? formatPercent(progress.percentage) : ""}
							</span>
						</div>
						<Progress value={Math.min(100, Math.max(0, progress.percentage ?? 0))} />
						<span className="truncate font-mono text-3xs text-muted-foreground">
							{translateLaunchDetail(progress.detail)}
						</span>
					</div>
				)}
			</div>

			{/* Actions Footer */}
			<div className="flex items-center justify-between border-t border-border/40 pt-3.5">
				{/* Primary Launch Action */}
				<div>
					{isRunning ? (
						<Button
							size="sm"
							variant="destructive"
							onClick={onStop}
							className="h-8 gap-1.5 px-3.5 text-xs font-medium shadow-sm hover:scale-[1.02] active:scale-[0.98]"
						>
							<Square className="size-3.5 fill-current" />
							<span>{t("instances.card.stop")}</span>
						</Button>
					) : isDownloading ? (
						<Button
							size="sm"
							disabled
							className="h-8 gap-1.5 px-3.5 text-xs font-medium opacity-75"
						>
							<Spinner className="size-3.5" />
							<span>{t("instances.card.preparingBtn")}</span>
						</Button>
					) : (
						<div className="inline-flex items-center rounded-md shadow-md">
							<Button
								size="sm"
								onClick={() => onPlay()}
								className="h-8 gap-1.5 rounded-r-none border-r border-primary/60 px-3.5 text-xs font-semibold active:scale-[0.98]"
							>
								<Play className="size-3.5 fill-current" />
								<span>{t("instances.card.play")}</span>
							</Button>
							<DropdownMenu
								onOpenChange={(open) => {
									if (open) loadWorlds()
								}}
							>
								<Tooltip>
									<TooltipTrigger
										render={
											<DropdownMenuTrigger
												render={
													<Button
														size="sm"
														className="h-8 rounded-l-none px-1.5"
														aria-label={t("instances.card.quickPlayOptions")}
													>
														<ChevronDown className="size-3.5" />
													</Button>
												}
											/>
										}
									/>
									<TooltipContent>{t("instances.card.quickPlayOptions")}</TooltipContent>
								</Tooltip>
								<DropdownMenuContent align="start" className="w-56">
									<DropdownMenuGroup>
										<DropdownMenuLabel className="text-3xs font-semibold tracking-wider text-muted-foreground uppercase">
											{t("instances.card.quickPlay")}
										</DropdownMenuLabel>
										<DropdownMenuItem
											onClick={() => setDirectConnectOpen(true)}
											className="cursor-pointer gap-2"
										>
											<Globe className="size-4 text-primary" />
											<div className="flex flex-col">
												<span className="text-xs font-medium">
													{t("instances.card.directConnect")}
												</span>
												<span className="text-3xs text-muted-foreground">
													{t("instances.card.directConnectDesc")}
												</span>
											</div>
										</DropdownMenuItem>

										{supportsSingleplayerQP ? (
											<DropdownMenuSub>
												<DropdownMenuSubTrigger className="cursor-pointer gap-2">
													<Compass className="size-4 text-info" />
													<div className="flex flex-col text-left">
														<span className="text-xs font-medium">
															{t("instances.card.quickLoadWorld")}
														</span>
														<span className="text-3xs text-muted-foreground">
															{t("instances.card.quickLoadWorldDesc")}
														</span>
													</div>
												</DropdownMenuSubTrigger>
												<DropdownMenuSubContent className="max-h-72 w-64 overflow-y-auto">
													<DropdownMenuGroup>
														<DropdownMenuLabel className="text-3xs font-semibold tracking-wider text-muted-foreground uppercase">
															{t("instances.card.singleplayerSaves")}
														</DropdownMenuLabel>
														{loadingWorlds ? (
															<div className="flex items-center gap-2 p-2.5 text-xs text-muted-foreground">
																<Spinner className="size-3.5" />
																<span>{t("instances.card.scanningWorlds")}</span>
															</div>
														) : worlds.length === 0 ? (
															<div className="p-2.5 text-xs text-muted-foreground">
																{t("instances.card.noSavedWorlds")}
															</div>
														) : (
															worlds.map((w) => (
																<DropdownMenuItem
																	key={w.folderName}
																	onClick={() => onPlay({ server: null, world: w.folderName })}
																	className="cursor-pointer gap-2.5 py-1.5"
																>
																	{w.icon ? (
																		<img
																			src={w.icon}
																			alt=""
																			className="size-7 shrink-0 rounded border border-border object-cover"
																		/>
																	) : (
																		<div className="flex size-7 shrink-0 items-center justify-center rounded border border-border bg-card text-muted-foreground">
																			<Compass className="size-3.5" />
																		</div>
																	)}
																	<div className="flex min-w-0 flex-col truncate">
																		<span className="truncate text-xs font-medium text-foreground">
																			{w.displayName}
																		</span>
																		<span className="truncate text-3xs text-muted-foreground">
																			{w.lastPlayed ? formatLastPlayed(w.lastPlayed) : w.folderName}
																		</span>
																	</div>
																</DropdownMenuItem>
															))
														)}
													</DropdownMenuGroup>
												</DropdownMenuSubContent>
											</DropdownMenuSub>
										) : (
											<DropdownMenuItem disabled className="gap-2 opacity-50">
												<Compass className="size-4 text-muted-foreground" />
												<div className="flex flex-col">
													<span className="text-xs font-medium">
														{t("instances.card.quickLoadWorld")}
													</span>
													<span className="text-3xs text-muted-foreground">
														{t("instances.card.requires120")}
													</span>
												</div>
											</DropdownMenuItem>
										)}
									</DropdownMenuGroup>
								</DropdownMenuContent>
							</DropdownMenu>
						</div>
					)}
				</div>

				{/* Utility Actions */}
				<div className="flex items-center gap-1">
					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="ghost"
									size="icon"
									onClick={onLogs}
									disabled={isDownloading || instance.id.startsWith("installing:")}
									className="size-8 text-muted-foreground hover:text-foreground"
									aria-label={t("instances.card.logsTooltip")}
								/>
							}
						>
							<Terminal className="size-4" />
						</TooltipTrigger>
						<TooltipContent>{t("instances.card.logsTooltip")}</TooltipContent>
					</Tooltip>
					<DropdownMenu>
						<Tooltip>
							<TooltipTrigger
								render={
									<DropdownMenuTrigger
										render={
											<Button
												variant="ghost"
												size="icon"
												disabled={isDownloading || instance.id.startsWith("installing:")}
												className="size-8 text-muted-foreground hover:text-foreground"
												aria-label={t("instances.card.moreActions")}
											>
												<MoreVertical className="size-4" />
											</Button>
										}
									/>
								}
							/>
							<TooltipContent>{t("instances.card.moreActions")}</TooltipContent>
						</Tooltip>
						<DropdownMenuContent align="end" className="w-52">
							<DropdownMenuGroup>
								<DropdownMenuItem onClick={onLogs} className="cursor-pointer gap-2">
									<Terminal className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("instances.card.logsTooltip")}</span>
								</DropdownMenuItem>
								<DropdownMenuItem
									onClick={() => {
										return void navigate({
											to: "/instances/$id",
											params: { id: instance.id },
										})
									}}
									className="cursor-pointer gap-2"
								>
									<Package className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("instanceContent.manageContent")}</span>
								</DropdownMenuItem>
								<DropdownMenuItem onClick={onSettings} className="cursor-pointer gap-2">
									<Settings className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("instances.card.settingsTooltip")}</span>
								</DropdownMenuItem>
								<DropdownMenuItem
									onClick={() => {
										return void instanceService.openInstanceFolder(instance.id)
									}}
									className="cursor-pointer gap-2"
								>
									<FolderOpen className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("instances.card.openFolderTooltip")}</span>
								</DropdownMenuItem>
								<DropdownMenuItem onClick={onDuplicate} className="cursor-pointer gap-2">
									<Copy className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("instances.card.duplicateTooltip")}</span>
								</DropdownMenuItem>
								<DropdownMenuItem
									onClick={() => {
										void navigate({
											to: "/instances/$id/export",
											params: { id: instance.id },
											search: { from: "home" },
										})
									}}
									className="cursor-pointer gap-2"
								>
									<Package className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("instances.card.exportTooltip")}</span>
								</DropdownMenuItem>
								<DropdownMenuItem
									onClick={() => {
										return navigate({ to: "/worlds", search: { instance: instance.id } })
									}}
									className="cursor-pointer gap-2"
								>
									<Globe className="size-4 text-muted-foreground" />
									<span className="text-xs">{t("nav.worlds")}</span>
								</DropdownMenuItem>
								<DropdownMenuSub>
									<DropdownMenuSubTrigger className="cursor-pointer gap-2">
										<Palette className="size-4 text-muted-foreground" />
										<span className="text-xs">
											{t("instances.card.colorTag", { defaultValue: "Color Tag" })}
										</span>
									</DropdownMenuSubTrigger>
									<DropdownMenuSubContent className="w-48">
										<DropdownMenuItem
											onClick={() => {
												return void handleSetColorTag(null)
											}}
											className="cursor-pointer justify-between gap-2"
										>
											<span className="flex items-center gap-2 text-xs text-muted-foreground">
												<X className="size-3.5" />
												{t("instances.colors.none", { defaultValue: "No Color" })}
											</span>
											{!instance.colorTag && <Check className="size-3.5 text-primary" />}
										</DropdownMenuItem>
										<DropdownMenuSeparator />
										{COLOR_TAGS.map((tag) => {
											const isSelected = colorTagDef?.id === tag.id
											return (
												<DropdownMenuItem
													key={tag.id}
													onClick={() => {
														return void handleSetColorTag(tag.id)
													}}
													className="cursor-pointer justify-between gap-2"
												>
													<span className="flex items-center gap-2 text-xs">
														<span className={`size-2.5 rounded-full ${tag.dotClass}`} />
														{t(tag.labelKey, { defaultValue: tag.defaultLabel })}
													</span>
													{isSelected && <Check className="size-3.5 text-primary" />}
												</DropdownMenuItem>
											)
										})}
										<DropdownMenuSeparator />
										<DropdownMenuItem
											onClick={() => {
												customColorInputRef.current?.click()
											}}
											className="cursor-pointer justify-between gap-2"
										>
											<span className="flex items-center gap-2 text-xs">
												<span
													className="size-2.5 rounded-full border border-border"
													style={{
														background: colorTagDef?.isCustom
															? colorTagDef.hex
															: "conic-gradient(from 0deg, #ef4444, #f59e0b, #10b981, #3b82f6, #8b5cf6, #ec4899, #ef4444)",
													}}
												/>
												{t("instances.colors.custom", { defaultValue: "Custom Color..." })}
											</span>
											{colorTagDef?.isCustom && <Check className="size-3.5 text-primary" />}
										</DropdownMenuItem>
									</DropdownMenuSubContent>
								</DropdownMenuSub>
								<DropdownMenuSub>
									<DropdownMenuSubTrigger className="cursor-pointer gap-2">
										<Folder className="size-4 text-muted-foreground" />
										<span className="text-xs">
											{t("instances.card.groupMenu", { defaultValue: "Group / Folder" })}
										</span>
									</DropdownMenuSubTrigger>
									<DropdownMenuSubContent className="w-52">
										<DropdownMenuItem
											onClick={() => {
												return void handleSetGroup(null)
											}}
											className="cursor-pointer justify-between gap-2"
										>
											<span className="flex items-center gap-2 text-xs text-muted-foreground">
												<X className="size-3.5" />
												{t("instances.groups.ungrouped", { defaultValue: "Ungrouped" })}
											</span>
											{!groupLabel && <Check className="size-3.5 text-primary" />}
										</DropdownMenuItem>
										{existingGroups.length > 0 && <DropdownMenuSeparator />}
										{existingGroups.map((groupName) => {
											const isCurrent = groupLabel === groupName
											return (
												<DropdownMenuItem
													key={groupName}
													onClick={() => {
														return void handleSetGroup(groupName)
													}}
													className="cursor-pointer justify-between gap-2"
												>
													<span className="flex items-center gap-2 truncate text-xs">
														<Folder className="size-3.5 shrink-0 text-amber-400" />
														<span className="truncate">{groupName}</span>
													</span>
													{isCurrent && <Check className="size-3.5 shrink-0 text-primary" />}
												</DropdownMenuItem>
											)
										})}
										<DropdownMenuSeparator />
										<DropdownMenuItem onClick={onSettings} className="cursor-pointer gap-2">
											<Settings className="size-3.5 text-muted-foreground" />
											<span className="text-xs">
												{t("instances.groups.editInSettings", {
													defaultValue: "Edit Group in Settings...",
												})}
											</span>
										</DropdownMenuItem>
									</DropdownMenuSubContent>
								</DropdownMenuSub>
								<DropdownMenuSub>
									<DropdownMenuSubTrigger className="cursor-pointer gap-2">
										<Share2 className="size-4 text-muted-foreground" />
										<span className="text-xs">{t("instances.card.createShortcut")}</span>
									</DropdownMenuSubTrigger>
									<DropdownMenuSubContent className="w-52">
										<DropdownMenuItem
											onClick={() => {
												return void handleCreateShortcut("desktop")
											}}
											className="cursor-pointer gap-2"
										>
											<Monitor className="size-4 text-primary" />
											<span className="text-xs">{t("instances.card.desktopShortcut")}</span>
										</DropdownMenuItem>
										<DropdownMenuItem
											onClick={() => {
												return void handleCreateShortcut("applicationsMenu")
											}}
											className="cursor-pointer gap-2"
										>
											<LayoutGrid className="size-4 text-primary" />
											<span className="text-xs">{t("instances.card.appMenuShortcut")}</span>
										</DropdownMenuItem>
										<DropdownMenuItem
											onClick={() => {
												return void handleCreateShortcut("steam")
											}}
											className="cursor-pointer gap-2"
										>
											<Gamepad2 className="size-4 text-primary" />
											<span className="text-xs">{t("instances.card.steamShortcut")}</span>
										</DropdownMenuItem>
									</DropdownMenuSubContent>
								</DropdownMenuSub>
							</DropdownMenuGroup>
							{!isRunning && !isDownloading && (
								<>
									<DropdownMenuSeparator />
									<DropdownMenuItem
										onClick={onDelete}
										variant="destructive"
										className="cursor-pointer gap-2"
									>
										<Trash2 className="size-4" />
										<span className="text-xs">{t("instances.card.deleteTooltip")}</span>
									</DropdownMenuItem>
								</>
							)}
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</div>

			<DirectConnectDialog
				open={directConnectOpen}
				onOpenChange={setDirectConnectOpen}
				instanceName={instance.name}
				onConnect={(addr) => onPlay({ server: addr, world: null })}
			/>
		</motion.div>
	)
}

InstanceCard.displayName = "InstanceCard"

export default memo(InstanceCard)
