import { Check, Copy, ExternalLink, Loader2, RotateCcw, ShieldCheck, Trash2 } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useCallback, useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { type AccountProfile, type MicrosoftDeviceCode } from "@/bindings"
import {
	useCancelMicrosoftLogin,
	useDiscardPendingMicrosoftLogin,
	useFinishMicrosoftLogin,
	usePendingMicrosoftLogin,
	useResumeMicrosoftLogin,
	useStartMicrosoftLogin,
} from "@/features/settings/api/accountsApi"
import { getApiErrorMessage, isApiError } from "@/shared/api/apiError"
import AnimatedCollapse from "@/shared/components/motion/AnimatedCollapse"
import AnimatedHeight from "@/shared/components/motion/AnimatedHeight"
import { cn } from "@/shared/lib/utils"
import { Button, buttonVariants } from "@/shared/ui/button"

const COPY_FEEDBACK_DURATION_MS = 2000

interface MicrosoftAuthPanelProps {
	onSuccess: (profile: AccountProfile) => void
}

const MicrosoftAuthPanel = ({ onSuccess }: MicrosoftAuthPanelProps) => {
	const { t } = useTranslation()
	const pendingQuery = usePendingMicrosoftLogin()
	const startMutation = useStartMicrosoftLogin()
	const finishMutation = useFinishMicrosoftLogin()
	const cancelMutation = useCancelMicrosoftLogin()
	const resumeMutation = useResumeMicrosoftLogin()
	const discardMutation = useDiscardPendingMicrosoftLogin()

	const isMountedRef = useRef(true)
	const isPollingRef = useRef(false)
	const cancelMutateRef = useRef(cancelMutation.mutate)
	const copyResetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

	const [deviceCode, setDeviceCode] = useState<MicrosoftDeviceCode | null>(null)
	const [errorMessage, setErrorMessage] = useState<string | null>(null)
	const [copied, setCopied] = useState(false)

	const handleStart = useCallback(() => {
		setErrorMessage(null)
		setCopied(false)

		startMutation.mutate(undefined, {
			onSuccess: (code) => {
				if (!isMountedRef.current) {
					cancelMutateRef.current()
					return
				}

				setDeviceCode(code)
				isPollingRef.current = true

				finishMutation.mutate(code, {
					onSuccess: (profile) => {
						isPollingRef.current = false
						if (!isMountedRef.current) {
							return
						}
						setDeviceCode(null)
						onSuccess(profile)
					},
					onError: (error) => {
						isPollingRef.current = false
						if (!isMountedRef.current) {
							return
						}
						setDeviceCode(null)
						if (isApiError(error) && error.kind === "cancelled") {
							return
						}
						setErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
					},
				})
			},
			onError: (error) => {
				if (!isMountedRef.current) {
					return
				}
				setErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
			},
		})
	}, [finishMutation, onSuccess, startMutation, t])

	const handleCancel = useCallback(() => {
		isPollingRef.current = false
		setDeviceCode(null)
		setErrorMessage(null)
		setCopied(false)

		cancelMutation.mutate(undefined, {
			onError: (error) => {
				if (!isMountedRef.current) {
					return
				}
				setErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
			},
		})
	}, [cancelMutation, t])

	const handleCopyCode = useCallback(async () => {
		if (!deviceCode) {
			return
		}
		try {
			await navigator.clipboard.writeText(deviceCode.userCode)
		} catch {
			return
		}
		if (!isMountedRef.current) {
			return
		}
		setCopied(true)
		if (copyResetTimerRef.current !== null) {
			clearTimeout(copyResetTimerRef.current)
		}
		copyResetTimerRef.current = setTimeout(() => {
			copyResetTimerRef.current = null
			if (isMountedRef.current) {
				setCopied(false)
			}
		}, COPY_FEEDBACK_DURATION_MS)
	}, [deviceCode])

	const handleResume = useCallback(
		(ticket: string) => {
			setErrorMessage(null)
			resumeMutation.mutate(ticket, {
				onSuccess: (profile) => {
					if (!isMountedRef.current) {
						return
					}
					onSuccess(profile)
				},
				onError: (error) => {
					if (!isMountedRef.current) {
						return
					}
					setErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
				},
			})
		},
		[onSuccess, resumeMutation, t],
	)

	const handleDiscard = useCallback(
		(ticket: string) => {
			setErrorMessage(null)
			discardMutation.mutate(ticket, {
				onError: (error) => {
					if (!isMountedRef.current) {
						return
					}
					setErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
				},
			})
		},
		[discardMutation, t],
	)

	useEffect(() => {
		cancelMutateRef.current = cancelMutation.mutate
	}, [cancelMutation.mutate])

	useEffect(() => {
		isMountedRef.current = true
		return () => {
			isMountedRef.current = false
			if (copyResetTimerRef.current !== null) {
				clearTimeout(copyResetTimerRef.current)
				copyResetTimerRef.current = null
			}
			if (isPollingRef.current) {
				isPollingRef.current = false
				cancelMutateRef.current()
			}
		}
	}, [])

	const pendingLogin = pendingQuery.data ?? null
	const isCancellingPrevious =
		deviceCode === null && (cancelMutation.isPending || finishMutation.isPending)
	const isBusy =
		startMutation.isPending ||
		finishMutation.isPending ||
		cancelMutation.isPending ||
		resumeMutation.isPending ||
		discardMutation.isPending
	const codeCharacters = deviceCode !== null ? deviceCode.userCode.split("") : []
	const verificationHost =
		deviceCode !== null ? deviceCode.verificationUri.replace(/^https?:\/\//, "") : ""

	return (
		<div className="flex flex-col gap-5">
			<div className="flex items-start gap-3">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-border/70 bg-background/60 text-primary">
					<ShieldCheck className="size-4" />
				</div>
				<div className="flex min-w-0 flex-col gap-0.5">
					<h2 className="text-sm font-semibold text-foreground">
						{t("accounts.auth.microsoft.title")}
					</h2>
					<p className="text-xs leading-relaxed text-muted-foreground">
						{t("accounts.auth.microsoft.description")}
					</p>
				</div>
			</div>

			<AnimatedHeight className="-m-px p-px">
				<div className="flex flex-col gap-5">
					<AnimatedCollapse show={pendingLogin !== null && deviceCode === null}>
						<div className="flex flex-col gap-3 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4">
							<div className="flex flex-col gap-0.5">
								<p className="text-xs font-semibold text-foreground">
									{t("accounts.auth.microsoft.pendingTitle")}
								</p>
								<p className="text-xs leading-relaxed text-muted-foreground">
									{t("accounts.auth.microsoft.pendingDescription")}
								</p>
							</div>
							<div className="flex flex-wrap items-center gap-2">
								<Button
									type="button"
									size="sm"
									disabled={isBusy}
									onClick={() => {
										if (pendingLogin !== null) {
											handleResume(pendingLogin.ticket)
										}
									}}
									className="cursor-pointer rounded-xl"
								>
									{resumeMutation.isPending ? (
										<Loader2 className="size-3.5 animate-spin" />
									) : (
										<RotateCcw className="size-3.5" />
									)}
									<span>{t("accounts.auth.microsoft.resume")}</span>
								</Button>
								<Button
									type="button"
									variant="outline"
									size="sm"
									disabled={isBusy}
									onClick={() => {
										if (pendingLogin !== null) {
											handleDiscard(pendingLogin.ticket)
										}
									}}
									className="cursor-pointer rounded-xl"
								>
									{discardMutation.isPending ? (
										<Loader2 className="size-3.5 animate-spin" />
									) : (
										<Trash2 className="size-3.5" />
									)}
									<span>{t("accounts.auth.microsoft.discard")}</span>
								</Button>
							</div>
						</div>
					</AnimatedCollapse>

					<AnimatePresence mode="wait" initial={false}>
						{deviceCode !== null ? (
							<motion.div
								key="device-code"
								initial={{ opacity: 0, y: 6, scale: 0.994 }}
								animate={{ opacity: 1, y: 0, scale: 1 }}
								exit={{ opacity: 0, y: -6, scale: 0.994 }}
								transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
								className="flex flex-col gap-4"
							>
								<div className="flex flex-col gap-2.5">
									<div className="flex items-center justify-between gap-2">
										<span className="shrink-0 text-xs font-medium tracking-wide text-muted-foreground uppercase">
											{t("accounts.auth.microsoft.codeLabel")}
										</span>
										<span className="truncate font-mono text-xs text-muted-foreground">
											{verificationHost}
										</span>
									</div>
									<button
										type="button"
										aria-label={deviceCode.userCode}
										onClick={() => {
											void handleCopyCode()
										}}
										className="flex w-full cursor-pointer items-center justify-center gap-1.5 rounded-2xl border border-border/70 bg-background/50 p-3 transition-colors hover:border-border sm:gap-2 sm:p-4"
									>
										{codeCharacters.map((char, index) => {
											if (char === "-") {
												return (
													<span
														key={`${index}-${char}`}
														aria-hidden="true"
														className="shrink-0 px-0.5 font-mono text-base font-bold text-muted-foreground sm:text-lg"
													>
														-
													</span>
												)
											}
											return (
												<span
													key={`${index}-${char}`}
													className="flex h-10 min-w-0 flex-1 items-center justify-center rounded-xl border border-border/70 bg-card/80 font-mono text-base font-bold text-foreground sm:h-12 sm:text-xl"
												>
													{char}
												</span>
											)
										})}
									</button>
								</div>

								<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
									<Button
										type="button"
										variant="outline"
										size="lg"
										onClick={() => {
											void handleCopyCode()
										}}
										className="w-full cursor-pointer rounded-xl"
									>
										{copied ? (
											<Check className="size-4 text-emerald-500" />
										) : (
											<Copy className="size-4" />
										)}
										<span>
											{copied
												? t("accounts.auth.microsoft.copied")
												: t("accounts.auth.microsoft.copyCode")}
										</span>
									</Button>

									<a
										href={deviceCode.verificationUri}
										target="_blank"
										rel="noreferrer"
										className={cn(
											buttonVariants({ variant: "default", size: "lg" }),
											"w-full rounded-xl",
										)}
									>
										<ExternalLink className="size-4" />
										<span>{t("accounts.auth.microsoft.openBrowser")}</span>
									</a>
								</div>

								<div className="flex items-center justify-between gap-2 border-t border-border/50 pt-3.5">
									<div className="flex min-w-0 items-center gap-2 text-xs text-muted-foreground">
										<Loader2 className="size-3.5 shrink-0 animate-spin text-primary" />
										<span className="truncate">{t("accounts.auth.microsoft.waiting")}</span>
									</div>
									<Button
										type="button"
										variant="ghost"
										size="sm"
										disabled={cancelMutation.isPending}
										onClick={handleCancel}
										className="shrink-0 cursor-pointer rounded-xl text-muted-foreground hover:text-foreground"
									>
										{t("accounts.auth.microsoft.cancel")}
									</Button>
								</div>
							</motion.div>
						) : (
							<motion.div
								key="start-button"
								initial={{ opacity: 0, y: 6, scale: 0.994 }}
								animate={{ opacity: 1, y: 0, scale: 1 }}
								exit={{ opacity: 0, y: -6, scale: 0.994 }}
								transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
							>
								<Button
									type="button"
									size="lg"
									disabled={isBusy}
									onClick={handleStart}
									className="w-full cursor-pointer rounded-xl px-4 text-xs font-semibold sm:w-auto"
								>
									{startMutation.isPending || isCancellingPrevious ? (
										<Loader2 className="size-4 animate-spin" />
									) : (
										<ShieldCheck className="size-4" />
									)}
									<span>{t("accounts.auth.microsoft.startButton")}</span>
								</Button>
								<AnimatedCollapse show={isCancellingPrevious}>
									<p className="pt-2 text-xs text-muted-foreground">
										{t("accounts.auth.microsoft.cancelling")}
									</p>
								</AnimatedCollapse>
							</motion.div>
						)}
					</AnimatePresence>

					<AnimatedCollapse show={errorMessage !== null}>
						<p
							role="alert"
							className="rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive"
						>
							{errorMessage}
						</p>
					</AnimatedCollapse>
				</div>
			</AnimatedHeight>
		</div>
	)
}

export default MicrosoftAuthPanel
