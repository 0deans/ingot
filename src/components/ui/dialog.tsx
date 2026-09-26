"use client"

import { Dialog as DialogPrimitive } from "@base-ui/react/dialog"
import { cn } from "cn"
import { XIcon } from "lucide-react"
import type * as React from "react"
import { useRef } from "react"
import { Button } from "@/components/ui/button"
import { isMobileEnvironment } from "@/lib/platform"

/** On phones every dialog is shown as a bottom sheet */
const isMobile = isMobileEnvironment()

function Dialog({ ...props }: DialogPrimitive.Root.Props) {
	return <DialogPrimitive.Root data-slot="dialog" {...props} />
}

function DialogTrigger({ ...props }: DialogPrimitive.Trigger.Props) {
	return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />
}

function DialogPortal({ ...props }: DialogPrimitive.Portal.Props) {
	return <DialogPrimitive.Portal data-slot="dialog-portal" {...props} />
}

function DialogClose({ ...props }: DialogPrimitive.Close.Props) {
	return <DialogPrimitive.Close data-slot="dialog-close" {...props} />
}

function DialogOverlay({ className, ...props }: DialogPrimitive.Backdrop.Props) {
	return (
		<DialogPrimitive.Backdrop
			data-slot="dialog-overlay"
			className={cn(
				"fixed inset-x-0 top-10 bottom-0 isolate z-50 bg-black/60 backdrop-blur-xs transition-opacity duration-200 ease-out data-closed:opacity-0 data-ending-style:opacity-0 data-open:opacity-100 data-starting-style:opacity-0",
				className,
				isMobile && "top-0",
			)}
			{...props}
		/>
	)
}

function DialogContent({
	className,
	children,
	showCloseButton = true,
	...props
}: DialogPrimitive.Popup.Props & {
	showCloseButton?: boolean
}) {
	// On phones focus the sheet itself rather than its close button, which would
	// otherwise show a focus ring every time a sheet opens
	const popupRef = useRef<HTMLDivElement>(null)
	return (
		<DialogPortal>
			<DialogOverlay />
			<DialogPrimitive.Popup
				ref={popupRef}
				initialFocus={isMobile ? popupRef : undefined}
				data-slot="dialog-content"
				className={cn(
					"-translate-1/2 fixed top-[calc(50%+1.25rem)] left-1/2 z-50 flex max-h-[calc(100vh-3.5rem)] min-h-0 w-full max-w-[calc(100%-2rem)] flex-col gap-4 overflow-hidden rounded-xl bg-popover p-4 text-popover-foreground text-sm outline-none ring-1 ring-foreground/10 transition-[opacity,transform] duration-200 ease-out data-closed:scale-95 data-ending-style:scale-95 data-open:scale-100 data-starting-style:scale-95 data-closed:opacity-0 data-ending-style:opacity-0 data-open:opacity-100 data-starting-style:opacity-0 sm:max-w-sm sm:p-5",
					className,
					// Bottom sheet: full width, anchored to the bottom, slides up
					isMobile &&
						"translate-0 top-auto bottom-0 left-0 max-h-[92dvh] w-full max-w-none rounded-t-[20px] rounded-b-none border-zinc-800 border-t bg-zinc-950 pt-6 ring-0 transition-transform duration-300 ease-[cubic-bezier(0.32,0.72,0,1)] data-closed:translate-y-full data-ending-style:translate-y-full data-open:translate-y-0 data-starting-style:translate-y-full data-closed:scale-100 data-ending-style:scale-100 data-starting-style:scale-100 data-closed:opacity-100 data-ending-style:opacity-100 data-starting-style:opacity-100",
				)}
				{...props}
			>
				{isMobile && <SheetDragHandle />}
				{children}
				{showCloseButton && (
					<DialogPrimitive.Close
						data-slot="dialog-close"
						render={
							<Button variant="ghost" className="absolute top-2 right-2 z-20" size="icon-sm" />
						}
					>
						<XIcon />
						<span className="sr-only">Close</span>
					</DialogPrimitive.Close>
				)}
			</DialogPrimitive.Popup>
		</DialogPortal>
	)
}

/**
 * Grab strip at the top of a bottom sheet: the sheet follows the finger and closes
 * when dragged far enough or flicked down.
 */
function SheetDragHandle() {
	const closeRef = useRef<HTMLButtonElement>(null)
	const drag = useRef<{ startY: number; startT: number; dy: number } | null>(null)
	const sheet = (el: Element | null) =>
		el?.closest<HTMLElement>("[data-slot=dialog-content]") ?? null

	const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
		e.currentTarget.setPointerCapture(e.pointerId)
		drag.current = { startY: e.clientY, startT: performance.now(), dy: 0 }
		const el = sheet(e.currentTarget)
		if (el) el.style.transition = "none"
	}
	const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
		if (!drag.current) return
		drag.current.dy = Math.max(0, e.clientY - drag.current.startY)
		const el = sheet(e.currentTarget)
		if (el) el.style.translate = `0 ${drag.current.dy}px`
	}
	const onPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
		const d = drag.current
		drag.current = null
		const el = sheet(e.currentTarget)
		if (!d || !el) return
		const velocity = d.dy / Math.max(1, performance.now() - d.startT)
		el.style.transition = "translate 220ms cubic-bezier(0.32,0.72,0,1)"
		if (d.dy > 110 || (d.dy > 24 && velocity > 0.5)) {
			el.style.translate = "0 100%"
			setTimeout(() => closeRef.current?.click(), 200)
		} else {
			el.style.translate = ""
		}
	}

	return (
		<>
			<div
				aria-hidden
				onPointerDown={onPointerDown}
				onPointerMove={onPointerMove}
				onPointerUp={onPointerUp}
				onPointerCancel={onPointerUp}
				className="absolute inset-x-12 top-0 z-10 flex h-7 cursor-grab touch-none justify-center pt-2"
			>
				<div className="h-1 w-10 rounded-full bg-zinc-700" />
			</div>
			<DialogPrimitive.Close ref={closeRef} className="hidden" tabIndex={-1} aria-hidden />
		</>
	)
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="dialog-header"
			className={cn("flex shrink-0 flex-col gap-2", className)}
			{...props}
		/>
	)
}

function DialogFooter({
	className,
	showCloseButton = false,
	children,
	...props
}: React.ComponentProps<"div"> & {
	showCloseButton?: boolean
}) {
	return (
		<div
			data-slot="dialog-footer"
			className={cn(
				"-mx-4 -mb-4 flex shrink-0 flex-col-reverse gap-2 rounded-b-xl border-t bg-muted/50 p-4 sm:flex-row sm:justify-end",
				className,
			)}
			{...props}
		>
			{children}
			{showCloseButton && (
				<DialogPrimitive.Close render={<Button variant="outline" />}>Close</DialogPrimitive.Close>
			)}
		</div>
	)
}

function DialogTitle({ className, ...props }: DialogPrimitive.Title.Props) {
	return (
		<DialogPrimitive.Title
			data-slot="dialog-title"
			className={cn("font-heading font-medium text-base leading-none", className)}
			{...props}
		/>
	)
}

function DialogDescription({ className, ...props }: DialogPrimitive.Description.Props) {
	return (
		<DialogPrimitive.Description
			data-slot="dialog-description"
			className={cn(
				"text-muted-foreground text-sm *:[a]:underline *:[a]:underline-offset-3 *:[a]:hover:text-foreground",
				className,
			)}
			{...props}
		/>
	)
}

export {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogOverlay,
	DialogPortal,
	DialogTitle,
	DialogTrigger,
}
