import { Boxes, User } from "lucide-react"
import { useMemo } from "react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { getAccountInitials } from "@/features/accounts/accountModel"
import { useActiveAccount } from "@/features/accounts/useActiveAccount"
import NotificationBellButton from "@/features/shell/components/notifications/NotificationBellButton"
import UpdatePillButton from "@/features/shell/components/UpdatePillButton"
import { findNavItem } from "@/features/shell/navigation"
import {
	isAnchoredOverlayOpen,
	useShellOverlayStore,
} from "@/features/shell/stores/useShellOverlayStore"
import { cn } from "@/shared/lib/utils"
import { useAppUpdateStore } from "@/shared/stores/useAppUpdateStore"
import {
	countUnreadNotifications,
	hasUnreadByLevel,
	useNotificationStore,
} from "@/shared/stores/useNotificationStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

interface MobileTopBarProps {
	currentPath: string
	layoutKey: string
}

const MobileTopBar = ({ currentPath, layoutKey }: MobileTopBarProps) => {
	const { t } = useTranslation()
	const { activeAccount } = useActiveAccount()
	const activeProcessesCount = useProcessStore((s) => {
		return s.runningProcesses.length
	})
	const notifications = useNotificationStore((s) => {
		return s.notifications
	})
	const { activeOverlay, toggleAnchored } = useShellOverlayStore(
		useShallow((s) => {
			return {
				activeOverlay: s.activeOverlay,
				toggleAnchored: s.toggleAnchored,
			}
		}),
	)

	const { availableUpdate, dismissed } = useAppUpdateStore(
		useShallow((s) => {
			return {
				availableUpdate: s.availableUpdate,
				dismissed: s.dismissed,
			}
		}),
	)

	const notificationsOpen = isAnchoredOverlayOpen(activeOverlay, "notifications", layoutKey)
	const updateOpen = isAnchoredOverlayOpen(activeOverlay, "update", layoutKey)
	const showUpdatePill = availableUpdate !== null && !dismissed
	const unreadCount = useMemo(() => {
		return countUnreadNotifications(notifications)
	}, [notifications])
	const hasUnreadError = useMemo(() => {
		return hasUnreadByLevel(notifications, "error")
	}, [notifications])
	const hasUnreadWarning = useMemo(() => {
		return hasUnreadByLevel(notifications, "warning")
	}, [notifications])

	const currentItem = findNavItem(currentPath)
	const accountAriaLabel = activeAccount
		? t("shell.switchAccountAria", { username: activeAccount.username })
		: t("accounts.empty.signIn")

	return (
		<header className="relative z-30 flex shrink-0 items-center justify-between border-b border-border bg-sidebar/85 px-3.5 pt-[calc(0.5rem+env(safe-area-inset-top))] pb-2.5 backdrop-blur-xl">
			<div className="flex min-w-0 items-center gap-2.5">
				<div className="flex size-7 shrink-0 items-center justify-center rounded-xl bg-primary/18 text-primary ring-1 ring-primary/30">
					<Boxes className="size-3.5" />
				</div>
				<div className="min-w-0">
					<p className="truncate text-[13.5px] font-semibold text-foreground">
						{t(currentItem.labelKey)}
					</p>
					<p className="truncate text-[10px] text-muted-foreground">
						{t("shell.launcherSubtitle")}
					</p>
				</div>
			</div>

			<div className="flex shrink-0 items-center gap-2">
				<UpdatePillButton
					visible={showUpdatePill}
					version={availableUpdate?.version ?? ""}
					open={updateOpen}
					size="md"
					onClick={(event) => {
						toggleAnchored("update", layoutKey, event.currentTarget)
					}}
				/>

				<button
					type="button"
					onClick={(event) => {
						toggleAnchored("processes", layoutKey, event.currentTarget)
					}}
					aria-label={t("shell.toggleActiveProcesses")}
					className="flex h-8 cursor-pointer items-center gap-1.5 rounded-full border border-border bg-foreground/5 px-2.5 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/10"
				>
					<span
						className={cn(
							"size-1.5 rounded-full",
							activeProcessesCount > 0 ? "animate-pulse bg-primary" : "bg-muted-foreground",
						)}
					/>
					<span className="tabular-nums">{activeProcessesCount}</span>
				</button>

				<NotificationBellButton
					open={notificationsOpen}
					unreadCount={unreadCount}
					hasUnreadError={hasUnreadError}
					hasUnreadWarning={hasUnreadWarning}
					size="md"
					onClick={(event) => {
						toggleAnchored("notifications", layoutKey, event.currentTarget)
					}}
				/>

				<button
					type="button"
					onClick={(event) => {
						toggleAnchored("account", layoutKey, event.currentTarget)
					}}
					aria-label={accountAriaLabel}
					className={cn(
						"flex size-8 cursor-pointer items-center justify-center rounded-full shadow-inner ring-1 ring-border",
						activeAccount
							? cn("bg-linear-to-br text-[11px] font-bold text-white", activeAccount.avatarBg)
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{activeAccount ? getAccountInitials(activeAccount.username) : <User className="size-4" />}
				</button>
			</div>
		</header>
	)
}

export default MobileTopBar
