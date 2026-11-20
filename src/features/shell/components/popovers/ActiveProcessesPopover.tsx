import type { TFunction } from "i18next"
import { Activity, Plus, Square, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
	resolveCenteredTitlebarPopoverStyle,
	type PopoverAnchorRect,
} from "@/features/shell/components/popovers/AccountSwitcherPopover"
import { cn } from "@/shared/lib/utils"
import { type RunningProcess } from "@/shared/stores/useProcessStore"

const PORT_PLAYERS_REGEX = /^Port :(\d+) \((\d+)\/(\d+) Players\)$/
const PORT_STATUS_REGEX = /^Port :(\d+) \((\d+)\/(\d+)\)$/

const formatProcessAccountLabel = (account: string, t: TFunction): string => {
	const playersMatch = PORT_PLAYERS_REGEX.exec(account)
	if (playersMatch?.[1] && playersMatch[2] && playersMatch[3]) {
		return t("shell.processes.serverPortPlayers", {
			port: playersMatch[1],
			online: playersMatch[2],
			max: playersMatch[3],
		})
	}
	const statusMatch = PORT_STATUS_REGEX.exec(account)
	if (statusMatch?.[1] && statusMatch[2] && statusMatch[3]) {
		return t("shell.processes.serverPortStatus", {
			port: statusMatch[1],
			online: statusMatch[2],
			max: statusMatch[3],
		})
	}
	return account
}

interface ActiveProcessesPopoverProps {
	open: boolean
	anchorRect: PopoverAnchorRect | null
	processes: RunningProcess[]
	onClose: () => void
	onStopProcess: (id: string) => void
	onStopAll: () => void
	onSpawnTest: () => void
}

const KIND_BADGE_CLASS: Record<RunningProcess["kind"], string> = {
	instance:
		"border-emerald-600/25 bg-emerald-500/12 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/15 dark:text-emerald-300",
	server:
		"border-sky-600/25 bg-sky-500/12 text-sky-700 dark:border-sky-500/30 dark:bg-sky-500/15 dark:text-sky-300",
}

const KIND_LABEL_KEY: Record<
	RunningProcess["kind"],
	"shell.processes.instanceBadge" | "shell.processes.serverBadge"
> = {
	instance: "shell.processes.instanceBadge",
	server: "shell.processes.serverBadge",
}

