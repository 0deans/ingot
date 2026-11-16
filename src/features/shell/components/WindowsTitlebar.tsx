import { ChevronDown } from "lucide-react"
import { AnimatePresence, LayoutGroup, motion } from "motion/react"
import { useMemo } from "react"
import { useShallow } from "zustand/react/shallow"
import CapsuleNavItem from "@/features/shell/components/CapsuleNavItem"
import NotificationBellButton from "@/features/shell/components/NotificationBellButton"
import { getVisibleNavItems, isNavItemActive, type NavItem } from "@/features/shell/navigation"
import {
	isAnchoredOverlayOpen,
	useShellOverlayStore,
} from "@/features/shell/stores/useShellOverlayStore"
import WindowControlButtons from "@/shared/components/WindowControlButtons"
import { cn } from "@/shared/lib/utils"
import {
	getAccountInitials,
	selectActiveAccount,
	useAccountStore,
} from "@/shared/stores/useAccountStore"
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

	const processesOpen = isAnchoredOverlayOpen(activeOverlay, "processes", layoutKey)
	const notificationsOpen = isAnchoredOverlayOpen(activeOverlay, "notifications", layoutKey)
	const accountOpen = isAnchoredOverlayOpen(activeOverlay, "account", layoutKey)
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
	const capsuleItems: NavItem[] = getVisibleNavItems(showDeveloperPages)
	const hasRunning = activeProcessesCount > 0
	const totalRamLabel =
		totalProcessRamMb >= 1024
			? `${(totalProcessRamMb / 1024).toFixed(1)} GB`
			: `${totalProcessRamMb} MB`
	const statusLabel = hasRunning ? `${activeProcessesCount} Active` : "Idle"
	const versionVisibilityClass = isCapsule ? "hidden md:inline-block" : "inline-block"

	return (
		<header
			data-tauri-drag-region
			className="relative z-40 flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border bg-sidebar/85 pr-0 pl-3 backdrop-blur-xl select-none"
		>
			<div data-tauri-drag-region className="flex shrink-0 items-center gap-2">
				<img
					src="/ingot.svg"
					alt="Ingot logo"
					draggable={false}
					className="pointer-events-none size-5 shrink-0 object-contain"
				/>
				<span className="pointer-events-none text-[12.5px] font-semibold tracking-tight text-foreground">
					Ingot
				</span>
				<span
					className={cn(
						"pointer-events-none rounded-full border border-border bg-foreground/5 px-1.5 py-0.5 text-[9.5px] font-medium text-muted-foreground",
						versionVisibilityClass,
					)}
				>
					v{APP_VERSION}
				</span>
			</div>

			<div
				data-tauri-drag-region
				className="@container flex min-w-0 flex-1 items-center justify-center overflow-hidden px-1"
			>
				<AnimatePresence initial={false}>
					{isCapsule ? (
						<LayoutGroup id="windows-titlebar-capsule">
							<motion.nav
								key="top-capsule-nav"
								aria-label="Primary Navigation"
								initial={{ opacity: 0, scale: 0.96, y: -3 }}
								animate={{ opacity: 1, scale: 1, y: 0 }}
								exit={{ opacity: 0, scale: 0.96, y: -3 }}
								transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
								className="no-scrollbar flex max-w-full min-w-0 items-center gap-0.5 overflow-x-auto rounded-full border border-border bg-foreground/5 p-0.5 backdrop-blur-md"
							>
								{capsuleItems.map((item) => {
									return (
										<CapsuleNavItem
											key={item.path}
											item={item}
											isActive={isNavItemActive(currentPath, item.path)}
										/>
									)
								})}
							</motion.nav>
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
							title={`${activeAccount.username} (${activeAccount.type})`}
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
