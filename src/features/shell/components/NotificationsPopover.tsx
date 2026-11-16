import { Bell } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect, useMemo, useState } from "react"
import {
	resolveCenteredTitlebarPopoverStyle,
	type PopoverAnchorRect,
} from "@/features/shell/components/AccountSwitcherPopover"
import NotificationFeedItem from "@/features/shell/components/NotificationFeedItem"
import NotificationPopoverHeader, {
	type NotificationFilter,
} from "@/features/shell/components/NotificationPopoverHeader"
import { cn } from "@/shared/lib/utils"
import {
	countUnreadNotifications,
	type NotificationItem,
} from "@/shared/stores/useNotificationStore"

interface NotificationsPopoverProps {
	open: boolean
	anchorRect: PopoverAnchorRect | null
	notifications: NotificationItem[]
	onClose: () => void
	onMarkRead: (id: string) => void
	onMarkAllRead: () => void
	onDismiss: (id: string) => void
	onClearAll: () => void
	onNavigateAction: (id: string, route: string) => void
}

const NotificationsPopover = ({
	open,
	anchorRect,
	notifications,
	onClose,
	onMarkRead,
	onMarkAllRead,
	onDismiss,
	onClearAll,
	onNavigateAction,
}: NotificationsPopoverProps) => {
	const [filter, setFilter] = useState<NotificationFilter>("all")

	const unreadCount = useMemo(() => {
		return countUnreadNotifications(notifications)
	}, [notifications])

	const filteredItems = useMemo(() => {
		if (filter === "unread") {
			return notifications.filter((item) => {
				return !item.read
			})
		}
		return notifications
	}, [filter, notifications])

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
	const emptyHeading =
		filter === "unread" && notifications.length > 0 ? "No unread alerts" : "All caught up"

	return (
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50">
					<button
						type="button"
						aria-label="Close notifications"
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/25"
					/>
					<motion.div
						role="dialog"
						aria-label="Notification Center"
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
						<NotificationPopoverHeader
							totalCount={notifications.length}
							unreadCount={unreadCount}
							filter={filter}
							onFilterChange={setFilter}
							onMarkAllRead={onMarkAllRead}
							onClearAll={onClearAll}
							onClose={onClose}
						/>

						<div className="custom-scrollbar max-h-80 overflow-y-auto p-2">
							{filteredItems.length === 0 ? (
								<div className="flex flex-col items-center justify-center px-4 py-9 text-center">
									<Bell className="mb-2 size-6 text-muted-foreground/55" />
									<p className="text-[12.5px] font-medium text-popover-foreground">
										{emptyHeading}
									</p>
									<p className="mt-0.5 max-w-64 text-[11px] text-muted-foreground">
										Important crash reports, server alerts, account warnings, and completed
										downloads appear here.
									</p>
								</div>
							) : (
								<div className="flex flex-col gap-1.5">
									{filteredItems.map((item) => {
										return (
											<NotificationFeedItem
												key={item.id}
												item={item}
												onMarkRead={onMarkRead}
												onDismiss={onDismiss}
												onNavigateAction={onNavigateAction}
											/>
										)
									})}
								</div>
							)}
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default NotificationsPopover
