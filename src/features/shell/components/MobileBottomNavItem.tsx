import { Link } from "@tanstack/react-router"
import { motion } from "motion/react"
import { type NavItem } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"

interface MobileBottomNavItemProps {
	item: NavItem
	isActive: boolean
}

const MobileBottomNavItem = ({ item, isActive }: MobileBottomNavItemProps) => {
	const Icon = item.icon

	return (
		<Link
			to={item.path}
			className={cn(
				"relative flex flex-col items-center justify-center gap-0.5 rounded-xl py-1.5 text-[10px] font-medium",
				isActive ? "text-primary" : "text-muted-foreground hover:text-foreground",
			)}
		>
			{isActive ? (
				<motion.span
					layoutId="mobile-tab-pill"
					transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
					className="absolute inset-0 rounded-xl bg-foreground/10"
				/>
			) : null}
			<Icon className="relative z-10 size-4" />
			<span className="relative z-10 truncate">{item.shortLabel}</span>
		</Link>
	)
}

export default MobileBottomNavItem
