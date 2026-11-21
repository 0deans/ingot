import { ChevronDown, Sparkles } from "lucide-react"
import { AnimatePresence, LayoutGroup, motion } from "motion/react"
import { useMemo } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import CapsuleGroupedNav from "@/features/shell/components/desktop/CapsuleGroupedNav"
import NotificationBellButton from "@/features/shell/components/notifications/NotificationBellButton"
import {
	isAnchoredOverlayOpen,
	useShellOverlayStore,
} from "@/features/shell/stores/useShellOverlayStore"
import WindowControlButtons from "@/shared/components/window/WindowControlButtons"
import { cn } from "@/shared/lib/utils"
import {
	ACCOUNT_TYPE_LABEL_KEYS,
	getAccountInitials,
	selectActiveAccount,
	useAccountStore,
} from "@/shared/stores/useAccountStore"
import { useAppUpdateStore } from "@/shared/stores/useAppUpdateStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	countUnreadNotifications,
	hasUnreadByLevel,
	useNotificationStore,
} from "@/shared/stores/useNotificationStore"
import { calculateTotalProcessRamMb, useProcessStore } from "@/shared/stores/useProcessStore"

interface WindowsTitlebarProps {
	currentPath: string
	layoutKey: string
}

const WindowsTitlebar = ({ currentPath, layoutKey }: WindowsTitlebarProps) => {
	const { t } = useTranslation()
	const { navVariant, showDeveloperPages } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				navVariant: s.navVariant,
				showDeveloperPages: s.showDeveloperPages,
			}
		}),
	)
	const activeAccount = useAccountStore(selectActiveAccount)
	const runningProcesses = useProcessStore((s) => {
		return s.runningProcesses
	})
	const notifications = useNotificationStore((s) => {
		return s.notifications
	})
	const activeOverlayResult = useShellOverlayStore(
		useShallow((s) => {
			return {
				activeOverlay: s.activeOverlay,
				toggleAnchored: s.toggleAnchored,
			}
		}),
	)
	const { activeOverlay, toggleAnchored } = activeOverlayResult

	const { availableUpdate, dismissed } = useAppUpdateStore(
		useShallow((s) => {
			return {
				availableUpdate: s.availableUpdate,
				dismissed: s.dismissed,
			}
		}),
	)

	const processesOpen = isAnchoredOverlayOpen(activeOverlay, "processes", layoutKey)
	const notificationsOpen = isAnchoredOverlayOpen(activeOverlay, "notifications", layoutKey)
	const accountOpen = isAnchoredOverlayOpen(activeOverlay, "account", layoutKey)
	const updateOpen = isAnchoredOverlayOpen(activeOverlay, "update", layoutKey)
	const showUpdatePill = availableUpdate !== null && !dismissed
	const activeProcessesCount = runningProcesses.length
	const totalProcessRamMb = useMemo(() => {
		return calculateTotalProcessRamMb(runningProcesses)
	}, [runningProcesses])
	const unreadNotificationsCount = useMemo(() => {
		return countUnreadNotifications(notifications)
	}, [notifications])
	const hasUnreadNotificationError = useMemo(() => {
		return hasUnreadByLevel(notifications, "error")
	}, [notifications])
	const hasUnreadNotificationWarning = useMemo(() => {
		return hasUnreadByLevel(notifications, "warning")
	}, [notifications])

	const isCapsule = navVariant === "capsule"
	const hasRunning = activeProcessesCount > 0
	const totalRamLabel =
		totalProcessRamMb >= 1024
			? `${(totalProcessRamMb / 1024).toFixed(1)} GB`
			: `${totalProcessRamMb} MB`
	const statusLabel = hasRunning
		? t("shell.statusActive", { count: activeProcessesCount })
		: t("shell.statusIdle")
	const getVersionPillToneClass = (): string => {
		if (!showUpdatePill) {
			return "pointer-events-none border-border bg-foreground/5 text-muted-foreground"
		}
		if (updateOpen) {
			return "cursor-pointer border-primary/50 bg-primary/22 text-primary"
		}
		return "cursor-pointer border-primary/35 bg-primary/12 text-primary hover:bg-primary/20"
	}
	const versionPillToneClass = getVersionPillToneClass()

	return (
		<header
			data-tauri-drag-region
			className="relative z-40 flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border bg-sidebar/85 pr-0 pl-3 backdrop-blur-xl select-none"
		>
			<div data-tauri-drag-region className="flex shrink-0 items-center gap-2">
				<img
					src="/ingot.svg"
					alt={t("shell.logoAlt")}
					draggable={false}
					className="pointer-events-none size-5 shrink-0 object-contain"
				/>
				<span className="pointer-events-none text-[12.5px] font-semibold tracking-tight text-foreground">
					Ingot
				</span>
				<button
					type="button"
					disabled={!showUpdatePill}
					aria-expanded={showUpdatePill ? updateOpen : undefined}
					aria-haspopup={showUpdatePill ? "dialog" : undefined}
					aria-label={
						showUpdatePill && availableUpdate
							? t("shell.update.pillAria", { version: availableUpdate.version })
							: undefined
					}
					onClick={(event) => {
						if (!showUpdatePill) {
							return
						}
						toggleAnchored("update", layoutKey, event.currentTarget)
					}}
					className={cn(
						"inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-medium whitespace-nowrap transition-colors",
						versionPillToneClass,
						isCapsule ? "hidden md:inline-flex" : "inline-flex",
					)}
				>
					<AnimatePresence mode="wait" initial={false}>
						{showUpdatePill && availableUpdate ? (
							<motion.span
								key={`update-${availableUpdate.version}`}
								initial={{ opacity: 0, y: 3 }}
								animate={{ opacity: 1, y: 0 }}
								exit={{ opacity: 0, y: -3 }}
								transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
								className="inline-flex items-center gap-1 font-semibold text-primary"
							>
								<Sparkles className="size-2.5 shrink-0" />
								<span>v{availableUpdate.version}</span>
							</motion.span>
						) : (
							<motion.span
								key="current-version"
								initial={{ opacity: 0, y: 3 }}
								animate={{ opacity: 1, y: 0 }}
								exit={{ opacity: 0, y: -3 }}
								transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
							>
								v{APP_VERSION}
							</motion.span>
						)}
					</AnimatePresence>
				</button>
			</div>

			<div
				data-tauri-drag-region
				className="@container flex min-w-0 flex-1 items-center justify-center overflow-hidden px-1"
			>
				<AnimatePresence initial={false}>
					{isCapsule ? (
						<LayoutGroup id="windows-titlebar-capsule">
							<motion.div
								key="top-capsule-nav"
								initial={{ opacity: 0, scale: 0.96, y: -3 }}
								animate={{ opacity: 1, scale: 1, y: 0 }}
								exit={{ opacity: 0, scale: 0.96, y: -3 }}
								transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
								className="flex max-w-full min-w-0 items-center"
							>
								<CapsuleGroupedNav
									currentPath={currentPath}
									showDeveloperPages={showDeveloperPages}
								/>
							</motion.div>
						</LayoutGroup>
					) : null}
				</AnimatePresence>
			</div>

			<div data-tauri-drag-region className="flex shrink-0 items-center gap-1.5">
				<AnimatePresence initial={false}>
					{isCapsule ? (
						<motion.button
							key="capsule-account-trigger"
							type="button"
							aria-expanded={accountOpen}
							aria-haspopup="dialog"
							title={`${activeAccount.username} (${t(ACCOUNT_TYPE_LABEL_KEYS[activeAccount.type])})`}
							onClick={(event) => {
								toggleAnchored("account", layoutKey, event.currentTarget)
							}}
							initial={{ opacity: 0, scale: 0.94 }}
							animate={{ opacity: 1, scale: 1 }}
							exit={{ opacity: 0, scale: 0.94 }}
							transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
							className="flex h-7 cursor-pointer items-center gap-1 rounded-full border border-border bg-foreground/5 p-1 text-[11.5px] font-medium text-foreground transition-colors hover:bg-foreground/10 xl:gap-1.5 xl:pr-2 xl:pl-1"
						>
							<span
								className={cn(
									"flex size-5 shrink-0 items-center justify-center rounded-full bg-linear-to-br text-[9px] leading-none font-bold tracking-tight text-white",
									activeAccount.avatarBg,
								)}
							>
								{getAccountInitials(activeAccount.username)}
							</span>
							<span className="hidden max-w-24 truncate xl:inline">{activeAccount.username}</span>
							<ChevronDown className="size-3 text-muted-foreground" />
						</motion.button>
					) : null}
				</AnimatePresence>

				<button
					type="button"
					aria-expanded={processesOpen}
					aria-haspopup="dialog"
					onClick={(event) => {
						toggleAnchored("processes", layoutKey, event.currentTarget)
					}}
					className={cn(
						"flex h-7 cursor-pointer items-center gap-1.5 rounded-full border px-2.5 text-[11px] font-medium whitespace-nowrap transition-colors",
						processesOpen
							? "border-primary/40 bg-primary/18 text-primary"
							: "border-border bg-foreground/5 text-foreground hover:bg-foreground/10",
					)}
				>
					<span
						className={cn(
							"size-1.5 rounded-full",
							hasRunning ? "animate-pulse bg-primary" : "bg-muted-foreground",
						)}
					/>
					<span className="tabular-nums">
						{statusLabel}
						{hasRunning ? <span className="hidden md:inline"> · {totalRamLabel}</span> : null}
					</span>
				</button>

				<NotificationBellButton
					open={notificationsOpen}
					unreadCount={unreadNotificationsCount}
					hasUnreadError={hasUnreadNotificationError}
					hasUnreadWarning={hasUnreadNotificationWarning}
					onClick={(event) => {
						toggleAnchored("notifications", layoutKey, event.currentTarget)
					}}
				/>

				<WindowControlButtons />
			</div>
		</header>
	)
}

export default WindowsTitlebar
