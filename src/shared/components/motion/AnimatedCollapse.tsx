import { AnimatePresence, motion } from "motion/react"
import type { ReactNode } from "react"
import { cn } from "@/shared/lib/utils"

interface AnimatedCollapseProps {
	show: boolean
	children: ReactNode
	className?: string
}

const AnimatedCollapse = ({ show, children, className }: AnimatedCollapseProps) => {
	return (
		<AnimatePresence initial={false}>
			{show ? (
				<motion.div
					initial={{ height: 0, opacity: 0, y: -4 }}
					animate={{ height: "auto", opacity: 1, y: 0 }}
					exit={{ height: 0, opacity: 0, y: -4 }}
					transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
					className={cn("overflow-hidden", className)}
				>
					{children}
				</motion.div>
			) : null}
		</AnimatePresence>
	)
}

export default AnimatedCollapse
