"use client"

import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip"
import { cn } from "cn"

function TooltipProvider({
  delay = 400,
  closeDelay = 150,
  timeout = 400,
  ...props
}: TooltipPrimitive.Provider.Props) {
  return (
    <TooltipPrimitive.Provider
      data-slot="tooltip-provider"
      delay={delay}
      closeDelay={closeDelay}
      timeout={timeout}
      {...props}
    />
  )
}

function Tooltip({ ...props }: TooltipPrimitive.Root.Props) {
  return <TooltipPrimitive.Root data-slot="tooltip" {...props} />
}

function TooltipTrigger({ ...props }: TooltipPrimitive.Trigger.Props) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />
}

function TooltipContent({
  className,
  side = "top",
  sideOffset = 8,
  align = "center",
  alignOffset = 0,
  children,
  showArrow = true,
  ...props
}: TooltipPrimitive.Popup.Props &
  Pick<
    TooltipPrimitive.Positioner.Props,
    "align" | "alignOffset" | "side" | "sideOffset"
  > & {
    showArrow?: boolean
  }) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Positioner
        align={align}
        alignOffset={alignOffset}
        side={side}
        sideOffset={sideOffset}
        className="isolate z-50"
      >
        <TooltipPrimitive.Popup
          data-slot="tooltip-content"
          className={cn(
            "relative z-50 flex w-fit max-w-xs origin-(--transform-origin) items-center gap-1.5 rounded-lg border border-border/80 bg-popover px-2.5 py-1 text-xs font-medium text-popover-foreground shadow-md backdrop-blur-xs has-data-[slot=kbd]:pr-1.5 data-[side=bottom]:slide-in-from-top-1.5 data-[side=inline-end]:slide-in-from-left-1.5 data-[side=inline-start]:slide-in-from-right-1.5 data-[side=left]:slide-in-from-right-1.5 data-[side=right]:slide-in-from-left-1.5 data-[side=top]:slide-in-from-bottom-1.5 **:data-[slot=kbd]:relative **:data-[slot=kbd]:isolate **:data-[slot=kbd]:z-50 **:data-[slot=kbd]:rounded-sm data-[state=delayed-open]:animate-in data-[state=delayed-open]:fade-in-0 data-[state=delayed-open]:zoom-in-95 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95",
            className
          )}
          {...props}
        >
          {children}
          {showArrow && (
            <TooltipPrimitive.Arrow
              className={cn(
                "z-50 block h-1.5 w-3 overflow-clip",
                "data-[side=bottom]:-top-1.5 data-[side=bottom]:rotate-0",
                "data-[side=top]:-bottom-1.5 data-[side=top]:rotate-180",
                "data-[side=left]:-right-2.25 data-[side=left]:rotate-90",
                "data-[side=inline-start]:-right-2.25 data-[side=inline-start]:rotate-90",
                "data-[side=right]:-left-2.25 data-[side=right]:-rotate-90",
                "data-[side=inline-end]:-left-2.25 data-[side=inline-end]:-rotate-90",
                "before:absolute before:bottom-0 before:left-1/2 before:size-[8.485px] before:border before:border-border/80 before:bg-popover before:content-[''] before:transform-[translate(-50%,50%)_rotate(45deg)]"
              )}
            />
          )}
        </TooltipPrimitive.Popup>
      </TooltipPrimitive.Positioner>
    </TooltipPrimitive.Portal>
  )
}

export { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider }
