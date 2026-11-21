import { Check } from "lucide-react"
import { motion } from "motion/react"
import { cn } from "@/shared/lib/utils"

interface SelectionCheckBadgeProps {
	size?: "sm" | "md"
	className?: string
}

const SelectionCheckBadge = ({ size = "sm", className }: SelectionCheckBadgeProps) => {
	const isMedium = size === "md"

	return (
		<motion.span
			initial={{ scale: 0.6, opacity: 0 }}
			animate={{ scale: 1, opacity: 1 }}
			transition={{ type: "spring", stiffness: 500, damping: 28 }}
			className={cn(
				"flex shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground",
				isMedium ? "size-5" : "size-4",
				className,
			)}
		>
			<Check className={isMedium ? "size-3" : "size-2.5"} />
		</motion.span>
	)
}

export default SelectionCheckBadge
