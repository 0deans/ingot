import { memo, type ReactNode } from "react"
import { useAppMotion } from "@/lib/motion"
import { cn } from "@/lib/utils"

interface PageTransitionProps {
	children: ReactNode
	transitionKey: string
	className?: string
}

function PageTransitionComponent({
	children,
	transitionKey,
	className = "flex size-full min-h-0 flex-1 flex-col overflow-hidden",
}: PageTransitionProps) {
	const { reducedMotion, style } = useAppMotion()

	if (reducedMotion || style === "instant") {
		return <div className={className}>{children}</div>
	}

	return (
		<div
			key={transitionKey}
			className={cn(
				className,
				"transform-gpu",
				style === "apple" && "animate-apple-page-enter",
				style === "fade" && "animate-fade-page-enter",
			)}
		>
			{children}
		</div>
	)
}

PageTransitionComponent.displayName = "PageTransition"

export const PageTransition = memo(PageTransitionComponent)
