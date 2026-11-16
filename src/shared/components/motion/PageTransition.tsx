import { motion } from "motion/react"
import type { ReactNode } from "react"

interface PageTransitionProps {
	routeKey: string
	children: ReactNode
}

const PageTransition = ({ routeKey, children }: PageTransitionProps) => {
	return (
		<motion.div
			key={routeKey}
			initial={{ opacity: 0, y: 6, scale: 0.994 }}
			animate={{ opacity: 1, y: 0, scale: 1 }}
			transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
			className="w-full"
		>
			{children}
		</motion.div>
	)
}

export default PageTransition