const ActiveProcessesPopover = ({
	open,
	anchorRect,
	processes,
	onClose,
	onStopProcess,
	onStopAll,
	onSpawnTest,
}: ActiveProcessesPopoverProps) => {
	const { t } = useTranslation()

	useEffect(() => {
		if (!open) {
			return
		}
		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				onClose()
			}
		}
		window.addEventListener("keydown", handleKeyDown)
		return () => {
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [open, onClose])

	const anchoredStyle = resolveCenteredTitlebarPopoverStyle(anchorRect, 384)
	const fallbackClass = anchoredStyle ? "" : "top-11 right-4"

	return (
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50">
					<button
						type="button"
						aria-label={t("shell.processes.closeAria")}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/25"
					/>
					<motion.div
						role="dialog"
						aria-label={t("shell.processes.title")}
						style={anchoredStyle}
						initial={{ opacity: 0, y: -8, scale: 0.97 }}
						animate={{ opacity: 1, y: 0, scale: 1 }}
						exit={{ opacity: 0, y: -8, scale: 0.97 }}
						transition={{ duration: 0.16, ease: "easeOut" }}
						className={cn(
							"ingot-glass-elevated fixed z-50 flex w-96 max-w-[calc(100vw-2rem)] flex-col overflow-hidden rounded-2xl",
							fallbackClass,
						)}
					>
						<div className="flex items-center justify-between gap-3 border-b border-border px-3.5 py-2.5">
							<div className="flex min-w-0 items-center gap-2">
								<Activity className="size-3.5 shrink-0 text-primary" />
								<span className="truncate text-[12.5px] font-semibold text-popover-foreground">
									{t("shell.processes.title")}
								</span>
								<span className="shrink-0 rounded-full bg-foreground/8 px-1.5 py-0.5 text-[10px] font-medium text-foreground/75">
									{processes.length}
								</span>
							</div>
							<button
								type="button"
								onClick={onClose}
								aria-label={t("shell.processes.dismissAria")}
								className="shrink-0 cursor-pointer rounded-md p-1 text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-3.5" />
							</button>
						</div>

						<div className="custom-scrollbar max-h-72 overflow-y-auto p-2">
							{processes.length === 0 ? (
								<div className="flex flex-col items-center justify-center py-8 text-center">
									<Activity className="mb-2 size-6 text-muted-foreground/60" />
									<p className="text-[12.5px] font-medium text-popover-foreground">
										{t("shell.processes.emptyTitle")}
									</p>
									<p className="mt-0.5 text-[11px] text-muted-foreground">
										{t("shell.processes.emptyDescription")}
									</p>
								</div>
							) : (
								<div className="flex flex-col gap-1.5">
									{processes.map((proc) => {
										return (
											<div
												key={proc.id}
												className="flex items-center justify-between gap-2.5 rounded-xl border border-border bg-foreground/4 px-3 py-2 transition-colors hover:bg-foreground/6"
											>
												<div className="min-w-0 flex-1">
													<div className="flex items-center gap-1.5">
														<span className="truncate text-[12.5px] font-medium text-popover-foreground">
															{proc.name}
														</span>
														<span
															className={cn(
																"shrink-0 rounded-full border px-1.5 py-0.5 text-[9.5px] font-medium",
																KIND_BADGE_CLASS[proc.kind],
															)}
														>
															{t(KIND_LABEL_KEY[proc.kind])}
														</span>
													</div>
													<p className="mt-0.5 truncate text-[11px] text-foreground/70">
														{proc.engine} Â· {proc.uptime}
													</p>
													<div className="mt-1.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-[10.5px] font-medium text-foreground/75">
														<span className="whitespace-nowrap">
															{t("shell.processes.ram", { ram: proc.ram })}
														</span>
														<span className="whitespace-nowrap">
															{t("shell.processes.cpu", { cpu: proc.cpu })}
														</span>
														{proc.account ? (
															<span className="truncate">
																{formatProcessAccountLabel(proc.account, t)}
															</span>
														) : null}
													</div>
												</div>
												<button
													type="button"
													onClick={() => {
														onStopProcess(proc.id)
													}}
													className="flex shrink-0 cursor-pointer items-center gap-1 rounded-lg border border-rose-600/25 bg-rose-500/12 px-2 py-1 text-[11px] font-medium whitespace-nowrap text-rose-700 transition-colors hover:bg-rose-500/22 dark:border-rose-500/25 dark:text-rose-300"
												>
													<Square className="size-2.5 fill-current" />
													<span>{t("shell.processes.stop")}</span>
												</button>
											</div>
										)
									})}
								</div>
							)}
						</div>

						<div className="flex items-center justify-between gap-2 border-t border-border bg-foreground/2 px-3 py-2">
							<button
								type="button"
								onClick={onSpawnTest}
								className="flex items-center gap-1 rounded-lg bg-foreground/8 px-2.5 py-1 text-[11.5px] font-medium text-popover-foreground transition-colors hover:bg-foreground/14"
							>
								<Plus className="size-3" />
								<span>{t("shell.processes.spawnTest")}</span>
							</button>
							{processes.length > 0 ? (
								<button
									type="button"
									onClick={onStopAll}
									className="rounded-lg bg-rose-500/15 px-2.5 py-1 text-[11.5px] font-medium text-rose-600 transition-colors hover:bg-rose-500/25 dark:text-rose-300"
								>
									{t("shell.processes.stopAll")}
								</button>
							) : null}
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default ActiveProcessesPopover
