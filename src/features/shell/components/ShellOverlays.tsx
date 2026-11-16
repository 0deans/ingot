import { useCallback, useEffect, useMemo } from "react"
import { useShallow } from "zustand/react/shallow"
import AccountSwitcherPopover, {
	type AccountPopoverAnchor,
} from "@/features/shell/components/AccountSwitcherPopover"
import ActiveProcessesPopover from "@/features/shell/components/ActiveProcessesPopover"
import MobileMoreSheet from "@/features/shell/components/MobileMoreSheet"
import NotificationsPopover from "@/features/shell/components/NotificationsPopover"
import { PRIMARY_MOBILE_PATHS, getVisibleNavItems } from "@/features/shell/navigation"
import {
	isAnchoredOverlayOpen,
	useShellOverlayStore,
} from "@/features/shell/stores/useShellOverlayStore"
import { navigateFromNotification } from "@/shared/lib/notify"
import { useAccountStore } from "@/shared/stores/useAccountStore"
import { useLauncherPrefsStore, type NavVariant } from "@/shared/stores/useLauncherPrefsStore"
import { useNotificationStore } from "@/shared/stores/useNotificationStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

const resolveAccountAnchorMode = (
	isMobile: boolean,
	navVariant: NavVariant,
	railExpanded: boolean,
): AccountPopoverAnchor => {
	if (isMobile) {
		return "mobile"
	}
	if (navVariant === "capsule") {
		return "capsule"
	}
	if (navVariant === "rail" && !railExpanded) {
		return "rail"
	}
	return "sidebar"
}

interface ShellOverlaysProps {
	currentPath: string
	layoutKey: string
	isMobile: boolean
}

const ShellOverlays = ({ currentPath, layoutKey, isMobile }: ShellOverlaysProps) => {
	const { navVariant, railExpanded, showDeveloperPages } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				navVariant: s.navVariant,
				railExpanded: s.railExpanded,
				showDeveloperPages: s.showDeveloperPages,
			}
		}),
	)

	const { activeOverlay, anchorsByKind, closeAll } = useShellOverlayStore(
		useShallow((s) => {
			return {
				activeOverlay: s.activeOverlay,
				anchorsByKind: s.anchorsByKind,
				closeAll: s.closeAll,
			}
		}),
	)

	const { accounts, activeAccountId, setActiveAccountId, addAccount } = useAccountStore(
		useShallow((s) => {
			return {
				accounts: s.accounts,
				activeAccountId: s.activeAccountId,
				setActiveAccountId: s.setActiveAccountId,
				addAccount: s.addAccount,
			}
		}),
	)

	const { runningProcesses, stopProcess, stopAllProcesses, spawnMockProcess } = useProcessStore(
		useShallow((s) => {
			return {
				runningProcesses: s.runningProcesses,
				stopProcess: s.stopProcess,
				stopAllProcesses: s.stopAllProcesses,
				spawnMockProcess: s.spawnMockProcess,
			}
		}),
	)

	const { notifications, markRead, markAllRead, dismissNotification, clearAll } =
		useNotificationStore(
			useShallow((s) => {
				return {
					notifications: s.notifications,
					markRead: s.markRead,
					markAllRead: s.markAllRead,
					dismissNotification: s.dismissNotification,
					clearAll: s.clearAll,
				}
			}),
		)

	const processesOpen = isAnchoredOverlayOpen(activeOverlay, "processes", layoutKey)
	const notificationsOpen = isAnchoredOverlayOpen(activeOverlay, "notifications", layoutKey)
	const accountOpen = isAnchoredOverlayOpen(activeOverlay, "account", layoutKey)
	const moreOpen = activeOverlay.kind === "mobileMore"

	const secondaryMobileItems = useMemo(() => {
		return getVisibleNavItems(showDeveloperPages).filter((item) => {
			return !PRIMARY_MOBILE_PATHS.includes(item.path)
		})
	}, [showDeveloperPages])

	const accountAnchorMode = resolveAccountAnchorMode(isMobile, navVariant, railExpanded)

	const handleSpawnTestProcess = useCallback(() => {
		spawnMockProcess()
	}, [spawnMockProcess])

	const handleNavigateNotification = useCallback(
		(notificationId: string, route: string) => {
			markRead(notificationId)
			closeAll()
			navigateFromNotification(route)
		},
		[closeAll, markRead],
	)

	useEffect(() => {
		if (!processesOpen && !notificationsOpen && !accountOpen) {
			return
		}
		window.addEventListener("resize", closeAll)
		return () => {
			window.removeEventListener("resize", closeAll)
		}
	}, [accountOpen, closeAll, notificationsOpen, processesOpen])

	return (
		<>
			<ActiveProcessesPopover
				open={processesOpen}
				anchorRect={anchorsByKind.processes ?? null}
				processes={runningProcesses}
				onClose={closeAll}
				onStopProcess={stopProcess}
				onStopAll={stopAllProcesses}
				onSpawnTest={handleSpawnTestProcess}
			/>

			<NotificationsPopover
				open={notificationsOpen}
				anchorRect={anchorsByKind.notifications ?? null}
				notifications={notifications}
				onClose={closeAll}
				onMarkRead={markRead}
				onMarkAllRead={markAllRead}
				onDismiss={dismissNotification}
				onClearAll={clearAll}
				onNavigateAction={handleNavigateNotification}
			/>

			<AccountSwitcherPopover
				open={accountOpen}
				anchorMode={accountAnchorMode}
				anchorRect={anchorsByKind.account ?? null}
				accounts={accounts}
				activeAccountId={activeAccountId}
				onClose={closeAll}
				onSelectAccount={setActiveAccountId}
				onAddAccount={addAccount}
			/>

			<MobileMoreSheet
				open={moreOpen}
				currentPath={currentPath}
				items={secondaryMobileItems}
				onClose={closeAll}
			/>
		</>
	)
}

export default ShellOverlays
