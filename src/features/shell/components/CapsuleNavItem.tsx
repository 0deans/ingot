import { Link } from "@tanstack/react-router"
import { motion } from "motion/react"
import { useTranslation } from "react-i18next"
import { type NavItem } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"

interface CapsuleNavItemProps {
	item: NavItem
	isActive: boolean
}

const CapsuleNavItem = ({ item, isActive }: CapsuleNavItemProps) => {
	const { t } = useTranslation()
	const Icon = item.icon
	const label = t(item.labelKey)
	const shortLabel = t(item.shortLabelKey)
	const badgeText = item.badgeKey ? t(item.badgeKey) : item.badge

	return (
		<Link
			to={item.path}
			title={label}
			aria-label={label}
			className={cn(
				"relative flex shrink-0 items-center justify-center gap-1 rounded-full px-1.5 py-1 text-[11.5px] font-medium @2xl:px-2 @3xl:gap-1.5 @3xl:px-2.5",
				isActive
					? "text-foreground"
					: "text-muted-foreground hover:bg-foreground/6 hover:text-foreground",
			)}
		>
			{isActive ? (
				<motion.span
					layoutId="capsule-active-pill"
					transition={{
						type: "spring",
						stiffness: 440,
						damping: 34,
						mass: 0.85,
					}}
					className="absolute inset-0 rounded-full bg-foreground/12 shadow-xs ring-1 ring-border"
				/>
			) : null}

			<span className="relative z-10 flex shrink-0 items-center justify-center">
				<Icon className={cn("size-3.5", isActive && "text-primary")} />
				{badgeText ? (
					<span
						aria-hidden="true"
						className="absolute -top-0.5 -right-0.5 size-1.5 rounded-full bg-primary ring-1 ring-sidebar/90 @2xl:hidden"
					/>
				) : null}
			</span>

			<span className="relative z-10 hidden whitespace-nowrap @2xl:inline">{shortLabel}</span>

			{badgeText ? (
				<>
					<span
						aria-hidden="true"
						className="relative z-10 hidden size-1.5 shrink-0 rounded-full bg-primary @2xl:inline-block @3xl:hidden"
					/>
					<span className="py-0.2 relative z-10 hidden rounded-full bg-primary/20 px-1.5 text-[9px] font-semibold text-primary @3xl:inline-flex">
						{badgeText}
					</span>
				</>
			) : null}
		</Link>
	)
}

export default CapsuleNavItem
