import { Bell } from "lucide-react"
import { type MouseEvent } from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

interface NotificationBellButtonProps {
	open: boolean
	unreadCount: number
	hasUnreadError: boolean
	hasUnreadWarning: boolean
	size?: "sm" | "md"
	onClick: (event: MouseEvent<HTMLButtonElement>) => void
}

const resolveBellBadgeClass = (hasUnreadError: boolean, hasUnreadWarning: boolean): string => {
	if (hasUnreadError) {
		return "bg-rose-500 text-white shadow-[0_0_8px_rgba(244,63,94,0.55)]"
	}
	if (hasUnreadWarning) {
		return "bg-amber-500 text-slate-950"
	}
	return "bg-primary text-primary-foreground"
}

const NotificationBellButton = ({
	open,
	unreadCount,
	hasUnreadError,
	hasUnreadWarning,
	size = "sm",
	onClick,
}: NotificationBellButtonProps) => {
	const { t } = useTranslation()
	const hasUnread = unreadCount > 0
	const badgeText = unreadCount > 9 ? "9+" : String(unreadCount)
	const badgeClass = resolveBellBadgeClass(hasUnreadError, hasUnreadWarning)
	const sizeClass = size === "md" ? "size-8" : "size-7"
	const ariaLabel = hasUnread
		? t("shell.notifications.unreadAria", { count: unreadCount })
		: t("shell.notifications.title")

	const buttonToneClass = open
		? "border-primary/40 bg-primary/18 text-primary"
		: hasUnreadError
			? "border-rose-500/35 bg-rose-500/10 text-rose-500 hover:bg-rose-500/16 dark:text-rose-300"
			: "border-border bg-foreground/5 text-muted-foreground hover:bg-foreground/10 hover:text-foreground"

	return (
		<button
			type="button"
			aria-label={ariaLabel}
			aria-expanded={open}
			aria-haspopup="dialog"
			onClick={onClick}
			className={cn(
				"relative flex shrink-0 cursor-pointer items-center justify-center rounded-full border transition-colors",
				sizeClass,
				buttonToneClass,
			)}
		>
			<Bell className="size-3.5" aria-hidden="true" />
			{hasUnread ? (
				<span
					aria-hidden="true"
					className={cn(
						"absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[9px] leading-none font-bold tabular-nums ring-2 ring-sidebar",
						badgeClass,
					)}
				>
					{badgeText}
				</span>
			) : null}
		</button>
	)
}

export default NotificationBellButton
