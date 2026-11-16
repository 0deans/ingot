import { Link } from "@tanstack/react-router"
import { type LucideIcon } from "lucide-react"
import { motion } from "motion/react"
import { type AppRoutePath } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"

interface SidebarNavItemProps {
	path: AppRoutePath
	label: string
	icon: LucideIcon
	isActive: boolean
	isIconOnly: boolean
	badgeText?: string
	badgeHighlight?: boolean
	showCompactDot?: boolean
}

const SidebarNavItem = ({
	path,
	label,
	icon: Icon,
	isActive,
	isIconOnly,
	badgeText,
	badgeHighlight = false,
	showCompactDot = false,
}: SidebarNavItemProps) => {
	const badgeClass = badgeHighlight
		? "bg-primary/20 text-primary tabular-nums"
		: "bg-foreground/10 text-foreground/80"

	return (
		<Link
			to={path}
			title={isIconOnly ? label : undefined}
			className={cn(
				"relative flex h-9 items-center rounded-xl",
				isIconOnly ? "justify-center px-2" : "gap-2.5 px-2.5",
				isActive
					? "text-sidebar-foreground"
					: "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
			)}
		>
			{isActive ? (
				<motion.span
					layoutId="sidebar-active-pill"
					transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
					className="absolute inset-0 rounded-xl border border-foreground/12 bg-foreground/10 shadow-xs"
				/>
			) : null}

			<Icon className={cn("relative z-10 size-4 shrink-0", isActive && "text-sidebar-primary")} />

			{!isIconOnly ? (
				<>
					<span className="relative z-10 min-w-0 flex-1 truncate text-[12.5px] font-medium whitespace-nowrap">
						{label}
					</span>
					{badgeText ? (
						<span
							className={cn(
								"relative z-10 shrink-0 rounded-full px-1.5 py-0.5 text-[10px] font-semibold",
								badgeClass,
							)}
						>
							{badgeText}
						</span>
					) : null}
				</>
			) : null}

			{isIconOnly && showCompactDot ? (
				<span className="absolute top-1.5 right-1.5 z-10 size-1.5 rounded-full bg-primary" />
			) : null}
		</Link>
	)
}

export default SidebarNavItem
