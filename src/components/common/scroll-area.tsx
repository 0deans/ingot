import type { ComponentProps } from "react"
import { ScrollArea as ShadcnScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"

interface ScrollAreaProps extends ComponentProps<typeof ShadcnScrollArea> {
	/** Fade the top and bottom edges while there is more to scroll (see .scroll-fade in index.css) */
	scrollFade?: boolean
}

/**
 * shadcn ScrollArea laid out as a flex column, so it fills a flex parent and also scrolls
 * inside a max-height (the viewport shrinks instead of growing with its content).
 */
function ScrollArea({ scrollFade = false, className, ...props }: ScrollAreaProps) {
	return (
		<ShadcnScrollArea
			data-scroll-fade={scrollFade || undefined}
			className={cn(
				"flex min-h-0 flex-col overflow-hidden *:data-[slot=scroll-area-viewport]:min-h-0 *:data-[slot=scroll-area-viewport]:flex-1",
				className,
			)}
			{...props}
		/>
	)
}

export { ScrollArea }
