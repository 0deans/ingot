import {
	AlertTriangle,
	Check,
	ChevronDown,
	ChevronRight,
	Copy,
	Home,
	RefreshCw,
	RotateCcw,
} from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import {
	buildErrorDiagnosticReport,
	type ErrorScope,
	extractErrorDetails,
} from "@/shared/lib/errorFormatting"
import { cn } from "@/shared/lib/utils"

type CopyStatus = "idle" | "copied" | "failed"

interface ErrorCopySpec {
	badge: string
	title: string
	description: string
}

const getErrorCopy = (scope: ErrorScope, isBackendLike: boolean): ErrorCopySpec => {
	if (scope === "root") {
		return {
			badge: "Recovery Mode",
			title: "The launcher interface encountered an error",
			description: isBackendLike
				? "A core service or data request failed while initializing the launcher window. Window controls remain active above."
				: "A layout component failed to render. Window controls remain active above so you can retry or reload safely.",
		}
	}

	if (scope === "shell") {
		return {
			badge: "Shell Recovery",
			title: "A shell panel failed to render",
			description:
				"The rest of the window stayed responsive. You can retry this section or continue using the launcher.",
		}
	}

	return {
		badge: "View Error",
		title: "Something went wrong in this view",
		description: isBackendLike
			? "An unexpected error occurred while communicating with the launcher backend."
			: "This page ran into a rendering error. The titlebar and navigation remain active so you can retry or switch views.",
	}
}

interface ErrorFallbackCardProps {
	error: unknown
	scope?: ErrorScope
	routePath?: string
	onRetry?: () => void
	onGoHome?: () => void
	onReloadApp?: () => void
}

const ErrorFallbackCard = ({
	error,
	scope = "route",
	routePath,
	onRetry,
	onGoHome,
	onReloadApp,
}: ErrorFallbackCardProps) => {
	const copyResetTimerRef = useRef<number | null>(null)

	const [copyStatus, setCopyStatus] = useState<CopyStatus>("idle")
	const [detailsExpanded, setDetailsExpanded] = useState(false)

	const details = useMemo(() => {
		return extractErrorDetails(error)
	}, [error])

	const diagnosticReport = useMemo(() => {
		return buildErrorDiagnosticReport({ details, scope, routePath })
	}, [details, routePath, scope])

	useEffect(() => {
		return () => {
			if (copyResetTimerRef.current !== null) {
				window.clearTimeout(copyResetTimerRef.current)
			}
		}
	}, [])

	const scheduleCopyReset = () => {
		if (copyResetTimerRef.current !== null) {
			window.clearTimeout(copyResetTimerRef.current)
		}
		copyResetTimerRef.current = window.setTimeout(() => {
			setCopyStatus("idle")
			copyResetTimerRef.current = null
		}, 1800)
	}

	const copyDiagnostics = async () => {
		try {
			await navigator.clipboard.writeText(diagnosticReport)
			setCopyStatus("copied")
		} catch {
			setCopyStatus("failed")
		}
		scheduleCopyReset()
	}

	const handleCopyClick = () => {
		void copyDiagnostics()
	}

	const copySpec = getErrorCopy(scope, details.isBackendLike)
	const hasTechnicalDetails = Boolean(details.stack || routePath)

	return (
		<div className="ingot-glass relative w-full max-w-xl overflow-hidden rounded-2xl border border-border/80 p-6 shadow-xl select-none">
			<div className="flex items-start gap-3.5">
				<div className="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-rose-500/25 bg-rose-500/12 text-rose-500">
					<AlertTriangle className="size-5" />
				</div>

				<div className="min-w-0 flex-1">
					<div className="flex flex-wrap items-center gap-2">
						<span className="rounded-full border border-rose-500/25 bg-rose-500/10 px-2 py-0.5 text-[10px] font-semibold tracking-wide text-rose-500 uppercase">
							{copySpec.badge}
						</span>
						{routePath ? (
							<span className="rounded-full border border-border bg-foreground/5 px-2 py-0.5 font-mono text-[10px] text-muted-foreground">
								{routePath}
							</span>
						) : null}
					</div>

					<h1 className="mt-2 text-sm font-semibold text-foreground">{copySpec.title}</h1>
					<p className="mt-1 text-xs leading-relaxed text-muted-foreground">
						{copySpec.description}
					</p>
				</div>
			</div>

			<div className="mt-4 rounded-xl border border-border bg-black/5 px-3.5 py-2.5 dark:bg-black/30">
				<p className="line-clamp-3 font-mono text-xs leading-relaxed wrap-break-word text-foreground/95 select-text">
					{details.summary}
				</p>
			</div>

			<div className="mt-3 rounded-xl border border-border bg-foreground/3">
				<div className="flex items-center justify-between gap-2 px-3 py-2">
					{hasTechnicalDetails ? (
						<button
							type="button"
							aria-expanded={detailsExpanded}
							onClick={() => {
								setDetailsExpanded((prev) => {
									return !prev
								})
							}}
							className="inline-flex cursor-pointer items-center gap-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:text-foreground"
						>
							{detailsExpanded ? (
								<ChevronDown className="size-3.5" />
							) : (
								<ChevronRight className="size-3.5" />
							)}
							<span>Technical details</span>
						</button>
					) : (
						<span className="text-[11px] font-medium text-muted-foreground">Diagnostics</span>
					)}

					<button
						type="button"
						onClick={handleCopyClick}
						className={cn(
							"inline-flex cursor-pointer items-center gap-1 rounded-md px-2 py-1 text-[11px] font-medium transition-colors",
							copyStatus === "copied"
								? "text-emerald-500"
								: "text-muted-foreground hover:bg-foreground/6 hover:text-foreground",
						)}
					>
						{copyStatus === "copied" ? (
							<>
								<Check className="size-3" />
								<span>Copied</span>
							</>
						) : (
							<>
								<Copy className="size-3" />
								<span>{copyStatus === "failed" ? "Copy failed" : "Copy diagnostics"}</span>
							</>
						)}
					</button>
				</div>

				{detailsExpanded && hasTechnicalDetails ? (
					<div className="border-t border-border bg-black/5 px-3 py-2.5 dark:bg-black/30">
						<pre className="custom-scrollbar max-h-40 overflow-y-auto font-mono text-[11px] leading-relaxed wrap-break-word whitespace-pre-wrap text-muted-foreground select-text">
							{diagnosticReport}
						</pre>
					</div>
				) : null}
			</div>

			<div className="mt-5 flex flex-wrap items-center justify-end gap-2">
				{onGoHome ? (
					<button
						type="button"
						onClick={onGoHome}
						className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-border bg-foreground/5 px-3 text-xs font-medium text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
					>
						<Home className="size-3.5" />
						<span>Go to Play</span>
					</button>
				) : null}

				{onReloadApp ? (
					<button
						type="button"
						onClick={onReloadApp}
						className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-border bg-foreground/5 px-3 text-xs font-medium text-foreground transition-colors hover:bg-foreground/10"
					>
						<RefreshCw className="size-3.5" />
						<span>Reload App</span>
					</button>
				) : null}

				{onRetry ? (
					<button
						type="button"
						onClick={onRetry}
						className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-xs font-semibold text-primary-foreground shadow-sm transition-opacity hover:opacity-90"
					>
						<RotateCcw className="size-3.5" />
						<span>Try Again</span>
					</button>
				) : null}
			</div>
		</div>
	)
}

export default ErrorFallbackCard
