import { CheckCircle2, Download, RefreshCw, Sparkles, X } from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Spinner } from "@/components/ui/spinner"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"
import { useUpdateService } from "@/services/update-service"

export const UpdateBanner = () => {
	const { t } = useTranslation()
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
		<div className="relative z-50 w-full border-border/40 border-b bg-gradient-to-r from-primary/40 via-card/60 to-primary/30 px-4 py-2 text-foreground backdrop-blur-md transition-all">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div className="flex items-center gap-2.5">
					<div
						className={cn(
							"flex size-6 shrink-0 items-center justify-center rounded-full text-xs",
							isReady
								? "bg-primary/20 text-primary"
								: isDownloading
									? "bg-info/20 text-info"
									: "bg-primary/20 text-primary",
						)}
					>
						{isReady ? (
							<CheckCircle2 className="size-3.5" />
						) : isDownloading ? (
							<Spinner className="size-3.5" />
						) : (
							<Sparkles className="size-3.5" />
						)}
					</div>

					<div className="flex flex-col sm:flex-row sm:items-center sm:gap-2">
						<span className="font-medium text-xs">
							{isReady
								? t("updateBanner.readyToInstall", { version: updateInfo.version })
								: isDownloading
									? t("updateBanner.downloading", {
											version: updateInfo.version,
											progress: downloadProgress,
										})
									: t("updateBanner.available", { version: updateInfo.version })}
						</span>
						{!isDownloading && !isReady && (
							<span className="text-[11px] text-muted-foreground">
								{t("updateBanner.currentVersion", { version: updateInfo.currentVersion })}
							</span>
						)}
					</div>
				</div>

				<div className="flex items-center gap-2">
					{isReady ? (
						<Button
							size="sm"
							onClick={() => relaunchApp()}
							className="h-7 gap-1.5 px-3 font-medium text-xs"
						>
							<RefreshCw className="size-3" />
							<span>{t("updateBanner.restartNow")}</span>
						</Button>
					) : isDownloading ? (
						<div className="flex items-center gap-2">
							<Progress value={downloadProgress} className="w-24 sm:w-32" />
							<span className="font-mono text-[11px] text-muted-foreground">
								{downloadProgress}%
							</span>
						</div>
					) : (
						<Button
							size="sm"
							onClick={() => downloadAndInstall()}
							className="h-7 gap-1.5 px-3 font-medium text-xs"
						>
							<Download className="size-3" />
							<span>{t("updateBanner.updateNow")}</span>
						</Button>
					)}

					{!isDownloading && (
						<Tooltip>
							<TooltipTrigger
								render={
									<button
										type="button"
										onClick={dismissBanner}
										className="flex size-6 items-center justify-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
										aria-label={t("updateBanner.dismiss")}
									/>
								}
							>
								<X className="size-3.5" />
							</TooltipTrigger>
							<TooltipContent>{t("updateBanner.dismiss")}</TooltipContent>
						</Tooltip>
					)}
				</div>
			</div>
		</div>
	)
}

UpdateBanner.displayName = "UpdateBanner"
export default memo(UpdateBanner)
