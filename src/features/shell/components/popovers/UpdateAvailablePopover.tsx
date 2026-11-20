import { Download, ExternalLink, Sparkles, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
	resolveCenteredTitlebarPopoverStyle,
	type PopoverAnchorRect,
} from "@/features/shell/components/popovers/AccountSwitcherPopover"
import { cn } from "@/shared/lib/utils"
import { getCurrentAppVersion, type AppUpdateInfo } from "@/shared/stores/useAppUpdateStore"
import { Button } from "@/shared/ui/button"

interface UpdateAvailablePopoverProps {
	open: boolean
	anchorRect: PopoverAnchorRect | null
	update: AppUpdateInfo | null
	isInstalling: boolean
	onClose: () => void
	onInstall: () => void
	onDismiss: () => void
}

const EASE_OUT_EXPO: readonly [number, number, number, number] = [0.22, 1, 0.36, 1]

const UpdateAvailablePopover = ({
	open,
	anchorRect,
	update,
	isInstalling,
	onClose,
	onInstall,
	onDismiss,
}: UpdateAvailablePopoverProps) => {
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

	const anchoredStyle = resolveCenteredTitlebarPopoverStyle(anchorRect, 340)
	const fallbackClass = anchoredStyle ? "" : "top-11 right-4"
	const currentVersion = getCurrentAppVersion()

	return (
		<AnimatePresence>
			{open && update ? (
				<div className="fixed inset-0 z-50">
					<button
						type="button"
						aria-label={t("shell.update.closePopoverAria")}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-transparent"
					/>

					<motion.div
						role="dialog"
						aria-label={t("shell.update.popoverTitle", { version: update.version })}
						initial={{ opacity: 0, y: -6, scale: 0.97 }}
						animate={{ opacity: 1, y: 0, scale: 1 }}
						exit={{ opacity: 0, y: -6, scale: 0.97 }}
						transition={{ duration: 0.16, ease: EASE_OUT_EXPO }}
						style={anchoredStyle}
						className={cn(
							"ingot-glass-elevated fixed z-50 flex w-[calc(100vw-1rem)] max-w-85 flex-col gap-3 rounded-2xl p-3.5 shadow-2xl",
							fallbackClass,
						)}
					>
						<div className="flex items-start justify-between gap-2.5">
							<div className="flex min-w-0 items-center gap-2.5">
								<div className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-primary/25 bg-primary/14 text-primary">
									<Sparkles className="size-4" />
								</div>
								<div className="min-w-0">
									<p className="truncate text-[13px] font-semibold text-foreground">
										{t("shell.update.popoverTitle", { version: update.version })}
									</p>
									<p className="text-[11px] text-muted-foreground">
										{t("shell.update.currentVersion", { version: currentVersion })}
									</p>
								</div>
							</div>

							<button
								type="button"
								onClick={onClose}
								aria-label={t("shell.update.closePopoverAria")}
								className="flex size-6 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
							>
								<X className="size-3.5" />
							</button>
						</div>

						<p className="rounded-xl border border-border bg-foreground/3 p-2.5 text-[11.5px] leading-relaxed text-foreground/85">
							{update.notes}
						</p>

						<div className="flex flex-wrap items-center justify-between gap-2">
							{update.changelogUrl ? (
								<a
									href={update.changelogUrl}
									target="_blank"
									rel="noreferrer"
									className="inline-flex items-center gap-1 text-[11.5px] font-medium text-primary hover:underline"
								>
									<span>{t("settings.general.updates.changelog")}</span>
									<ExternalLink className="size-3" />
								</a>
							) : (
								<span />
							)}

							<div className="flex items-center gap-1.5">
								<Button
									type="button"
									variant="ghost"
									size="sm"
									onClick={() => {
										onDismiss()
										onClose()
									}}
									className="cursor-pointer rounded-md text-[11.5px]"
								>
									{t("shell.update.later")}
								</Button>
								<Button
									type="button"
									size="sm"
									disabled={isInstalling}
									onClick={onInstall}
									className="cursor-pointer rounded-md px-3 text-[11.5px] font-semibold"
								>
									<Download className="size-3.5" />
									<span>
										{isInstalling ? t("shell.update.installing") : t("shell.update.updateNow")}
									</span>
								</Button>
							</div>
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default UpdateAvailablePopover
