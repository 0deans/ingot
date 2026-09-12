import { ScrollArea as ScrollAreaPrimitive } from "@base-ui/react/scroll-area"
import { cn } from "cn"

interface ScrollAreaProps extends ScrollAreaPrimitive.Root.Props {
	viewportClassName?: string
	scrollFade?: boolean
}

function ScrollArea({
	className,
	children,
	viewportClassName,
	scrollFade = false,
	...props
}: ScrollAreaProps) {
	return (
		<ScrollAreaPrimitive.Root
			data-slot="scroll-area"
			className={cn("relative flex min-h-0 flex-col overflow-hidden", className)}
			{...props}
		>
			<ScrollAreaPrimitive.Viewport
				data-slot="scroll-area-viewport"
				className={cn(
					"size-full min-h-0 flex-1 rounded-[inherit] outline-none transition-[color,box-shadow] focus-visible:outline-1 focus-visible:ring-[3px] focus-visible:ring-ring/50",
					scrollFade && "scroll-fade",
					viewportClassName,
				)}
			>
				{children}
			</ScrollAreaPrimitive.Viewport>
			<ScrollBar />
			<ScrollAreaPrimitive.Corner />
		</ScrollAreaPrimitive.Root>
	)
}

function ScrollBar({
	className,
	orientation = "vertical",
	...props
}: ScrollAreaPrimitive.Scrollbar.Props) {
	return (
		<ScrollAreaPrimitive.Scrollbar
			data-slot="scroll-area-scrollbar"
			data-orientation={orientation}
			orientation={orientation}
			className={cn(
				"flex touch-none select-none p-0.5 transition-colors data-horizontal:h-2 data-vertical:h-full data-vertical:w-2 data-horizontal:flex-col data-horizontal:border-t data-horizontal:border-t-transparent data-vertical:border-l data-vertical:border-l-transparent",
				className,
			)}
			{...props}
		>
			<ScrollAreaPrimitive.Thumb
				data-slot="scroll-area-thumb"
				className="relative flex-1 rounded-full bg-muted-foreground/30 transition-colors hover:bg-muted-foreground/50"
			/>
		</ScrollAreaPrimitive.Scrollbar>
	)
}

export { ScrollArea, ScrollBar }
