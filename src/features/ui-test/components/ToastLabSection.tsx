import { Bell, Download, Sparkles, Wand2, XIcon } from "lucide-react"
import { useEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import ToastPositionPicker, {
	type ToastPosition,
} from "@/features/ui-test/components/ToastPositionPicker"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { notifyBackgroundEvent } from "@/shared/lib/notify"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { appleToast } from "@/shared/ui/sonner"

type ToastDensity = "detailed" | "compact"
type ToastDurationPreset = "2s" | "4s" | "8s" | "pinned"

const resolveDurationMs = (preset: ToastDurationPreset): number => {
	if (preset === "2s") {
		return 2000
	}
	if (preset === "4s") {
		return 4000
	}
	if (preset === "8s") {
		return 8000
	}
	return Number.POSITIVE_INFINITY
}

const ToastLabSection = () => {
	const { t } = useTranslation()
	const { preferredToastPosition, setPreferredToastPosition } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				preferredToastPosition: s.toastPosition,
				setPreferredToastPosition: s.setToastPosition,
			}
		}),
	)

	const timersRef = useRef<number[]>([])
	const stepToastIdRef = useRef<string | number | null>(null)
	const stepValueRef = useRef<number>(0)

	const [toastPosition, setToastPosition] = useState<ToastPosition>(preferredToastPosition)
	const [density, setDensity] = useState<ToastDensity>("detailed")
	const [durationPreset, setDurationPreset] = useState<ToastDurationPreset>("4s")

	const densityOptions: SegmentedOption<ToastDensity>[] = useMemo(() => {
		return [
			{ value: "detailed", label: t("uiLab.toast.density.detailed") },
			{ value: "compact", label: t("uiLab.toast.density.compact") },
		]
	}, [t])

	const durationOptions: SegmentedOption<ToastDurationPreset>[] = useMemo(() => {
		return [
			{ value: "2s", label: "2s" },
			{ value: "4s", label: "4s" },
			{ value: "8s", label: "8s" },
			{ value: "pinned", label: t("uiLab.toast.durationStay") },
		]
	}, [t])

	useEffect(() => {
		return () => {
			for (const timerId of timersRef.current) {
				window.clearInterval(timerId)
				window.clearTimeout(timerId)
			}
			timersRef.current = []
		}
	}, [])

	const showDescription = density === "detailed"
	const durationMs = resolveDurationMs(durationPreset)

	const buildOptions = (descriptionText: string) => {
		return {
			position: toastPosition,
			duration: durationMs,
			description: showDescription ? descriptionText : undefined,
		}
	}

	const runAnimatedDownload = (shouldFail: boolean) => {
		let progress = 8
		const totalMb = 64.0
		const id = appleToast.progress(t("uiLab.toast.samples.downloadingIris"), {
			value: progress,
			detail: t("uiLab.toast.samples.downloadingIrisDetail", {
				loaded: "5.1",
				total: totalMb.toFixed(1),
				speed: "18.4",
			}),
			description: showDescription
				? t("uiLab.toast.samples.downloadingIrisDescription")
				: undefined,
			position: toastPosition,
		})

		const intervalId = window.setInterval(() => {
			progress = Math.min(100, progress + 18)
			const loadedMb = (progress / 100) * totalMb

			if (shouldFail && progress >= 62) {
				window.clearInterval(intervalId)
				appleToast.failProgress(id, t("uiLab.toast.samples.downloadInterrupted"), {
					description: showDescription
						? t("uiLab.toast.samples.downloadInterruptedDescription")
						: undefined,
					position: toastPosition,
					duration: durationMs,
					action: {
						label: t("uiLab.toast.samples.retry"),
						onClick: () => {
							runAnimatedDownload(false)
						},
					},
				})
				return
			}

			if (progress >= 100) {
				window.clearInterval(intervalId)
				appleToast.completeProgress(id, t("uiLab.toast.samples.irisInstalled"), {
					description: showDescription
						? t("uiLab.toast.samples.irisInstalledDescription")
						: undefined,
					position: toastPosition,
					duration: durationMs,
				})
				return
			}

			appleToast.updateProgress(id, {
				value: progress,
				detail: t("uiLab.toast.samples.downloadingIrisDetail", {
					loaded: loadedMb.toFixed(1),
					total: totalMb.toFixed(1),
					speed: (16.2 + (progress % 7)).toFixed(1),
				}),
			})
		}, 380)

		timersRef.current.push(intervalId)
	}

	const triggerStepProgress = () => {
		const nextStep = stepToastIdRef.current === null ? 25 : stepValueRef.current + 25

		if (stepToastIdRef.current === null) {
			const id = appleToast.progress(t("uiLab.toast.samples.indexingBackups"), {
				value: 25,
				detail: t("uiLab.toast.samples.snapshotsVerified", { count: 1 }),
				description: showDescription ? t("uiLab.toast.samples.stepProgressDescription") : undefined,
				position: toastPosition,
			})
			stepToastIdRef.current = id
			stepValueRef.current = 25
			return
		}

		if (nextStep >= 100) {
			const completedId = stepToastIdRef.current
			stepToastIdRef.current = null
			stepValueRef.current = 0
			appleToast.completeProgress(completedId, t("uiLab.toast.samples.allSnapshotsIndexed"), {
				description: showDescription
					? t("uiLab.toast.samples.allSnapshotsIndexedDescription")
					: undefined,
				position: toastPosition,
				duration: durationMs,
			})
			return
		}

		const chunk = nextStep / 25
		stepValueRef.current = nextStep
		appleToast.updateProgress(stepToastIdRef.current, {
			value: nextStep,
			detail: t("uiLab.toast.samples.snapshotsVerified", { count: chunk }),
		})
	}

	const triggerPromiseResolve = () => {
		const simulatedJob = new Promise<{ packName: string }>((resolve) => {
			window.setTimeout(() => {
				resolve({ packName: "Horizon Vanilla+" })
			}, 1500)
		})
		appleToast.promise(simulatedJob, {
			loading: t("uiLab.toast.samples.verifyingManifest"),
			success: (result) => {
				return t("uiLab.toast.samples.packReadyToLaunch", { packName: result.packName })
			},
			error: t("uiLab.toast.samples.manifestFailed"),
			description: showDescription
				? t("uiLab.toast.samples.verifyingManifestDescription")
				: undefined,
			position: toastPosition,
			duration: durationMs,
		})
	}

	const triggerPromiseReject = () => {
		const failingJob = new Promise<string>((_resolve, reject) => {
			window.setTimeout(() => {
				reject(new Error("Checksum mismatch"))
			}, 1400)
		})
		appleToast.promise(failingJob, {
			loading: t("uiLab.toast.samples.syncingAssetIndex"),
			success: t("uiLab.toast.samples.assetsSynced"),
			error: t("uiLab.toast.samples.assetChecksumFailed"),
			description: showDescription
				? t("uiLab.toast.samples.assetChecksumFailedDescription")
				: undefined,
			position: toastPosition,
			duration: durationMs,
		})
	}

	const triggerCustomCard = () => {
		appleToast.custom(
			(toastId) => {
				return (
					<div className="relative overflow-hidden rounded-[22px] border border-white/55 bg-(--ingot-surface-elevated) px-3.5 py-3 text-popover-foreground shadow-[0_18px_44px_-12px_rgba(15,23,42,0.26),inset_0_1px_0_rgba(255,255,255,0.72)] ring-1 ring-black/6 backdrop-blur-2xl backdrop-saturate-180 dark:border-white/14 dark:shadow-[0_24px_52px_-12px_rgba(0,0,0,0.68),inset_0_1px_0_rgba(255,255,255,0.16)] dark:ring-black/40">
						<div className="flex items-center gap-3">
							<div className="flex size-9 shrink-0 items-center justify-center rounded-[13px] border border-primary/35 bg-linear-to-b from-primary/24 to-primary/10 text-primary">
								<Download className="size-4" />
							</div>
							<div className="min-w-0 flex-1">
								<div className="flex items-center justify-between gap-2">
									<p className="truncate text-[12.5px] font-semibold tracking-tight">
										{t("uiLab.toast.samples.customCardTitle")}
									</p>
									<span className="rounded-full bg-primary/14 px-1.5 py-0.5 font-mono text-[10.5px] font-semibold text-primary">
										JSX
									</span>
								</div>
								<p className="mt-0.5 text-[11.5px] text-muted-foreground">
									{t("uiLab.toast.samples.customCardDescription")}
								</p>
							</div>
							<button
								type="button"
								aria-label={t("uiLab.toast.samples.closeCustomCardAria")}
								onClick={() => {
									appleToast.dismiss(toastId)
								}}
								className="flex size-5.5 shrink-0 cursor-pointer items-center justify-center rounded-full border border-border/50 bg-foreground/8 text-muted-foreground transition-colors hover:bg-foreground/15 hover:text-foreground"
							>
								<XIcon className="size-3" />
							</button>
						</div>
					</div>
				)
			},
			{
				position: toastPosition,
				duration: durationMs,
			},
		)
	}

	const triggerStackBurst = () => {
		appleToast.info(
			t("uiLab.toast.samples.stackInfoTitle"),
			buildOptions(t("uiLab.toast.samples.stackInfoDescription")),
		)
		window.setTimeout(() => {
			appleToast.warning(
				t("uiLab.toast.samples.stackWarningTitle"),
				buildOptions(t("uiLab.toast.samples.stackWarningDescription")),
			)
		}, 160)
		window.setTimeout(() => {
			appleToast.error(
				t("uiLab.toast.samples.stackErrorTitle"),
				buildOptions(t("uiLab.toast.samples.stackErrorDescription")),
			)
		}, 320)
		window.setTimeout(() => {
			appleToast.success(
				t("uiLab.toast.samples.stackSuccessTitle"),
				buildOptions(t("uiLab.toast.samples.stackSuccessDescription")),
			)
		}, 480)
	}

	return (
		<section className="flex min-w-0 flex-col gap-4 rounded-2xl border border-border bg-card/65 p-4">
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2">
					<Bell className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">{t("uiLab.toast.title")}</h2>
				</div>
				<button
					type="button"
					onClick={() => {
						stepToastIdRef.current = null
						stepValueRef.current = 0
						appleToast.dismiss()
					}}
					className="cursor-pointer rounded-xl border border-border px-2.5 py-1 text-[11.5px] font-medium text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
				>
					{t("uiLab.toast.dismissAll")}
				</button>
			</div>

			<p className="text-[12px] text-muted-foreground">{t("uiLab.toast.subtitle")}</p>

			<div className="grid grid-cols-1 gap-5 rounded-xl border border-border/60 bg-foreground/2 p-4 @xl:grid-cols-[auto_1fr] @xl:items-center">
				<div className="flex flex-col gap-2">
					<ToastPositionPicker
						value={toastPosition}
						showDetails={showDescription}
						layoutId="uitest-toast-position-card"
						onChange={setToastPosition}
					/>
					{toastPosition !== preferredToastPosition ? (
						<button
							type="button"
							onClick={() => {
								setPreferredToastPosition(toastPosition)
								appleToast.success(t("uiLab.toast.savedAsAppDefaultToast"), {
									position: toastPosition,
								})
							}}
							className="w-fit cursor-pointer rounded-lg border border-primary/30 bg-primary/12 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors hover:bg-primary/20"
						>
							{t("uiLab.toast.saveAsAppDefault")}
						</button>
					) : null}
				</div>

				<div className="flex min-w-0 flex-col gap-4 @xl:border-l @xl:border-border/50 @xl:pl-5">
					<div className="flex min-w-0 flex-col gap-1.5">
						<span className="text-[11.5px] font-medium text-foreground/90">
							{t("uiLab.toast.contentLabel")}
						</span>
						<SegmentedControl
							layoutId="uitest-toast-density"
							options={densityOptions}
							value={density}
							onChange={setDensity}
							tone="emerald"
						/>
						<span className="text-[11px] text-muted-foreground">
							{showDescription
								? t("uiLab.toast.densityHintDetailed")
								: t("uiLab.toast.densityHintCompact")}
						</span>
					</div>
					<div className="flex min-w-0 flex-col gap-1.5">
						<span className="text-[11.5px] font-medium text-foreground/90">
							{t("uiLab.toast.autoCloseLabel")}
						</span>
						<SegmentedControl
							layoutId="uitest-toast-duration"
							options={durationOptions}
							value={durationPreset}
							onChange={setDurationPreset}
							tone="neutral"
						/>
						<span className="text-[11px] text-muted-foreground">
							{durationPreset === "pinned"
								? t("uiLab.toast.durationPinnedHint")
								: t("uiLab.toast.durationTimedHint", { duration: durationPreset })}
						</span>
					</div>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					{t("uiLab.toast.notificationTypesLabel")}
				</span>
				<div className="flex flex-wrap gap-2">
					<button
						type="button"
						onClick={() => {
							appleToast(
								t("uiLab.toast.samples.defaultTitle"),
								buildOptions(t("uiLab.toast.samples.defaultDescription")),
							)
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.types.default")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.success(
								t("uiLab.toast.samples.successTitle"),
								buildOptions(t("uiLab.toast.samples.successDescription")),
							)
						}}
						className="cursor-pointer rounded-xl border border-emerald-500/30 bg-emerald-500/12 px-3 py-1.5 text-[12px] font-medium text-emerald-600 transition-colors hover:bg-emerald-500/20 dark:text-emerald-300"
					>
						{t("uiLab.toast.types.success")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.error(
								t("uiLab.toast.samples.errorTitle"),
								buildOptions(t("uiLab.toast.samples.errorDescription")),
							)
						}}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/12 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/20 dark:text-rose-300"
					>
						{t("uiLab.toast.types.error")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.warning(
								t("uiLab.toast.samples.warningTitle"),
								buildOptions(t("uiLab.toast.samples.warningDescription")),
							)
						}}
						className="cursor-pointer rounded-xl border border-amber-500/30 bg-amber-500/12 px-3 py-1.5 text-[12px] font-medium text-amber-600 transition-colors hover:bg-amber-500/20 dark:text-amber-300"
					>
						{t("uiLab.toast.types.warning")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.info(
								t("uiLab.toast.samples.infoTitle"),
								buildOptions(t("uiLab.toast.samples.infoDescription")),
							)
						}}
						className="cursor-pointer rounded-xl border border-sky-500/30 bg-sky-500/12 px-3 py-1.5 text-[12px] font-medium text-sky-600 transition-colors hover:bg-sky-500/20 dark:text-sky-300"
					>
						{t("uiLab.toast.types.info")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.loading(
								t("uiLab.toast.samples.loadingTitle"),
								buildOptions(t("uiLab.toast.samples.loadingDescription")),
							)
						}}
						className="cursor-pointer rounded-xl border border-primary/35 bg-primary/12 px-3 py-1.5 text-[12px] font-medium text-primary transition-colors hover:bg-primary/20"
					>
						{t("uiLab.toast.types.loading")}
					</button>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					{t("uiLab.toast.progressMorphsLabel")}
				</span>
				<div className="flex flex-wrap gap-2">
					<button
						type="button"
						onClick={() => {
							runAnimatedDownload(false)
						}}
						className="flex cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3 py-1.5 text-[12px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
					>
						<Download className="size-3.5" />
						<span>{t("uiLab.toast.actions.downloadProgress")}</span>
					</button>
					<button
						type="button"
						onClick={() => {
							runAnimatedDownload(true)
						}}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/18 dark:text-rose-300"
					>
						{t("uiLab.toast.actions.progressFail")}
					</button>
					<button
						type="button"
						onClick={triggerStepProgress}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.actions.stepProgress")}
					</button>
					<button
						type="button"
						onClick={triggerPromiseResolve}
						className="flex cursor-pointer items-center gap-1.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-[12px] font-medium text-emerald-600 transition-colors hover:bg-emerald-500/18 dark:text-emerald-300"
					>
						<Sparkles className="size-3.5" />
						<span>{t("uiLab.toast.actions.promiseResolve")}</span>
					</button>
					<button
						type="button"
						onClick={triggerPromiseReject}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/18 dark:text-rose-300"
					>
						{t("uiLab.toast.actions.promiseReject")}
					</button>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					{t("uiLab.toast.variantsLabel")}
				</span>
				<div className="flex flex-wrap gap-2">
					<button
						type="button"
						onClick={() => {
							appleToast.success(t("uiLab.toast.samples.screenshotSavedTitle"), {
								...buildOptions("horizon_sunset_2140.png (3840x2160)"),
								action: {
									label: t("uiLab.toast.samples.reveal"),
									onClick: () => {
										appleToast.info(t("uiLab.toast.samples.openedPreview"), {
											position: toastPosition,
											duration: durationMs,
										})
									},
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.variants.actionPill")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.warning(t("uiLab.toast.samples.overwriteKeybindsTitle"), {
								...buildOptions(t("uiLab.toast.samples.overwriteKeybindsDescription")),
								cancel: {
									label: t("uiLab.toast.samples.keep"),
									onClick: () => {
										appleToast.info(t("uiLab.toast.samples.keptKeybinds"), {
											position: toastPosition,
											duration: durationMs,
										})
									},
								},
								action: {
									label: t("uiLab.toast.samples.apply"),
									onClick: () => {
										appleToast.success(t("uiLab.toast.samples.appliedKeybinds"), {
											position: toastPosition,
											duration: durationMs,
										})
									},
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.variants.actionCancel")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast(t("uiLab.toast.samples.shaderPresetTitle"), {
								...buildOptions(t("uiLab.toast.samples.shaderPresetDescription")),
								icon: <Wand2 className="size-4 text-primary" />,
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.variants.customIcon")}
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast(t("uiLab.toast.samples.minimalHudTitle"), {
								...buildOptions(t("uiLab.toast.samples.minimalHudDescription")),
								icon: null,
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.variants.noIcon")}
					</button>
					<button
						type="button"
						onClick={triggerCustomCard}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.variants.customJsxCard")}
					</button>
					<button
						type="button"
						onClick={triggerStackBurst}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						{t("uiLab.toast.variants.stackBurst")}
					</button>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					{t("uiLab.toast.bellAlertsLabel")}
				</span>
				<div className="flex flex-wrap gap-2">
					<button
						type="button"
						onClick={() => {
							notifyBackgroundEvent({
								level: "error",
								category: "instance",
								title: t("uiLab.toast.samples.bellCrashTitle"),
								message: t("uiLab.toast.samples.bellCrashMessage"),
								action: {
									label: t("uiLab.toast.samples.bellCrashAction"),
									route: "/instances/inst-1/logs",
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/12 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/20 dark:text-rose-300"
					>
						{t("uiLab.toast.bellAlerts.instanceCrash")}
					</button>
					<button
						type="button"
						onClick={() => {
							notifyBackgroundEvent({
								level: "warning",
								category: "server",
								title: t("uiLab.toast.samples.bellServerTitle"),
								message: t("uiLab.toast.samples.bellServerMessage"),
								action: {
									label: t("uiLab.toast.samples.bellServerAction"),
									route: "/servers",
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-amber-500/30 bg-amber-500/12 px-3 py-1.5 text-[12px] font-medium text-amber-600 transition-colors hover:bg-amber-500/20 dark:text-amber-300"
					>
						{t("uiLab.toast.bellAlerts.serverPortAlert")}
					</button>
					<button
						type="button"
						onClick={() => {
							notifyBackgroundEvent({
								level: "success",
								category: "download",
								title: t("uiLab.toast.samples.bellPackTitle"),
								message: t("uiLab.toast.samples.bellPackMessage"),
								action: {
									label: t("uiLab.toast.samples.bellPackAction"),
									route: "/modpacks",
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-emerald-500/30 bg-emerald-500/12 px-3 py-1.5 text-[12px] font-medium text-emerald-600 transition-colors hover:bg-emerald-500/20 dark:text-emerald-300"
					>
						{t("uiLab.toast.bellAlerts.packInstalled")}
					</button>
					<button
						type="button"
						onClick={() => {
							notifyBackgroundEvent({
								level: "warning",
								category: "auth",
								title: t("uiLab.toast.samples.bellAuthTitle"),
								message: t("uiLab.toast.samples.bellAuthMessage"),
								action: {
									label: t("uiLab.toast.samples.bellAuthAction"),
									route: "/settings/accounts",
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-sky-500/30 bg-sky-500/12 px-3 py-1.5 text-[12px] font-medium text-sky-600 transition-colors hover:bg-sky-500/20 dark:text-sky-300"
					>
						{t("uiLab.toast.bellAlerts.sessionWarning")}
					</button>
				</div>
			</div>
		</section>
	)
}

export default ToastLabSection
