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
import { SectionCardHeader } from "@/components/common/section-card"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
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
		<Card>
			<SectionCardHeader
				icon={ArrowUpCircle}
				title={t("settings.updates.title")}
				description={t("settings.updates.description")}
			/>
			<CardContent className="flex flex-col gap-4">
				<div className="flex flex-col gap-3 rounded-lg border border-border/30 bg-background/60 p-3.5">
					<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
						<div className="flex items-center gap-3">
							<div className="flex size-9 items-center justify-center rounded-lg bg-card text-primary">
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
							className="h-8 gap-1.5 text-xs"
						>
							<RefreshCw className={cn("size-3.5", isChecking && "animate-spin text-primary")} />
							<span>
								{isChecking
									? t("settings.updates.checking")
									: t("settings.updates.checkForUpdates")}
							</span>
						</Button>
					</div>

					{/* Available Update Panel */}
					{isAvailable && updateInfo && (
						<div className="mt-2 flex flex-col gap-2.5 rounded-lg border border-primary/30 bg-primary/10 p-3">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<Sparkles className="size-4 text-primary" />
									<span className="font-medium text-primary text-xs">
										{t("settings.updates.versionAvailable", { version: updateInfo.version })}
									</span>
								</div>
								<a
									href={`https://github.com/0deans/ingot/releases/tag/v${updateInfo.version}`}
									target="_blank"
									rel="noreferrer"
									className="flex items-center gap-1 text-[11px] text-primary/80 hover:text-primary hover:underline"
								>
									<span>{t("settings.updates.changelog")}</span>
									<ExternalLink className="size-3" />
								</a>
							</div>

							{updateInfo.body && (
								<p className="line-clamp-3 whitespace-pre-line font-sans text-[11px] text-foreground/80">
									{updateInfo.body}
								</p>
							)}

							<div className="pt-1">
								<Button
									size="sm"
									onClick={() => downloadAndInstall()}
									className="h-8 gap-1.5 font-medium text-xs"
								>
									<Download className="size-3.5" />
									<span>{t("settings.updates.downloadAndInstall")}</span>
								</Button>
							</div>
						</div>
					)}

					{/* Downloading Panel */}
					{isDownloading && (
						<div className="mt-2 flex flex-col gap-2 rounded-lg border border-info/30 bg-info/10 p-3">
							<div className="flex items-center justify-between text-xs">
								<div className="flex items-center gap-2 text-info">
									<Spinner className="size-3.5" />
									<span className="font-medium">
										{t("settings.updates.downloading", { version: updateInfo?.version })}
									</span>
								</div>
								<span className="font-mono text-info text-xs">{downloadProgress}%</span>
							</div>
							<Progress value={downloadProgress} />
						</div>
					)}

					{/* Ready to Restart Panel */}
					{isReady && (
						<div className="mt-2 flex items-center justify-between rounded-lg border border-primary/40 bg-primary/10 p-3">
							<div className="flex items-center gap-2.5">
								<CheckCircle2 className="size-4 text-primary" />
								<div>
									<div className="font-medium text-primary text-xs">
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
								className="h-8 gap-1.5 font-medium text-xs"
							>
								<RefreshCw className="size-3.5" />
								<span>{t("settings.updates.restartIngot")}</span>
							</Button>
						</div>
					)}

					{/* Up to Date confirmation */}
					{status === "up-to-date" && (
						<div className="mt-1 flex items-center gap-2 text-primary/90 text-xs">
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
			</CardContent>
		</Card>
	)
}

UpdateSettings.displayName = "UpdateSettings"
export default memo(UpdateSettings)
