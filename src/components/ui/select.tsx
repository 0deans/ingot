"use client"

import { Select as SelectPrimitive } from "@base-ui/react/select"
import { cn } from "cn"
import { CheckIcon, ChevronDownIcon } from "lucide-react"

function Select<Value, Multiple extends boolean | undefined = false>({
	...props
}: SelectPrimitive.Root.Props<Value, Multiple>) {
	return <SelectPrimitive.Root data-slot="select" {...props} />
}

function SelectGroup({ ...props }: SelectPrimitive.Group.Props) {
	return <SelectPrimitive.Group data-slot="select-group" {...props} />
}

function SelectValue({ className, ...props }: SelectPrimitive.Value.Props) {
	return (
		<SelectPrimitive.Value
			data-slot="select-value"
			className={cn("truncate", className)}
			{...props}
		/>
	)
}

function SelectTrigger({ className, children, ...props }: SelectPrimitive.Trigger.Props) {
	return (
		<SelectPrimitive.Trigger
			data-slot="select-trigger"
			className={cn(
				"flex h-9 w-full select-none items-center justify-between gap-2 rounded-md border border-input bg-zinc-900/90 px-3 py-1.5 text-foreground text-xs shadow-xs outline-none transition-[color,box-shadow] hover:bg-zinc-800/60 focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-[placeholder]:text-muted-foreground",
				className,
			)}
			{...props}
		>
			{children}
			<SelectPrimitive.Icon
				data-slot="select-icon"
				className="shrink-0 text-muted-foreground transition-transform duration-100"
			>
				<ChevronDownIcon className="size-4 opacity-60" />
			</SelectPrimitive.Icon>
		</SelectPrimitive.Trigger>
	)
}

function SelectContent({
	className,
	children,
	side = "bottom",
	sideOffset = 4,
	align = "start",
	alignOffset = 0,
	...props
}: SelectPrimitive.Popup.Props &
	Pick<SelectPrimitive.Positioner.Props, "side" | "sideOffset" | "align" | "alignOffset">) {
	return (
		<SelectPrimitive.Portal>
			<SelectPrimitive.Backdrop className="fixed inset-0 z-50 bg-transparent" />
			<SelectPrimitive.Positioner
				className="isolate z-50 outline-none"
				side={side}
				sideOffset={sideOffset}
				align={align}
				alignOffset={alignOffset}
			>
				<SelectPrimitive.Popup
					data-slot="select-content"
					className={cn(
						"data-open:fade-in-0 data-open:zoom-in-95 data-closed:fade-out-0 data-closed:zoom-out-95 z-50 max-h-64 min-w-(--anchor-width) origin-(--transform-origin) overflow-y-auto overflow-x-hidden rounded-lg border border-border/60 bg-popover p-1 text-popover-foreground shadow-xl outline-none ring-1 ring-foreground/10 duration-100 [scrollbar-color:rgba(255,255,255,0.2)_transparent] [scrollbar-width:thin] data-closed:animate-out data-open:animate-in [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-white/20 hover:[&::-webkit-scrollbar-thumb]:bg-white/35 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar]:w-1.5",
						className,
					)}
					{...props}
				>
					<SelectPrimitive.List className="flex flex-col gap-0.5">{children}</SelectPrimitive.List>
				</SelectPrimitive.Popup>
			</SelectPrimitive.Positioner>
		</SelectPrimitive.Portal>
	)
}

function SelectLabel({ className, ...props }: SelectPrimitive.GroupLabel.Props) {
	return (
		<SelectPrimitive.GroupLabel
			data-slot="select-label"
			className={cn("px-2 py-1.5 font-medium text-muted-foreground text-xs", className)}
			{...props}
		/>
	)
}

function SelectItem({ className, children, ...props }: SelectPrimitive.Item.Props) {
	return (
		<SelectPrimitive.Item
			data-slot="select-item"
			className={cn(
				"relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pr-8 pl-2 text-foreground text-xs outline-none transition-colors data-disabled:pointer-events-none data-highlighted:bg-accent data-highlighted:text-accent-foreground data-disabled:opacity-50",
				className,
			)}
			{...props}
		>
			<SelectPrimitive.ItemIndicator className="absolute right-2 flex size-3.5 items-center justify-center text-primary">
				<CheckIcon className="size-3.5" />
			</SelectPrimitive.ItemIndicator>
			<SelectPrimitive.ItemText className="truncate">{children}</SelectPrimitive.ItemText>
		</SelectPrimitive.Item>
	)
}

function SelectSeparator({ className, ...props }: SelectPrimitive.Separator.Props) {
	return (
		<SelectPrimitive.Separator
			data-slot="select-separator"
			className={cn("-mx-1 my-1 h-px bg-border", className)}
			{...props}
		/>
	)
}

export {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
}
