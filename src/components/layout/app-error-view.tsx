import { openUrl } from "@tauri-apps/plugin-opener"
import { cn } from "cn"
import {
	AlertTriangle,
	Check,
	ChevronDown,
	Copy,
	ExternalLink,
	Home,
	RefreshCw,
	RotateCcw,
	Terminal,
} from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { memo, useCallback, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import {
	buildDiagnosticReport,
	buildGitHubIssueUrl,
	collectDeviceDetails,
	getErrorMessage,
	getVersionString,
} from "@/lib/error-report"

export interface AppErrorViewProps {
	error: unknown
	reset?: () => void
	info?: {
		componentStack?: string
	}
	isRoot?: boolean
	onGoHome?: () => void
}

const AppErrorView = ({ error, reset, info, isRoot = false, onGoHome }: AppErrorViewProps) => {
	const { t } = useTranslation()

	const [isDetailsOpen, setIsDetailsOpen] = useState(false)
	const [hasCopied, setHasCopied] = useState(false)

	const errorMessage = useMemo(() => {
		return getErrorMessage(error)
	}, [error])

	const diagnosticInfo = useMemo(() => {
		return buildDiagnosticReport(error, info)
	}, [error, info])

	const appVersion = useMemo(() => {
		return getVersionString()
	}, [])

	const deviceDetails = useMemo(() => {
		return collectDeviceDetails()
	}, [])

	const handleTryAgain = useCallback(() => {
		if (reset) {
			reset()
		} else {
			window.location.reload()
		}
	}, [reset])

	const handleReload = useCallback(() => {
		window.location.reload()
	}, [])

	const handleGoHome = useCallback(() => {
		if (onGoHome) {
			onGoHome()
			return
		}
		window.location.reload()
	}, [onGoHome])

	const handleCopyDetails = useCallback(() => {
		if (typeof navigator !== "undefined" && navigator.clipboard) {
			navigator.clipboard
				.writeText(diagnosticInfo)
				.then(() => {
					setHasCopied(true)
					toast.success(t("appError.copied", "Copied to Clipboard"))
				})
				.catch(() => {
					toast.error(t("appError.copyFailed", "Failed to copy error details"))
				})
		}
	}, [diagnosticInfo, t])

	const handleToggleDetails = useCallback(() => {
		setIsDetailsOpen((prev) => {
			return !prev
		})
	}, [])

	const handleReportIssue = useCallback(() => {
		const issueUrl = buildGitHubIssueUrl(error, info)
		openUrl(issueUrl).catch(() => {
			window.open(issueUrl, "_blank")
		})
	}, [error, info])

	useEffect(() => {
		if (!hasCopied) return

		const timer = setTimeout(() => {
			setHasCopied(false)
		}, 2000)

		return () => {
			clearTimeout(timer)
		}
	}, [hasCopied])

	return (
		<div
			data-slot="app-error-view"
			className="flex size-full min-h-0 flex-1 flex-col items-center overflow-y-auto px-4 py-4 select-none sm:px-6"
		>
			<div className="my-auto flex w-full max-w-xl flex-col items-center py-4 text-center">
				{/* Warning Badge & Glowing Icon */}
				<div className="relative mb-3 flex size-12 items-center justify-center rounded-xl bg-destructive/10 text-destructive shadow-md ring-1 shadow-destructive/10 ring-destructive/20">
					<AlertTriangle className="size-6 stroke-[2.2]" />
				</div>

				<div className="mb-2 inline-flex items-center gap-1.5 rounded-full border border-destructive/20 bg-destructive/10 px-2.5 py-0.5 text-[11px] font-semibold text-destructive">
					<span>{t("appError.badge", "Application Error")}</span>
				</div>

				{/* Headings */}
				<h1 className="font-heading text-xl font-bold tracking-tight text-foreground sm:text-2xl">
					{t("appError.title", "Something went wrong")}
				</h1>

				<p className="mt-1.5 max-w-md text-xs leading-relaxed text-balance text-muted-foreground sm:text-sm">
					{t(
						"appError.description",
						"Ingot encountered an unexpected error while displaying this page.",
					)}{" "}
					{t("appError.safeNotice", "Your instances, saves, and accounts remain completely safe.")}
				</p>

				{/* Error message card */}
				<div className="mt-4 w-full rounded-xl border border-destructive/20 bg-destructive/5 p-3 text-left shadow-xs">
					<div className="flex items-start gap-2.5">
						<Terminal className="mt-0.5 size-4 shrink-0 text-destructive" />
						<div className="min-w-0 flex-1">
							<p className="text-[10px] font-semibold tracking-wider text-destructive uppercase">
								{t("appError.errorLabel", "Error Message")}
							</p>
							<p className="mt-0.5 font-mono text-xs leading-relaxed font-medium break-words text-foreground/90 select-text">
								{errorMessage}
							</p>
						</div>
					</div>
				</div>

				{/* Primary action buttons */}
				<div className="mt-5 flex flex-wrap items-center justify-center gap-2">
					{reset ? (
						<Button
							variant="default"
							size="sm"
							onClick={handleTryAgain}
							className="cursor-pointer gap-2 shadow-xs"
						>
							<RefreshCw className="size-3.5" />
							<span>{t("appError.tryAgain", "Try Again")}</span>
						</Button>
					) : null}

					{!isRoot ? (
						<Button
							variant="secondary"
							size="sm"
							onClick={handleGoHome}
							className="cursor-pointer gap-2"
						>
							<Home className="size-3.5" />
							<span>{t("appError.goHome", "Return to Home")}</span>
						</Button>
					) : null}

					<Button
						variant="outline"
						size="sm"
						onClick={handleReload}
						className="cursor-pointer gap-2"
					>
						<RotateCcw className="size-3.5" />
						<span>{t("appError.reload", "Reload Launcher")}</span>
					</Button>
				</div>

				{/* Secondary utility actions */}
				<div className="mt-3 flex flex-wrap items-center justify-center gap-1">
					<Button
						variant="ghost"
						size="xs"
						onClick={handleCopyDetails}
						className="h-7 cursor-pointer gap-1.5 text-xs text-muted-foreground hover:text-foreground"
					>
						{hasCopied ? (
							<>
								<Check className="size-3 text-emerald-500" />
								<span className="text-emerald-500">
									{t("appError.copied", "Copied to Clipboard")}
								</span>
							</>
						) : (
							<>
								<Copy className="size-3" />
								<span>{t("appError.copyDetails", "Copy Error Details")}</span>
							</>
						)}
					</Button>

					<Button
						variant="ghost"
						size="xs"
						onClick={handleToggleDetails}
						className="h-7 cursor-pointer gap-1.5 text-xs text-muted-foreground hover:text-foreground"
					>
						<ChevronDown
							className={cn(
								"size-3 transition-transform duration-200",
								isDetailsOpen && "rotate-180",
							)}
						/>
						<span>
							{isDetailsOpen
								? t("appError.hideDetails", "Hide Technical Details")
								: t("appError.showDetails", "Show Technical Details")}
						</span>
					</Button>

					<Button
						variant="ghost"
						size="xs"
						onClick={handleReportIssue}
						className="h-7 cursor-pointer gap-1.5 text-xs text-muted-foreground hover:text-foreground"
					>
						<ExternalLink className="size-3" />
						<span>{t("appError.reportIssue", "Report Issue")}</span>
					</Button>
				</div>

				{/* Animated collapsible diagnostic details */}
				<AnimatePresence initial={false}>
					{isDetailsOpen ? (
						<motion.div
							key="technical-details-panel"
							initial={{ opacity: 0, height: 0 }}
							animate={{ opacity: 1, height: "auto" }}
							exit={{ opacity: 0, height: 0 }}
							transition={{ duration: 0.22, ease: [0.16, 1, 0.3, 1] }}
							className="w-full overflow-hidden"
						>
							<div className="mt-3 w-full rounded-xl border border-border/60 bg-muted/30 p-3 text-left shadow-inner">
								<div className="flex items-center justify-between border-b border-border/40 pb-2">
									<span className="text-xs font-semibold text-muted-foreground">
										{t("appError.stackLabel", "Diagnostic Information")}
									</span>
									<span className="font-mono text-[10px] text-muted-foreground/70">
										Ingot v{appVersion}
									</span>
								</div>

								{/* Quick Device Overview Badges */}
								<div className="mt-2.5 grid grid-cols-2 gap-1.5 text-left sm:grid-cols-4">
									<div className="rounded-lg border border-border/40 bg-background/60 p-1.5">
										<span className="block text-[9px] font-semibold text-muted-foreground uppercase">
											{t("appError.deviceOs", "OS & Arch")}
										</span>
										<span className="font-mono text-[11px] font-medium text-foreground">
											{deviceDetails.os} ({deviceDetails.arch})
										</span>
									</div>

									<div className="rounded-lg border border-border/40 bg-background/60 p-1.5">
										<span className="block text-[9px] font-semibold text-muted-foreground uppercase">
											{t("appError.deviceCpuRam", "CPU & RAM")}
										</span>
										<span className="font-mono text-[11px] font-medium text-foreground">
											{deviceDetails.cpuCores} {t("appError.cores", "cores")}
											{deviceDetails.deviceMemory ? ` • ${deviceDetails.deviceMemory}` : ""}
										</span>
									</div>

									<div className="rounded-lg border border-border/40 bg-background/60 p-1.5">
										<span className="block text-[9px] font-semibold text-muted-foreground uppercase">
											{t("appError.deviceDisplay", "Display")}
										</span>
										<span className="font-mono text-[11px] font-medium text-foreground">
											{deviceDetails.screenResolution} @ {deviceDetails.pixelRatio}
										</span>
									</div>

									<div className="rounded-lg border border-border/40 bg-background/60 p-1.5">
										<span className="block text-[9px] font-semibold text-muted-foreground uppercase">
											{t("appError.deviceEnvironment", "Environment")}
										</span>
										<span className="font-mono text-[11px] font-medium text-foreground">
											{deviceDetails.appMode} • {deviceDetails.isOnline ? "Online" : "Offline"}
										</span>
									</div>
								</div>

								<pre className="mt-2.5 max-h-36 overflow-auto overscroll-contain rounded-lg border border-border/30 bg-black/60 p-2.5 font-mono text-[11px] leading-relaxed text-muted-foreground select-text sm:max-h-40">
									{diagnosticInfo}
								</pre>
							</div>
						</motion.div>
					) : null}
				</AnimatePresence>
			</div>
		</div>
	)
}

AppErrorView.displayName = "AppErrorView"

export default memo(AppErrorView)
