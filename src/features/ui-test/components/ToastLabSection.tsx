import { Bell, Download, Sparkles, Wand2, XIcon } from "lucide-react"
import { useEffect, useRef, useState } from "react"
import { useShallow } from "zustand/react/shallow"
import ToastPositionPicker, {
	type ToastPosition,
} from "@/features/ui-test/components/ToastPositionPicker"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { appleToast } from "@/shared/ui/sonner"

type ToastDensity = "detailed" | "compact"
type ToastDurationPreset = "2s" | "4s" | "8s" | "pinned"

const DENSITY_OPTIONS: SegmentedOption<ToastDensity>[] = [
	{ value: "detailed", label: "Title + Details" },
	{ value: "compact", label: "Title Only" },
]

const DURATION_OPTIONS: SegmentedOption<ToastDurationPreset>[] = [
	{ value: "2s", label: "2s" },
	{ value: "4s", label: "4s" },
	{ value: "8s", label: "8s" },
	{ value: "pinned", label: "Stay" },
]

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
		const id = appleToast.progress("Downloading Iris Shaders 1.8.2", {
			value: progress,
			detail: `5.1 / ${totalMb.toFixed(1)} MB · 18.4 MB/s`,
			description: showDescription ? "Fetching bundle from Modrinth CDN..." : undefined,
			position: toastPosition,
		})

		const intervalId = window.setInterval(() => {
			progress = Math.min(100, progress + 18)
			const loadedMb = (progress / 100) * totalMb

			if (shouldFail && progress >= 62) {
				window.clearInterval(intervalId)
				appleToast.failProgress(id, "Download interrupted", {
					description: showDescription ? "Connection reset while streaming shard 3/5." : undefined,
					position: toastPosition,
					duration: durationMs,
					action: {
						label: "Retry",
						onClick: () => {
							runAnimatedDownload(false)
						},
					},
				})
				return
			}

			if (progress >= 100) {
				window.clearInterval(intervalId)
				appleToast.completeProgress(id, "Iris Shaders 1.8.2 installed", {
					description: showDescription
						? "Verified SHA-512 checksum and added to profile."
						: undefined,
					position: toastPosition,
					duration: durationMs,
				})
				return
			}

			appleToast.updateProgress(id, {
				value: progress,
				detail: `${loadedMb.toFixed(1)} / ${totalMb.toFixed(1)} MB · ${(16.2 + (progress % 7)).toFixed(1)} MB/s`,
			})
		}, 380)

		timersRef.current.push(intervalId)
	}

	const triggerStepProgress = () => {
		const nextStep = stepToastIdRef.current === null ? 25 : stepValueRef.current + 25

		if (stepToastIdRef.current === null) {
			const id = appleToast.progress("Indexing world backups", {
				value: 25,
				detail: "1 / 4 snapshots verified",
				description: showDescription ? "Click Step Progress again to advance." : undefined,
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
			appleToast.completeProgress(completedId, "All 4 snapshots indexed", {
				description: showDescription ? "Local backup catalog is up to date." : undefined,
				position: toastPosition,
				duration: durationMs,
			})
			return
		}

		const chunk = nextStep / 25
		stepValueRef.current = nextStep
		appleToast.updateProgress(stepToastIdRef.current, {
			value: nextStep,
			detail: `${chunk} / 4 snapshots verified`,
		})
	}

	const triggerPromiseResolve = () => {
		const simulatedJob = new Promise<{ packName: string }>((resolve) => {
			window.setTimeout(() => {
				resolve({ packName: "Horizon Vanilla+" })
			}, 1500)
		})
		appleToast.promise(simulatedJob, {
			loading: "Verifying modpack manifest...",
			success: (result) => {
				return `${result.packName} ready to launch`
			},
			error: "Manifest verification failed",
			description: showDescription ? "Checked 42 mods and shader pipeline." : undefined,
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
			loading: "Syncing asset index...",
			success: "Assets synced",
			error: "Asset checksum failed",
			description: showDescription ? "Remote manifest signature did not match." : undefined,
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
										Custom Card Slot
									</p>
									<span className="rounded-full bg-primary/14 px-1.5 py-0.5 font-mono text-[10.5px] font-semibold text-primary">
										JSX
									</span>
								</div>
								<p className="mt-0.5 text-[11.5px] text-muted-foreground">
									Bespoke layout inside the spring stack.
								</p>
							</div>
							<button
								type="button"
								aria-label="Close custom card"
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
			"Checking Modrinth index...",
			buildOptions("Comparing 18 installed mods for updates."),
		)
		window.setTimeout(() => {
			appleToast.warning(
				"Sodium 0.6.10 available",
				buildOptions("Release notes ready in Updates panel."),
			)
		}, 160)
		window.setTimeout(() => {
			appleToast.error(
				"Texture atlas warning",
				buildOptions("One pack resource exceeded 4K limit."),
			)
		}, 320)
		window.setTimeout(() => {
			appleToast.success("Shader cache warmed", buildOptions("Compiled 14 pipelines in 0.4s."))
		}, 480)
	}

	return (
		<section className="flex min-w-0 flex-col gap-4 rounded-2xl border border-border bg-card/65 p-4">
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2">
					<Bell className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">
						Apple-Style Toast Notifications
					</h2>
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
					Dismiss All
				</button>
			</div>

			<p className="text-[12px] text-muted-foreground">
				Test Liquid Glass styling, determinate ring &amp; bar progress, promise morphs, error
				micro-shake feedback, interactive pills, and Notification Center stack expansion.
			</p>

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
								appleToast.success("Saved as app toast position", {
									position: toastPosition,
								})
							}}
							className="w-fit cursor-pointer rounded-lg border border-primary/30 bg-primary/12 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors hover:bg-primary/20"
						>
							Save as App Default
						</button>
					) : null}
				</div>

				<div className="flex min-w-0 flex-col gap-4 @xl:border-l @xl:border-border/50 @xl:pl-5">
					<div className="flex min-w-0 flex-col gap-1.5">
						<span className="text-[11.5px] font-medium text-foreground/90">Content</span>
						<SegmentedControl
							layoutId="uitest-toast-density"
							options={DENSITY_OPTIONS}
							value={density}
							onChange={setDensity}
							tone="emerald"
						/>
						<span className="text-[11px] text-muted-foreground">
							{showDescription
								? "Shows both headline and secondary detail text."
								: "Compact single-line banner with title only."}
						</span>
					</div>
					<div className="flex min-w-0 flex-col gap-1.5">
						<span className="text-[11.5px] font-medium text-foreground/90">Auto-Close</span>
						<SegmentedControl
							layoutId="uitest-toast-duration"
							options={DURATION_OPTIONS}
							value={durationPreset}
							onChange={setDurationPreset}
							tone="neutral"
						/>
						<span className="text-[11px] text-muted-foreground">
							{durationPreset === "pinned"
								? "Keeps notifications open until you dismiss them."
								: `Closes automatically after ${durationPreset} unless hovered.`}
						</span>
					</div>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					Notification Types
				</span>
				<div className="flex flex-wrap gap-2">
					<button
						type="button"
						onClick={() => {
							appleToast(
								"Ingot Launcher ready",
								buildOptions("All workspace profiles indexed cleanly."),
							)
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Default
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.success(
								"Profile synced cleanly",
								buildOptions("Fabric 1.21.4 shader pack verified across 42 mods."),
							)
						}}
						className="cursor-pointer rounded-xl border border-emerald-500/30 bg-emerald-500/12 px-3 py-1.5 text-[12px] font-medium text-emerald-600 transition-colors hover:bg-emerald-500/20 dark:text-emerald-300"
					>
						Success
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.error(
								"Modrinth checksum mismatch",
								buildOptions("Iris Shaders 1.8.2 failed SHA-512 verification."),
							)
						}}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/12 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/20 dark:text-rose-300"
					>
						Error
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.warning(
								"Java 21 runtime recommended",
								buildOptions("Selected instance currently targets an older JVM build."),
							)
						}}
						className="cursor-pointer rounded-xl border border-amber-500/30 bg-amber-500/12 px-3 py-1.5 text-[12px] font-medium text-amber-600 transition-colors hover:bg-amber-500/20 dark:text-amber-300"
					>
						Warning
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.info(
								"Cloud backup queued",
								buildOptions("World snapshots will upload automatically after exit."),
							)
						}}
						className="cursor-pointer rounded-xl border border-sky-500/30 bg-sky-500/12 px-3 py-1.5 text-[12px] font-medium text-sky-600 transition-colors hover:bg-sky-500/20 dark:text-sky-300"
					>
						Info
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.loading(
								"Downloading NeoForge artifacts...",
								buildOptions("Pinned until manually dismissed or resolved."),
							)
						}}
						className="cursor-pointer rounded-xl border border-primary/35 bg-primary/12 px-3 py-1.5 text-[12px] font-medium text-primary transition-colors hover:bg-primary/20"
					>
						Loading
					</button>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					Progress &amp; Morphs
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
						<span>Download Progress</span>
					</button>
					<button
						type="button"
						onClick={() => {
							runAnimatedDownload(true)
						}}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/18 dark:text-rose-300"
					>
						Progress Fail
					</button>
					<button
						type="button"
						onClick={triggerStepProgress}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Step Progress (+25%)
					</button>
					<button
						type="button"
						onClick={triggerPromiseResolve}
						className="flex cursor-pointer items-center gap-1.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-1.5 text-[12px] font-medium text-emerald-600 transition-colors hover:bg-emerald-500/18 dark:text-emerald-300"
					>
						<Sparkles className="size-3.5" />
						<span>Promise Resolve</span>
					</button>
					<button
						type="button"
						onClick={triggerPromiseReject}
						className="cursor-pointer rounded-xl border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-[12px] font-medium text-rose-600 transition-colors hover:bg-rose-500/18 dark:text-rose-300"
					>
						Promise Reject
					</button>
				</div>
			</div>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
					Variants &amp; Interactions
				</span>
				<div className="flex flex-wrap gap-2">
					<button
						type="button"
						onClick={() => {
							appleToast.success("Screenshot saved to Gallery", {
								...buildOptions("horizon_sunset_2140.png (3840x2160)"),
								action: {
									label: "Reveal",
									onClick: () => {
										appleToast.info("Opened screenshot folder preview", {
											position: toastPosition,
											duration: durationMs,
										})
									},
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Action Pill
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast.warning("Overwrite existing keybinds?", {
								...buildOptions("Applying server preset changes 6 bindings."),
								cancel: {
									label: "Keep",
									onClick: () => {
										appleToast.info("Kept current keybinds", {
											position: toastPosition,
											duration: durationMs,
										})
									},
								},
								action: {
									label: "Apply",
									onClick: () => {
										appleToast.success("Preset keybinds applied", {
											position: toastPosition,
											duration: durationMs,
										})
									},
								},
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Action + Cancel
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast("Shader preset tuned", {
								...buildOptions("Volumetric clouds & reflections enabled."),
								icon: <Wand2 className="size-4 text-primary" />,
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Custom Icon
					</button>
					<button
						type="button"
						onClick={() => {
							appleToast("Minimal HUD notice", {
								...buildOptions("Rendered without a leading medallion icon."),
								icon: null,
							})
						}}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						No Icon
					</button>
					<button
						type="button"
						onClick={triggerCustomCard}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Custom JSX Card
					</button>
					<button
						type="button"
						onClick={triggerStackBurst}
						className="cursor-pointer rounded-xl border border-border bg-foreground/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition-colors hover:bg-foreground/12"
					>
						Stack Burst (4x)
					</button>
				</div>
			</div>
		</section>
	)
}

export default ToastLabSection
