import {
	ArrowUpRight,
	CircleCheckIcon,
	InfoIcon,
	OctagonXIcon,
	TriangleAlertIcon,
	X,
} from "lucide-react"
import { cn } from "@/shared/lib/utils"
import {
	formatNotificationTimeAgo,
	type NotificationCategory,
	type NotificationItem,
	type NotificationLevel,
} from "@/shared/stores/useNotificationStore"

const CATEGORY_LABELS: Record<NotificationCategory, string> = {
	instance: "Instance",
	server: "Server",
	download: "Download",
	auth: "Account",
	system: "System",
}

const LEVEL_BADGE_CLASS: Record<NotificationLevel, string> = {
	success: "border-emerald-500/30 bg-emerald-500/14 text-emerald-600 dark:text-emerald-300",
	error: "border-rose-500/30 bg-rose-500/14 text-rose-600 dark:text-rose-300",
	warning: "border-amber-500/30 bg-amber-500/14 text-amber-600 dark:text-amber-300",
	info: "border-sky-500/30 bg-sky-500/14 text-sky-600 dark:text-sky-300",
}

const LEVEL_DOT_CLASS: Record<NotificationLevel, string> = {
	success: "bg-emerald-400",
	error: "bg-rose-400",
	warning: "bg-amber-400",
	info: "bg-sky-400",
}

interface NotificationLevelIconProps {
	level: NotificationLevel
}

const NotificationLevelIcon = ({ level }: NotificationLevelIconProps) => {
	if (level === "success") {
		return <CircleCheckIcon className="size-3.5" aria-hidden="true" />
	}
	if (level === "error") {
		return <OctagonXIcon className="size-3.5" aria-hidden="true" />
	}
	if (level === "warning") {
		return <TriangleAlertIcon className="size-3.5" aria-hidden="true" />
	}
	return <InfoIcon className="size-3.5" aria-hidden="true" />
}

interface NotificationFeedItemProps {
	item: NotificationItem
	onMarkRead: (id: string) => void
	onDismiss: (id: string) => void
	onNavigateAction: (id: string, route: string) => void
}

const NotificationFeedItem = ({
	item,
	onMarkRead,
	onDismiss,
	onNavigateAction,
}: NotificationFeedItemProps) => {
	return (
		<div
			className={cn(
				"group/notif relative flex items-start gap-2.5 rounded-xl border px-3 py-2.5 transition-colors",
				item.read
					? "border-border/70 bg-foreground/2 hover:bg-foreground/5"
					: "border-primary/25 bg-primary/6 hover:bg-primary/10",
			)}
		>
			<div
				className={cn(
					"mt-0.5 flex size-7.5 shrink-0 items-center justify-center rounded-lg border",
					LEVEL_BADGE_CLASS[item.level],
				)}
			>
				<NotificationLevelIcon level={item.level} />
			</div>

			<div
				className="min-w-0 flex-1"
				onClick={() => {
					if (!item.read) {
						onMarkRead(item.id)
					}
				}}
			>
				<div className="flex items-center gap-1.5">
					{!item.read ? (
						<span
							aria-label="Unread"
							className={cn("size-1.5 shrink-0 rounded-full", LEVEL_DOT_CLASS[item.level])}
						/>
					) : null}
					<span className="rounded bg-foreground/8 px-1.5 py-0.5 text-[9.5px] font-semibold text-muted-foreground">
						{CATEGORY_LABELS[item.category]}
					</span>
					<span className="text-[10px] text-muted-foreground tabular-nums">
						{formatNotificationTimeAgo(item.createdAt)}
					</span>
				</div>

				<p className="mt-1 text-[12px] leading-snug font-semibold text-popover-foreground">
					{item.title}
				</p>
				{item.message ? (
					<p className="mt-0.5 text-[11px] leading-snug text-muted-foreground">{item.message}</p>
				) : null}

				{item.action ? (
					<button
						type="button"
						onClick={(event) => {
							event.stopPropagation()
							if (item.action) {
								onNavigateAction(item.id, item.action.route)
							}
						}}
						className="mt-2 inline-flex cursor-pointer items-center gap-1 rounded-full border border-border bg-foreground/6 px-2.5 py-1 text-[10.5px] font-semibold text-foreground transition-colors hover:border-primary/40 hover:bg-primary/14 hover:text-primary"
					>
						<span>{item.action.label}</span>
						<ArrowUpRight className="size-3" />
					</button>
				) : null}
			</div>

			<button
				type="button"
				aria-label={`Dismiss ${item.title}`}
				onClick={(event) => {
					event.stopPropagation()
					onDismiss(item.id)
				}}
				className="mt-0.5 flex size-5.5 shrink-0 cursor-pointer items-center justify-center rounded-full text-muted-foreground opacity-0 transition-all group-hover/notif:opacity-100 hover:bg-foreground/10 hover:text-foreground focus-visible:opacity-100"
			>
				<X className="size-3" />
			</button>
		</div>
	)
}

export default NotificationFeedItem
