import {
	ChevronDown,
	Clock,
	Compass,
	FolderOpen,
	Globe,
	HardDrive,
	Loader2,
	Play,
	Settings,
	Square,
	Trash2,
} from "lucide-react"
import { memo, useEffect, useState } from "react"
import type {
	InstanceConfig,
	InstanceWorldSummary,
	LaunchProgressEvent,
	QuickPlayOptions,
	RunningInstanceSummary,
} from "@/bindings"
import { DirectConnectDialog } from "@/components/instances/direct-connect-dialog"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
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
	if (seconds < 60) return `${seconds}s`
	const minutes = Math.floor(seconds / 60)
	if (minutes < 60) return `${minutes}m ${seconds % 60}s`
	const hours = Math.floor(minutes / 60)
	const remMinutes = minutes % 60
	return `${hours}h ${remMinutes}m`
}

function formatLastPlayed(timestamp?: number | null): string {
	if (!timestamp) return "Never played"
	const diff = Math.floor(Date.now() / 1000) - timestamp
	if (diff < 60) return "Just now"
	if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
	if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
	return new Date(timestamp * 1000).toLocaleDateString()
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
}: InstanceCardProps) => {
	const isRunning = !!runningInfo
	const isDownloading = !!progress && !isRunning

	const [elapsedSeconds, setElapsedSeconds] = useState(0)
	const [worlds, setWorlds] = useState<InstanceWorldSummary[]>([])
	const [loadingWorlds, setLoadingWorlds] = useState(false)
	const [directConnectOpen, setDirectConnectOpen] = useState(false)

	const supportsSingleplayerQP = isVersionAtLeast(instance.gameVersion, 1, 20)

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
	const loaderName = loaderType === "vanilla" ? "Vanilla" : String(loaderType).toUpperCase()
	const versionStr = instance.gameVersion || ""

	const hasCustomRam = instance.memoryMaxMb != null
	const effectiveRamMb = instance.memoryMaxMb ?? globalMaxRamMb
	const effectiveRamGb = (effectiveRamMb / 1024).toFixed(1)

	return (
		<div
			className={`group relative flex flex-col justify-between overflow-hidden rounded-2xl border bg-zinc-950/60 p-5 transition-all duration-200 hover:border-zinc-700/80 hover:bg-zinc-900/50 hover:shadow-xl ${
				isRunning
					? "border-emerald-500/40 shadow-emerald-950/20 ring-1 ring-emerald-500/20"
					: isDownloading
						? "border-amber-500/40 ring-1 ring-amber-500/20"
						: "border-border/50"
			}`}
		>
			{/* Top Bar: Loader Badge & Status */}
			<div className="flex items-start justify-between gap-3">
				<div className="flex items-center gap-2.5">
					<div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 shadow-inner">
						<LoaderIcon loader={loaderType} size={22} />
					</div>
					<div className="flex flex-col">
						<span className="font-semibold text-[11px] text-zinc-400 uppercase tracking-wider">
							{loaderName}
						</span>
						<span className="font-mono text-xs text-zinc-300">{versionStr}</span>
					</div>
				</div>

				{/* Status Pill */}
				<div>
					{isRunning ? (
						<span className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-1 font-medium text-[11px] text-emerald-400 shadow-sm">
							<span className="size-1.5 animate-pulse rounded-full bg-emerald-400" />
							{formatDuration(elapsedSeconds)}
						</span>
					) : isDownloading ? (
						<span className="inline-flex items-center gap-1.5 rounded-full border border-amber-500/30 bg-amber-500/10 px-2.5 py-1 font-medium text-[11px] text-amber-400 shadow-sm">
							<Loader2 className="size-3 animate-spin" />
							{progress.percentage != null ? `${progress.percentage.toFixed(0)}%` : "Preparing"}
						</span>
					) : (
						<span className="inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/80 px-2.5 py-0.5 text-[11px] text-zinc-400">
							{formatLastPlayed(instance.lastPlayed)}
						</span>
					)}
				</div>
			</div>

			{/* Main Info */}
			<div className="my-4 flex flex-col gap-2">
				<h3
					className="truncate font-semibold text-base text-white tracking-tight"
					title={instance.name}
				>
					{instance.name}
				</h3>

				<div className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
					<span
						className="inline-flex items-center gap-1 rounded-md border border-zinc-800/80 bg-zinc-900/60 px-2 py-0.5"
						title={hasCustomRam ? "Per-instance memory override" : "Inherited from global settings"}
					>
						<HardDrive className="size-3 text-sky-400" />
						<span>
							{effectiveRamGb} GB RAM
							{hasCustomRam && (
								<span className="ml-1 font-medium text-[10px] text-sky-400">(Custom)</span>
							)}
						</span>
					</span>

					{instance.totalPlayTimeSeconds > 0 && (
						<span className="inline-flex items-center gap-1 rounded-md border border-zinc-800/80 bg-zinc-900/60 px-2 py-0.5">
							<Clock className="size-3 text-emerald-400" />
							<span>{formatDuration(instance.totalPlayTimeSeconds)} played</span>
						</span>
					)}
				</div>

				{/* Progress Track if downloading */}
				{isDownloading && (
					<div className="mt-1 flex flex-col gap-1.5 rounded-lg border border-amber-500/20 bg-zinc-950/60 p-2.5 text-[11px]">
						<div className="flex items-center justify-between font-medium text-amber-300">
							<span className="truncate">{progress.phase}</span>
							<span className="font-mono">
								{progress.percentage != null ? `${progress.percentage.toFixed(0)}%` : ""}
							</span>
						</div>
						<div className="h-1.5 w-full overflow-hidden rounded-full bg-zinc-800">
							<div
								className="h-full rounded-full bg-amber-400 transition-all duration-200"
								style={{
									width: `${Math.min(100, Math.max(0, progress.percentage ?? 0))}%`,
								}}
							/>
						</div>
						<span className="truncate font-mono text-[10px] text-zinc-400">{progress.detail}</span>
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
							<span>Stop</span>
						</Button>
					) : isDownloading ? (
						<Button
							size="sm"
							disabled
							className="h-8 gap-1.5 px-3.5 font-medium text-xs opacity-75"
						>
							<Loader2 className="size-3.5 animate-spin" />
							<span>Preparing...</span>
						</Button>
					) : (
						<div className="inline-flex items-center rounded-md shadow-emerald-950/40 shadow-md">
							<Button
								size="sm"
								onClick={() => onPlay()}
								className="h-8 gap-1.5 rounded-r-none border-emerald-700/60 border-r bg-emerald-600 px-3.5 font-semibold text-white text-xs hover:bg-emerald-500 active:scale-[0.98]"
							>
								<Play className="size-3.5 fill-current" />
								<span>Play</span>
							</Button>
							<DropdownMenu
								onOpenChange={(open) => {
									if (open) loadWorlds()
								}}
							>
								<DropdownMenuTrigger
									render={
										<Button
											size="sm"
											className="h-8 rounded-l-none bg-emerald-600 px-1.5 text-white hover:bg-emerald-500"
											title="Quick Play options"
										>
											<ChevronDown className="size-3.5" />
										</Button>
									}
								/>
								<DropdownMenuContent align="start" className="w-56">
									<DropdownMenuLabel className="font-semibold text-[10px] text-zinc-400 uppercase tracking-wider">
										Quick Play
									</DropdownMenuLabel>
									<DropdownMenuItem
										onClick={() => setDirectConnectOpen(true)}
										className="cursor-pointer gap-2"
									>
										<Globe className="size-4 text-emerald-400" />
										<div className="flex flex-col">
											<span className="font-medium text-xs">Direct Connect...</span>
											<span className="text-[10px] text-muted-foreground">
												Join a server directly
											</span>
										</div>
									</DropdownMenuItem>

									{supportsSingleplayerQP ? (
										<DropdownMenuSub>
											<DropdownMenuSubTrigger className="cursor-pointer gap-2">
												<Compass className="size-4 text-sky-400" />
												<div className="flex flex-col text-left">
													<span className="font-medium text-xs">Quick Load World</span>
													<span className="text-[10px] text-muted-foreground">
														Jump straight into a save
													</span>
												</div>
											</DropdownMenuSubTrigger>
											<DropdownMenuSubContent className="max-h-72 w-64 overflow-y-auto">
												<DropdownMenuLabel className="font-semibold text-[10px] text-zinc-400 uppercase tracking-wider">
													Singleplayer Saves
												</DropdownMenuLabel>
												{loadingWorlds ? (
													<div className="flex items-center gap-2 p-2.5 text-muted-foreground text-xs">
														<Loader2 className="size-3.5 animate-spin" />
														<span>Scanning worlds...</span>
													</div>
												) : worlds.length === 0 ? (
													<div className="p-2.5 text-xs text-zinc-400">
														No saved worlds found in this instance.
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
																	className="size-7 shrink-0 rounded border border-zinc-800 object-cover"
																/>
															) : (
																<div className="flex size-7 shrink-0 items-center justify-center rounded border border-zinc-800 bg-zinc-900 text-zinc-400">
																	<Compass className="size-3.5" />
																</div>
															)}
															<div className="flex min-w-0 flex-col truncate">
																<span className="truncate font-medium text-xs text-zinc-200">
																	{w.displayName}
																</span>
																<span className="truncate text-[10px] text-zinc-500">
																	{w.lastPlayed ? formatLastPlayed(w.lastPlayed) : w.folderName}
																</span>
															</div>
														</DropdownMenuItem>
													))
												)}
											</DropdownMenuSubContent>
										</DropdownMenuSub>
									) : (
										<DropdownMenuItem disabled className="gap-2 opacity-50">
											<Compass className="size-4 text-zinc-500" />
											<div className="flex flex-col">
												<span className="font-medium text-xs">Quick Load World</span>
												<span className="text-[10px] text-zinc-500">Requires Minecraft 1.20+</span>
											</div>
										</DropdownMenuItem>
									)}
								</DropdownMenuContent>
							</DropdownMenu>
						</div>
					)}
				</div>

				{/* Utility Actions */}
				<div className="flex items-center gap-1">
					<Button
						variant="ghost"
						size="icon"
						onClick={() => instanceService.openInstanceFolder(instance.id)}
						className="size-8 text-zinc-400 hover:bg-zinc-800 hover:text-white"
						title="Open instance folder in File Explorer"
					>
						<FolderOpen className="size-3.5" />
					</Button>

					<Button
						variant="ghost"
						size="icon"
						onClick={onSettings}
						className="size-8 text-zinc-400 hover:bg-zinc-800 hover:text-white"
						title="Instance Settings (RAM, JVM args)"
					>
						<Settings className="size-3.5" />
					</Button>

					{!isRunning && !isDownloading && (
						<Button
							variant="ghost"
							size="icon"
							onClick={onDelete}
							className="size-8 text-zinc-400 hover:bg-destructive/15 hover:text-destructive"
							title="Delete Instance"
						>
							<Trash2 className="size-3.5" />
						</Button>
					)}
				</div>
			</div>

			<DirectConnectDialog
				open={directConnectOpen}
				onOpenChange={setDirectConnectOpen}
				instanceName={instance.name}
				onConnect={(addr) => onPlay({ server: addr, world: null })}
			/>
		</div>
	)
}

export default memo(InstanceCard)
