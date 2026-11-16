import { Link, type LinkProps } from "@tanstack/react-router"
import { type LucideIcon } from "lucide-react"
import { motion } from "motion/react"
import { cn } from "@/shared/lib/utils"

export interface RouteNavTabItem {
	key: string
	label: string
	icon?: LucideIcon
	isActive: boolean
	link: LinkProps
}

interface RouteNavTabsProps {
	ariaLabel: string
	layoutId: string
	items: readonly RouteNavTabItem[]
	className?: string
}

const RouteNavTabs = ({ ariaLabel, layoutId, items, className }: RouteNavTabsProps) => {
	return (
		<nav
			aria-label={ariaLabel}
			className={cn(
				"no-scrollbar inline-flex w-fit max-w-full items-center gap-0.5 overflow-x-auto rounded-xl border border-border bg-sidebar/80 p-1",
				className,
			)}
		>
			{items.map((item) => {
				const Icon = item.icon

				return (
					<Link
						key={item.key}
						{...item.link}
						className={cn(
							"relative flex h-7 shrink-0 items-center gap-1.5 rounded-lg px-2.5 text-xs font-medium whitespace-nowrap transition-colors",
							item.isActive ? "text-primary" : "text-muted-foreground hover:text-foreground",
						)}
					>
						{item.isActive ? (
							<motion.span
								layoutId={layoutId}
								transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
								className="pointer-events-none absolute inset-0 rounded-lg border border-primary/30 bg-primary/15 shadow-xs"
							/>
						) : null}
						{Icon ? <Icon className="relative z-10 size-3.5 shrink-0" /> : null}
						<span className="relative z-10">{item.label}</span>
					</Link>
				)
			})}
		</nav>
	)
}

export default RouteNavTabs
