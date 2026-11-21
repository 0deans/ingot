import { motion, useReducedMotion } from "motion/react"
import { type ReactNode, useEffect, useRef, useState } from "react"
import { cn } from "@/shared/lib/utils"

interface AnimatedHeightProps {
	children: ReactNode
	className?: string
}

const SMOOTH_EASE = [0.22, 1, 0.36, 1] as const

const AnimatedHeight = ({ children, className }: AnimatedHeightProps) => {
	const prefersReducedMotion = useReducedMotion()

	const contentRef = useRef<HTMLDivElement | null>(null)
	const hasMeasuredRef = useRef(false)

	const [height, setHeight] = useState<number | "auto">("auto")
	const [shouldAnimate, setShouldAnimate] = useState(false)

	useEffect(() => {
		const element = contentRef.current
		if (!element) {
			return
		}

		const observer = new ResizeObserver((entries) => {
			if (entries.length === 0) {
				return
			}

			const nextHeight = element.getBoundingClientRect().height
			if (!hasMeasuredRef.current) {
				hasMeasuredRef.current = true
				setHeight(nextHeight)
				return
			}

			setShouldAnimate(true)
			setHeight(nextHeight)
		})

		observer.observe(element)

		return () => {
			observer.disconnect()
		}
	}, [])

	const transition =
		prefersReducedMotion || !shouldAnimate ? { duration: 0 } : { duration: 0.22, ease: SMOOTH_EASE }

	return (
		<motion.div
			initial={false}
			animate={{ height }}
			transition={transition}
			className={cn("overflow-hidden", className)}
		>
			<div ref={contentRef}>{children}</div>
		</motion.div>
	)
}

export default AnimatedHeight
