import {
	ArrowUpCircle,
	CheckCircle2,
	Download,
	ExternalLink,
	RefreshCw,
	Sparkles,
} from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { cn } from "@/shared/lib/utils"
import { getCurrentAppVersion, useAppUpdateStore } from "@/shared/stores/useAppUpdateStore"
import { useLauncherPrefsStore, type UpdateChannel } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"

type UpdateCheckState = "idle" | "checking" | "upToDate" | "available"

const EASE_OUT_EXPO: readonly [number, number, number, number] = [0.22, 1, 0.36, 1]

const UpdatesVersionCard = () => {
	const { t } = useTranslation()
	const checkTimerRef = useRef<number | null>(null)
	const { updateChannel, setUpdateChannel } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				updateChannel: s.updateChannel,
				setUpdateChannel: s.setUpdateChannel,
			}
		}),
	)
	const { availableUpdate, isInstalling, startInstall } = useAppUpdateStore(
		useShallow((s) => {
			return {
				availableUpdate: s.availableUpdate,
				isInstalling: s.isInstalling,
				startInstall: s.startInstall,
			}
		}),
	)

	const [checkState, setCheckState] = useState<UpdateCheckState>("idle")
	const [lastCheckedLabel, setLastCheckedLabel] = useState<string | null>(null)

	const channelOptions = useMemo<SegmentedOption<UpdateChannel>[]>(() => {
		return [
			{
				value: "stable",
				label: t("settings.general.updates.channels.stable"),
			},
			{
				value: "nightly",
				label: t("settings.general.updates.channels.nightly"),
			},
		]
	}, [t])

	useEffect(() => {
		return () => {
			if (checkTimerRef.current !== null) {
				window.clearTimeout(checkTimerRef.current)
			}
		}
	}, [])

	const currentVersion = getCurrentAppVersion()

	const handleCheckForUpdates = () => {
		if (checkState === "checking") {
			return
		}
		setCheckState("checking")

		if (checkTimerRef.current !== null) {
			window.clearTimeout(checkTimerRef.current)
		}

		checkTimerRef.current = window.setTimeout(() => {
			checkTimerRef.current = null
			setCheckState(availableUpdate ? "available" : "upToDate")
			setLastCheckedLabel(t("settings.general.updates.lastCheckedJustNow"))
		}, 550)
	}

	const lastCheckedText = lastCheckedLabel ?? t("settings.general.updates.lastCheckedNever")
	const isChecking = checkState === "checking"
	const showUpToDate = checkState === "upToDate" && availableUpdate === null

	return (
		<section className="@container flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-start gap-2.5">
				<div className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-foreground/5 text-primary">
					<ArrowUpCircle className="size-4" />
				</div>
				<div className="min-w-0">
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.general.updates.title")}
					</h2>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						{t("settings.general.updates.description")}
					</p>
				</div>
			</div>

			<div className="flex flex-col gap-3 rounded-xl border border-border bg-foreground/3 p-3.5">
				<div className="flex flex-col gap-3 @lg:flex-row @lg:items-center @lg:justify-between">
					<div className="flex min-w-0 items-center gap-3">
						<div className="flex size-10 shrink-0 items-center justify-center rounded-lg border border-primary/25 bg-primary/12 text-primary">
							<Sparkles className="size-5" />
						</div>

						<div className="flex min-w-0 flex-1 flex-col gap-0.5">
							<div className="flex flex-wrap items-center gap-2">
								<p className="text-[13.5px] font-semibold text-foreground">
									{t("settings.general.updates.appName", { version: currentVersion })}
								</p>

								<AnimatePresence mode="wait" initial={false}>
									{checkState === "checking" ? (
										<motion.span
											key="checking-badge"
											initial={{ opacity: 0, scale: 0.92 }}
											animate={{ opacity: 1, scale: 1 }}
											exit={{ opacity: 0, scale: 0.92 }}
											transition={{ duration: 0.16, ease: EASE_OUT_EXPO }}
											className="inline-flex items-center rounded-full border border-primary/25 bg-primary/10 px-2 py-0.5 text-[10.5px] font-medium text-primary"
										>
											{t("settings.general.updates.checking")}
										</motion.span>
									) : null}

									{showUpToDate ? (
										<motion.span
											key="up-to-date-badge"
											initial={{ opacity: 0, scale: 0.92 }}
											animate={{ opacity: 1, scale: 1 }}
											exit={{ opacity: 0, scale: 0.92 }}
											transition={{ duration: 0.16, ease: EASE_OUT_EXPO }}
											className="inline-flex items-center gap-1 rounded-full border border-primary/25 bg-primary/10 px-2 py-0.5 text-[10.5px] font-medium text-primary"
										>
											<CheckCircle2 className="size-3 shrink-0" />
											<span>{t("settings.general.updates.upToDateBadge")}</span>
										</motion.span>
									) : null}
								</AnimatePresence>
							</div>

							<AnimatePresence mode="wait" initial={false}>
								<motion.p
									key={lastCheckedText}
									initial={{ opacity: 0, y: 2 }}
									animate={{ opacity: 1, y: 0 }}
									exit={{ opacity: 0, y: -2 }}
									transition={{ duration: 0.16, ease: EASE_OUT_EXPO }}
									className="text-[11px] text-muted-foreground"
								>
									{lastCheckedText}
								</motion.p>
							</AnimatePresence>
						</div>
					</div>

					<Button
						type="button"
						variant="outline"
						disabled={isChecking}
						onClick={handleCheckForUpdates}
						className={cn(
							"w-full rounded-md text-[12px] @lg:w-auto",
							isChecking ? "cursor-wait" : "cursor-pointer",
						)}
					>
						<RefreshCw className={cn("size-3.5 shrink-0", isChecking ? "animate-spin" : "")} />
						<span>{t("settings.general.updates.checkForUpdates")}</span>
					</Button>
				</div>

				<div className="flex flex-wrap items-center justify-between gap-2.5 border-t border-border/60 pt-3">
					<div className="min-w-0 flex-1">
						<p className="text-[12px] font-medium text-foreground">
							{t("settings.general.updates.channelLabel")}
						</p>
						<p className="text-[11px] leading-relaxed text-muted-foreground">
							{t("settings.general.updates.channelDescription")}
						</p>
					</div>

					<SegmentedControl
						layoutId="updates-channel-pill"
						options={channelOptions}
						value={updateChannel}
						onChange={setUpdateChannel}
						size="sm"
						tone="emerald"
					/>
				</div>

				<AnimatePresence initial={false}>
					{availableUpdate ? (
						<motion.div
							key="available-banner"
							initial={{ opacity: 0, height: 0, marginTop: 0 }}
							animate={{ opacity: 1, height: "auto", marginTop: 12 }}
							exit={{ opacity: 0, height: 0, marginTop: 0 }}
							transition={{ duration: 0.22, ease: EASE_OUT_EXPO }}
							className="overflow-hidden"
						>
							<div className="flex flex-col gap-2.5 rounded-lg border border-primary/40 bg-primary/8 p-3.5">
								<div className="flex flex-wrap items-center justify-between gap-2">
									<div className="flex items-center gap-2 text-primary">
										<Sparkles className="size-4 shrink-0" />
										<span className="text-[13px] font-semibold">
											{t("settings.general.updates.availableTitle", {
												version: availableUpdate.version,
											})}
										</span>
									</div>

									{availableUpdate.changelogUrl ? (
										<a
											href={availableUpdate.changelogUrl}
											target="_blank"
											rel="noreferrer"
											className="inline-flex items-center gap-1 text-[11.5px] font-medium text-primary hover:underline"
										>
											<span>{t("settings.general.updates.changelog")}</span>
											<ExternalLink className="size-3" />
										</a>
									) : null}
								</div>

								<p className="text-[11.5px] leading-relaxed text-foreground/85">
									{availableUpdate.notes}
								</p>

								<div>
									<Button
										type="button"
										disabled={isInstalling}
										onClick={startInstall}
										className="w-full cursor-pointer rounded-md text-[12px] font-semibold @sm:w-auto"
									>
										<Download className="size-3.5" />
										<span>
											{isInstalling
												? t("shell.update.installing")
												: t("settings.general.updates.downloadAndInstall")}
										</span>
									</Button>
								</div>
							</div>
						</motion.div>
					) : null}
				</AnimatePresence>
			</div>
		</section>
	)
}

export default UpdatesVersionCard
