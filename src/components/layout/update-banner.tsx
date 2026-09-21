import { CheckCircle2, Download, Loader2, RefreshCw, Sparkles, X } from "lucide-react"
import { memo } from "react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { useUpdateService } from "@/services/update-service"

export const UpdateBanner = () => {
	const {
		status,
		updateInfo,
		downloadProgress,
		isBannerDismissed,
		downloadAndInstall,
		relaunchApp,
		dismissBanner,
	} = useUpdateService()

	if (isBannerDismissed || !updateInfo) {
		return null
	}

	if (status !== "available" && status !== "downloading" && status !== "ready") {
		return null
	}

	const isDownloading = status === "downloading"
	const isReady = status === "ready"

	return (
		<div className="relative z-50 w-full border-border/40 border-b bg-gradient-to-r from-emerald-950/40 via-zinc-900/60 to-emerald-950/30 px-4 py-2 text-foreground backdrop-blur-md transition-all">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div className="flex items-center gap-2.5">
					<div
						className={cn(
							"flex size-6 shrink-0 items-center justify-center rounded-full text-xs",
							isReady
								? "bg-emerald-500/20 text-emerald-400"
								: isDownloading
									? "bg-sky-500/20 text-sky-400"
									: "bg-emerald-500/20 text-emerald-400",
						)}
					>
						{isReady ? (
							<CheckCircle2 className="size-3.5" />
						) : isDownloading ? (
							<Loader2 className="size-3.5 animate-spin" />
						) : (
							<Sparkles className="size-3.5" />
						)}
					</div>

					<div className="flex flex-col sm:flex-row sm:items-center sm:gap-2">
						<span className="font-medium text-xs">
							{isReady
								? `Ingot v${updateInfo.version} is ready to install!`
								: isDownloading
									? `Downloading update v${updateInfo.version}... (${downloadProgress}%)`
									: `New version available: Ingot v${updateInfo.version}`}
						</span>
						{!isDownloading && !isReady && (
							<span className="text-[11px] text-muted-foreground">
								Current: v{updateInfo.currentVersion}
							</span>
						)}
					</div>
				</div>

				<div className="flex items-center gap-2">
					{isReady ? (
						<Button
							size="sm"
							onClick={() => relaunchApp()}
							className="h-7 gap-1.5 bg-emerald-600 px-3 font-medium text-white text-xs hover:bg-emerald-500"
						>
							<RefreshCw className="size-3" />
							<span>Restart Now</span>
						</Button>
					) : isDownloading ? (
						<div className="flex items-center gap-2">
							<div className="h-1.5 w-24 overflow-hidden rounded-full bg-zinc-800 sm:w-32">
								<div
									className="h-full bg-emerald-500 transition-all duration-200"
									style={{ width: `${downloadProgress}%` }}
								/>
							</div>
							<span className="font-mono text-[11px] text-muted-foreground">
								{downloadProgress}%
							</span>
						</div>
					) : (
						<Button
							size="sm"
							onClick={() => downloadAndInstall()}
							className="h-7 gap-1.5 bg-emerald-600 px-3 font-medium text-white text-xs hover:bg-emerald-500"
						>
							<Download className="size-3" />
							<span>Update Now</span>
						</Button>
					)}

					{!isDownloading && (
						<button
							type="button"
							onClick={dismissBanner}
							className="flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-zinc-800 hover:text-foreground"
							title="Dismiss"
						>
							<X className="size-3.5" />
						</button>
					)}
				</div>
			</div>
		</div>
	)
}

UpdateBanner.displayName = "UpdateBanner"
export default memo(UpdateBanner)
