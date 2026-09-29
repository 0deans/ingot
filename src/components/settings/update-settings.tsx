import {
	AlertCircle,
	ArrowUpCircle,
	CheckCircle2,
	Download,
	ExternalLink,
	RefreshCw,
	Sparkles,
} from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Spinner } from "@/components/ui/spinner"
import { formatRelative } from "@/lib/format"
import { cn } from "@/lib/utils"
import { useUpdateService } from "@/services/update-service"

export const UpdateSettings = () => {
	const { t } = useTranslation()
	const {
		status,
		updateInfo,
		downloadProgress,
		lastCheckedAt,
		errorMessage,
		checkForUpdates,
		downloadAndInstall,
		relaunchApp,
	} = useUpdateService()

	const isChecking = status === "checking"
	const isDownloading = status === "downloading"
	const isReady = status === "ready"
	const isAvailable = status === "available"

	const formatLastChecked = (timestamp: number | null) => {
		if (!timestamp) return t("common.never")
		return formatRelative(timestamp)
	}

	return (
		<div className="flex flex-col gap-4 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
			<div className="flex flex-col gap-1">
				<div className="flex items-center gap-2">
					<ArrowUpCircle className="size-4 text-emerald-400" />
					<h3 className="font-semibold text-foreground text-sm">{t("settings.updates.title")}</h3>
				</div>
				<p className="text-muted-foreground text-xs">{t("settings.updates.description")}</p>
			</div>

			<div className="flex flex-col gap-3 rounded-lg border border-border/30 bg-zinc-950/60 p-3.5">
				<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
					<div className="flex items-center gap-3">
						<div className="flex size-9 items-center justify-center rounded-lg bg-zinc-900 text-emerald-400">
							<Sparkles className="size-4.5" />
						</div>
						<div>
							<div className="flex items-center gap-2">
								<span className="font-medium text-foreground text-xs sm:text-sm">
									Ingot v{updateInfo?.currentVersion || APP_VERSION}
								</span>
								<Badge variant="secondary" className="font-mono">
									{t("settings.updates.releaseChannel")}
								</Badge>
							</div>
							<div className="text-[11px] text-muted-foreground">
								{t("settings.updates.lastChecked", { time: formatLastChecked(lastCheckedAt) })}
							</div>
						</div>
					</div>

					<Button
						variant="outline"
						size="sm"
						disabled={isChecking || isDownloading}
						onClick={() => checkForUpdates(false)}
						className="h-8 gap-1.5 border-zinc-800 bg-zinc-900/80 text-xs hover:border-zinc-700 hover:text-foreground"
					>
						<RefreshCw className={cn("size-3.5", isChecking && "animate-spin text-emerald-400")} />
						<span>
							{isChecking ? t("settings.updates.checking") : t("settings.updates.checkForUpdates")}
						</span>
					</Button>
				</div>

				{/* Available Update Panel */}
				{isAvailable && updateInfo && (
					<div className="mt-2 flex flex-col gap-2.5 rounded-lg border border-emerald-500/30 bg-emerald-950/20 p-3">
						<div className="flex items-center justify-between">
							<div className="flex items-center gap-2">
								<Sparkles className="size-4 text-emerald-400" />
								<span className="font-medium text-emerald-300 text-xs">
									{t("settings.updates.versionAvailable", { version: updateInfo.version })}
								</span>
							</div>
							<a
								href={`https://github.com/0deans/ingot/releases/tag/v${updateInfo.version}`}
								target="_blank"
								rel="noreferrer"
								className="flex items-center gap-1 text-[11px] text-emerald-400/80 hover:text-emerald-300 hover:underline"
							>
								<span>{t("settings.updates.changelog")}</span>
								<ExternalLink className="size-3" />
							</a>
						</div>

						{updateInfo.body && (
							<p className="line-clamp-3 whitespace-pre-line font-sans text-[11px] text-zinc-300/80">
								{updateInfo.body}
							</p>
						)}

						<div className="pt-1">
							<Button
								size="sm"
								onClick={() => downloadAndInstall()}
								className="h-8 gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
							>
								<Download className="size-3.5" />
								<span>{t("settings.updates.downloadAndInstall")}</span>
							</Button>
						</div>
					</div>
				)}

				{/* Downloading Panel */}
				{isDownloading && (
					<div className="mt-2 flex flex-col gap-2 rounded-lg border border-sky-500/30 bg-sky-950/20 p-3">
						<div className="flex items-center justify-between text-xs">
							<div className="flex items-center gap-2 text-sky-300">
								<Spinner className="size-3.5" />
								<span className="font-medium">
									{t("settings.updates.downloading", { version: updateInfo?.version })}
								</span>
							</div>
							<span className="font-mono text-sky-400 text-xs">{downloadProgress}%</span>
						</div>
						<Progress value={downloadProgress} />
					</div>
				)}

				{/* Ready to Restart Panel */}
				{isReady && (
					<div className="mt-2 flex items-center justify-between rounded-lg border border-emerald-500/40 bg-emerald-950/30 p-3">
						<div className="flex items-center gap-2.5">
							<CheckCircle2 className="size-4 text-emerald-400" />
							<div>
								<div className="font-medium text-emerald-300 text-xs">
									{t("settings.updates.installedTitle")}
								</div>
								<div className="text-[11px] text-muted-foreground">
									{t("settings.updates.installedDesc")}
								</div>
							</div>
						</div>
						<Button
							size="sm"
							onClick={() => relaunchApp()}
							className="h-8 gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
						>
							<RefreshCw className="size-3.5" />
							<span>{t("settings.updates.restartIngot")}</span>
						</Button>
					</div>
				)}

				{/* Up to Date confirmation */}
				{status === "up-to-date" && (
					<div className="mt-1 flex items-center gap-2 text-emerald-400/90 text-xs">
						<CheckCircle2 className="size-3.5 shrink-0" />
						<span>{t("settings.updates.upToDate")}</span>
					</div>
				)}

				{/* Error Panel */}
				{status === "error" && errorMessage && (
					<Alert variant="destructive" className="mt-1">
						<AlertCircle />
						<AlertTitle>{t("settings.updates.errorTitle")}</AlertTitle>
						<AlertDescription>{errorMessage}</AlertDescription>
					</Alert>
				)}
			</div>
		</div>
	)
}

UpdateSettings.displayName = "UpdateSettings"
export default memo(UpdateSettings)
