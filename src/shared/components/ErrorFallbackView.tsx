import { useRouter, type ErrorComponentProps } from "@tanstack/react-router"
import { AlertTriangle, Check, Copy, Home, RotateCcw } from "lucide-react"
import { useEffect, useRef, useState } from "react"

type CopyStatus = "idle" | "copied" | "failed"

const ErrorFallbackView = ({ error, reset }: ErrorComponentProps) => {
	const router = useRouter()

	const copyResetTimerRef = useRef<number | null>(null)

	const [copyStatus, setCopyStatus] = useState<CopyStatus>("idle")

	useEffect(() => {
		return () => {
			if (copyResetTimerRef.current !== null) {
				window.clearTimeout(copyResetTimerRef.current)
			}
		}
	}, [])

	const errorMessage = error instanceof Error ? error.message : String(error ?? "Unknown error")
	const errorStack = error instanceof Error ? error.stack : undefined

	const handleRetry = () => {
		reset()
		void router.invalidate()
	}

	const scheduleCopyReset = () => {
		if (copyResetTimerRef.current !== null) {
			window.clearTimeout(copyResetTimerRef.current)
		}
		copyResetTimerRef.current = window.setTimeout(() => {
			setCopyStatus("idle")
			copyResetTimerRef.current = null
		}, 1800)
	}

	const copyDetails = async () => {
		const payload = errorStack ? `${errorMessage}\n\n${errorStack}` : errorMessage
		try {
			await navigator.clipboard.writeText(payload)
			setCopyStatus("copied")
		} catch {
			setCopyStatus("failed")
		}
		scheduleCopyReset()
	}

	const handleCopyClick = () => {
		void copyDetails()
	}

	const handleNavigateHome = () => {
		void router.navigate({ to: "/" })
	}

	return (
		<div className="flex h-full w-full flex-col items-center justify-center p-6 select-none">
			<div className="ingot-glass w-full max-w-lg rounded-2xl p-6">
				<div className="flex items-start gap-3.5">
					<div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl border border-rose-500/25 bg-rose-500/15 text-rose-500">
						<AlertTriangle className="h-5 w-5" />
					</div>

					<div className="min-w-0 flex-1">
						<h1 className="text-sm font-semibold text-foreground">
							Something went wrong in this view
						</h1>
						<p className="mt-1 text-xs leading-relaxed text-muted-foreground">
							An unexpected error occurred while rendering or loading data from the launcher
							backend.
						</p>
					</div>
				</div>

				<div className="mt-4 rounded-xl border border-border bg-black/4 p-3 dark:bg-black/30">
					<div className="mb-1.5 flex items-center justify-between gap-2 border-b border-border pb-1.5">
						<span className="font-mono text-[10px] font-semibold tracking-wider text-rose-400 uppercase">
							Diagnostic Output
						</span>
						<button
							type="button"
							onClick={handleCopyClick}
							className="inline-flex cursor-pointer items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-black/6 hover:text-foreground dark:hover:bg-white/6"
						>
							{copyStatus === "copied" ? (
								<>
									<Check className="h-3 w-3 text-emerald-500" />
									<span className="text-emerald-500">Copied</span>
								</>
							) : (
								<>
									<Copy className="h-3 w-3" />
									<span>{copyStatus === "failed" ? "Copy failed" : "Copy"}</span>
								</>
							)}
						</button>
					</div>

					<pre className="custom-scrollbar max-h-36 overflow-y-auto font-mono text-[11px] leading-relaxed wrap-break-word whitespace-pre-wrap text-foreground/90 select-text">
						{errorStack ?? errorMessage}
					</pre>
				</div>

				<div className="mt-5 flex items-center justify-end gap-2">
					<button
						type="button"
						onClick={handleNavigateHome}
						className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-border bg-black/4 px-3 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground dark:bg-white/5"
					>
						<Home className="h-3.5 w-3.5" />
						<span>Go to Play</span>
					</button>

					<button
						type="button"
						onClick={handleRetry}
						className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-xs font-semibold text-primary-foreground shadow-sm transition-opacity hover:opacity-90"
					>
						<RotateCcw className="h-3.5 w-3.5" />
						<span>Try Again</span>
					</button>
				</div>
			</div>
		</div>
	)
}

export default ErrorFallbackView
