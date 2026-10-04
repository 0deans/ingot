import {
	ChevronDown,
	Clock,
	Compass,
	Copy,
	FolderOpen,
	Globe,
	HardDrive,
	Play,
	Settings,
	Square,
	Trash2,
} from "lucide-react"
import { motion } from "motion/react"
import { memo, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type {
	InstanceConfig,
	InstanceWorldSummary,
	LaunchProgressEvent,
	QuickPlayOptions,
	RunningInstanceSummary,
} from "@/bindings"
import { DirectConnectDialog } from "@/components/instances/direct-connect-dialog"
import LoaderIcon from "@/components/instances/loader-icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
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
	onPlay: (quickPlay?: QuickPlayOptions) => void
	onStop: () => void
	onSettings: () => void
	onDelete: () => void
	onDuplicate: () => void
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
	onPlay,
	onStop,
	onSettings,
	onDelete,
	onDuplicate,
}: InstanceCardProps) => {
	const { t } = useTranslation()
	const isRunning = !!runningInfo
	const isDownloading = !!progress && !isRunning

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

	useEffect(() => {
		if (!isRunning || !runningInfo) {
			setElapsedSeconds(0)
			return
		}

		const update = () => {
			const now = Math.floor(Date.now() / 1000)
			setElapsedSeconds(Math.max(0, now - runningInfo.startedAt))
		}
		update()
		const timer = setInterval(update, 1000)
		return () => clearInterval(timer)
	}, [isRunning, runningInfo])

	const loaderType = instance.loader || "vanilla"
	const loaderName =
		loaderType === "vanilla"
			? t("versionChange.vanilla").toLocaleUpperCase()
			: String(loaderType).toUpperCase()
	const versionStr = instance.gameVersion || ""

	const hasCustomRam = instance.memoryMaxMb != null
	const effectiveRamMb = instance.memoryMaxMb ?? globalMaxRamMb

	return (
		<motion.div
			whileHover={{ y: -2, transition: { duration: 0.15 } }}
			className={`group relative flex flex-col justify-between overflow-hidden rounded-2xl border bg-background/60 p-5 transition-[border-color,background-color,box-shadow] duration-200 hover:border-input/80 hover:bg-card/50 hover:shadow-xl ${
				isRunning
					? "border-primary/40 ring-1 ring-primary/20"
					: isDownloading
						? "border-warning/40 ring-1 ring-warning/20"
						: "border-border/50"
			}`}
		>
			{/* Top Bar: Loader Badge & Status */}
			<div className="flex items-start justify-between gap-3">
				<div className="flex items-center gap-2.5">
					<div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-border bg-card shadow-inner">
						<LoaderIcon loader={loaderType} size={22} />
					</div>
					<div className="flex flex-col">
						<span className="font-semibold text-2xs text-muted-foreground uppercase tracking-wider">
							{loaderName}
						</span>
						<span className="font-mono text-foreground/80 text-xs">{versionStr}</span>
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
					className="truncate font-semibold text-base text-foreground tracking-tight"
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
								<span className="ml-1 font-medium text-3xs text-info">
									{t("instances.card.customRam")}
								</span>
							)}
						</span>
					</span>

					{instance.totalPlayTimeSeconds > 0 && (
						<span className="inline-flex items-center gap-1 rounded-md border border-border/80 bg-card/60 px-2 py-0.5">
							<Clock className="size-3 text-primary" />
							<span>
								{t("instances.card.playedDuration", {
									duration: formatDuration(instance.totalPlayTimeSeconds),
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
			<div className="flex items-center justify-between border-border/40 border-t pt-3.5">
				{/* Primary Launch Action */}
				<div>
					{isRunning ? (
						<Button
							size="sm"
							variant="destructive"
							onClick={onStop}
							className="h-8 gap-1.5 px-3.5 font-medium text-xs shadow-sm hover:scale-[1.02] active:scale-[0.98]"
						>
							<Square className="size-3.5 fill-current" />
							<span>{t("instances.card.stop")}</span>
						</Button>
					) : isDownloading ? (
						<Button
							size="sm"
							disabled
							className="h-8 gap-1.5 px-3.5 font-medium text-xs opacity-75"
						>
							<Spinner className="size-3.5" />
							<span>{t("instances.card.preparingBtn")}</span>
						</Button>
					) : (
						<div className="inline-flex items-center rounded-md shadow-md">
							<Button
								size="sm"
								onClick={() => onPlay()}
								className="h-8 gap-1.5 rounded-r-none border-primary/60 border-r px-3.5 font-semibold text-xs active:scale-[0.98]"
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
										<DropdownMenuLabel className="font-semibold text-3xs text-muted-foreground uppercase tracking-wider">
											{t("instances.card.quickPlay")}
										</DropdownMenuLabel>
										<DropdownMenuItem
											onClick={() => setDirectConnectOpen(true)}
											className="cursor-pointer gap-2"
										>
											<Globe className="size-4 text-primary" />
											<div className="flex flex-col">
												<span className="font-medium text-xs">
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
														<span className="font-medium text-xs">
															{t("instances.card.quickLoadWorld")}
														</span>
														<span className="text-3xs text-muted-foreground">
															{t("instances.card.quickLoadWorldDesc")}
														</span>
													</div>
												</DropdownMenuSubTrigger>
												<DropdownMenuSubContent className="max-h-72 w-64 overflow-y-auto">
													<DropdownMenuGroup>
														<DropdownMenuLabel className="font-semibold text-3xs text-muted-foreground uppercase tracking-wider">
															{t("instances.card.singleplayerSaves")}
														</DropdownMenuLabel>
														{loadingWorlds ? (
															<div className="flex items-center gap-2 p-2.5 text-muted-foreground text-xs">
																<Spinner className="size-3.5" />
																<span>{t("instances.card.scanningWorlds")}</span>
															</div>
														) : worlds.length === 0 ? (
															<div className="p-2.5 text-muted-foreground text-xs">
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
																		<span className="truncate font-medium text-foreground text-xs">
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
													<span className="font-medium text-xs">
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
									onClick={() => instanceService.openInstanceFolder(instance.id)}
									className="size-8 text-muted-foreground"
									aria-label={t("instances.card.openFolderTooltip")}
								/>
							}
						>
							<FolderOpen className="size-3.5" />
						</TooltipTrigger>
						<TooltipContent>{t("instances.card.openFolderTooltip")}</TooltipContent>
					</Tooltip>

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="ghost"
									size="icon"
									onClick={onDuplicate}
									className="size-8 text-muted-foreground"
									aria-label={t("instances.card.duplicateTooltip")}
								/>
							}
						>
							<Copy className="size-3.5" />
						</TooltipTrigger>
						<TooltipContent>{t("instances.card.duplicateTooltip")}</TooltipContent>
					</Tooltip>

					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="ghost"
									size="icon"
									onClick={onSettings}
									className="size-8 text-muted-foreground"
									aria-label={t("instances.card.settingsTooltip")}
								/>
							}
						>
							<Settings className="size-3.5" />
						</TooltipTrigger>
						<TooltipContent>{t("instances.card.settingsTooltip")}</TooltipContent>
					</Tooltip>

					{!isRunning && !isDownloading && (
						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="ghost"
										size="icon"
										onClick={onDelete}
										className="size-8 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
										aria-label={t("instances.card.deleteTooltip")}
									/>
								}
							>
								<Trash2 className="size-3.5" />
							</TooltipTrigger>
							<TooltipContent>{t("instances.card.deleteTooltip")}</TooltipContent>
						</Tooltip>
					)}
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

export default memo(InstanceCard)
