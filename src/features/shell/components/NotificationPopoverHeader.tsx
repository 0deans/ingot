import { Bell, CheckCheck, Trash2, X } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

export type NotificationFilter = "all" | "unread"

interface NotificationPopoverHeaderProps {
	totalCount: number
	unreadCount: number
	filter: NotificationFilter
	onFilterChange: (next: NotificationFilter) => void
	onMarkAllRead: () => void
	onClearAll: () => void
	onClose: () => void
}

const NotificationPopoverHeader = ({
	totalCount,
	unreadCount,
	filter,
	onFilterChange,
	onMarkAllRead,
	onClearAll,
	onClose,
}: NotificationPopoverHeaderProps) => {
	const { t } = useTranslation()
	const hasItems = totalCount > 0
	const hasUnread = unreadCount > 0

	return (
		<>
			<div className="flex items-center justify-between border-b border-border px-3.5 py-2.5">
				<div className="flex items-center gap-2">
					<Bell className="size-3.5 text-primary" />
					<span className="text-[12.5px] font-semibold text-popover-foreground">
						{t("shell.notifications.title")}
					</span>
					{hasUnread ? (
						<span className="rounded-full bg-primary/18 px-1.5 py-0.5 text-[10px] font-semibold text-primary tabular-nums">
							{t("shell.notifications.newCount", { count: unreadCount })}
						</span>
					) : null}
				</div>

				<div className="flex items-center gap-1">
					{hasUnread ? (
						<button
							type="button"
							onClick={onMarkAllRead}
							className="inline-flex cursor-pointer items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] font-medium text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
						>
							<CheckCheck className="size-3" />
							<span>{t("shell.notifications.readAll")}</span>
						</button>
					) : null}
					{hasItems ? (
						<button
							type="button"
							onClick={onClearAll}
							aria-label={t("shell.notifications.clearAllAriaLabel")}
							className="inline-flex cursor-pointer items-center gap-1 rounded-md px-1.5 py-1 text-[10.5px] font-medium text-muted-foreground transition-colors hover:bg-rose-500/12 hover:text-rose-600 dark:hover:text-rose-400"
						>
							<Trash2 className="size-3" />
							<span>{t("common.clear")}</span>
						</button>
					) : null}
					<button
						type="button"
						onClick={onClose}
						aria-label={t("shell.notifications.dismissAria")}
						className="cursor-pointer rounded-md p-1 text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
					>
						<X className="size-3.5" />
					</button>
				</div>
			</div>

			{hasItems ? (
				<div className="flex items-center gap-1 border-b border-border/60 bg-foreground/2 px-3 py-1.5">
					<button
						type="button"
						onClick={() => {
							onFilterChange("all")
						}}
						className={cn(
							"cursor-pointer rounded-full px-2.5 py-0.5 text-[11px] font-medium transition-colors",
							filter === "all"
								? "bg-foreground/12 text-foreground"
								: "text-muted-foreground hover:text-foreground",
						)}
					>
						{t("shell.notifications.allFilter", { count: totalCount })}
					</button>
					<button
						type="button"
						onClick={() => {
							onFilterChange("unread")
						}}
						className={cn(
							"cursor-pointer rounded-full px-2.5 py-0.5 text-[11px] font-medium transition-colors",
							filter === "unread"
								? "bg-foreground/12 text-foreground"
								: "text-muted-foreground hover:text-foreground",
						)}
					>
						{t("shell.notifications.unreadFilter", { count: unreadCount })}
					</button>
				</div>
			) : null}
		</>
	)
}

export default NotificationPopoverHeader
