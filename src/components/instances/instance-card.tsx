import { Clock, FolderOpen, HardDrive, Loader2, Play, Settings, Square, Trash2 } from "lucide-react"
import { memo, useEffect, useState } from "react"
import type { InstanceConfig, LaunchProgressEvent, RunningInstanceSummary } from "@/bindings"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
import { instanceService } from "@/services/instance-service"

interface InstanceCardProps {
	instance: InstanceConfig
	runningInfo?: RunningInstanceSummary
	progress?: LaunchProgressEvent
	globalMaxRamMb: number
	onPlay: () => void
	onStop: () => void
	onSettings: () => void
	onDelete: () => void
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
						<Button
							size="sm"
							onClick={onPlay}
							className="h-8 gap-1.5 bg-emerald-600 px-4 font-semibold text-white text-xs shadow-emerald-950/40 shadow-md transition-all hover:scale-[1.02] hover:bg-emerald-500 active:scale-[0.98]"
						>
							<Play className="size-3.5 fill-current" />
							<span>Play</span>
						</Button>
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
		</div>
	)
}

export default memo(InstanceCard)
