import { Dialog as DialogPrimitive } from "@base-ui/react/dialog"
import i18n from "i18next"
import { XIcon } from "lucide-react"
import { type ComponentProps, useRef } from "react"
import { Button } from "@/components/ui/button"
import {
	DialogClose,
	DialogFooter,
	DialogHeader,
	DialogOverlay,
	DialogPortal,
	Dialog as ShadcnDialog,
	DialogDescription as ShadcnDialogDescription,
	DialogTitle as ShadcnDialogTitle,
} from "@/components/ui/dialog"
import {
	Drawer,
	DrawerClose,
	DrawerContent,
	DrawerDescription,
	DrawerTitle,
} from "@/components/ui/drawer"
import { isMobileEnvironment } from "@/lib/platform"
import { cn } from "@/lib/utils"

/*
 * The app's dialog, assembled from shadcn parts with the same API as shadcn's Dialog:
 * - on phones it is a shadcn Drawer: a bottom sheet you can swipe down to close
 * - on desktop the backdrop leaves the custom title bar uncovered, and a dialog opened from
 *   another dialog gets its own backdrop (Base UI skips it for nested dialogs by default)
 */

const isMobile = isMobileEnvironment()

/** Classes of shadcn's DialogContent popup, unchanged */
const DIALOG_POPUP =
	"fixed top-1/2 left-1/2 z-50 grid w-full max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-4 rounded-xl bg-popover p-4 text-sm text-popover-foreground ring-1 ring-foreground/10 duration-100 outline-none sm:max-w-sm data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95"

/** Keeps the backdrop below the custom title bar (see components/layout/titlebar) */
const BELOW_TITLEBAR = "top-10"

function Dialog(props: ComponentProps<typeof ShadcnDialog>) {
	if (isMobile) {
		return <Drawer showSwipeHandle {...(props as ComponentProps<typeof Drawer>)} />
	}
	return <ShadcnDialog {...props} />
}

type DialogContentProps = DialogPrimitive.Popup.Props & { showCloseButton?: boolean }

function DialogContent(props: DialogContentProps) {
	return isMobile ? <SheetContent {...props} /> : <DesktopContent {...props} />
}

function DesktopContent({
	className,
	children,
	showCloseButton = true,
	...props
}: DialogContentProps) {
	return (
		<DialogPortal>
			<DialogOverlay forceRender className={BELOW_TITLEBAR} />
			<DialogPrimitive.Popup
				data-slot="dialog-content"
				className={cn(DIALOG_POPUP, className)}
				{...props}
			>
				{children}
				{showCloseButton && (
					<DialogClose render={<CloseButton />}>
						<CloseIcon />
					</DialogClose>
				)}
			</DialogPrimitive.Popup>
		</DialogPortal>
	)
}

function SheetContent({ className, children, showCloseButton = true }: DialogContentProps) {
	// Focus the sheet itself rather than its close button, which would otherwise show a
	// focus ring every time a sheet opens
	const popupRef = useRef<HTMLDivElement>(null)

	return (
		<DrawerContent ref={popupRef} initialFocus={popupRef}>
			{/* Caller sizes are meant for the centered desktop dialog; a sheet is always full width */}
			<div
				className={cn(
					"flex min-h-0 flex-1 flex-col gap-4 p-4 pt-2",
					className,
					"w-full max-w-none",
				)}
			>
				{children}
			</div>
			{showCloseButton && (
				<DrawerClose render={<CloseButton />}>
					<CloseIcon />
				</DrawerClose>
			)}
		</DrawerContent>
	)
}

function CloseButton({ className, ...props }: ComponentProps<typeof Button>) {
	return (
		<Button
			variant="ghost"
			size="icon-sm"
			className={cn("absolute top-2 right-2", className)}
			{...props}
		/>
	)
}

function CloseIcon() {
	return (
		<>
			<XIcon />
			<span className="sr-only">{i18n.t("common.close")}</span>
		</>
	)
}

function DialogTitle(props: ComponentProps<typeof ShadcnDialogTitle>) {
	if (isMobile) return <DrawerTitle {...(props as ComponentProps<typeof DrawerTitle>)} />
	return <ShadcnDialogTitle {...props} />
}

function DialogDescription(props: ComponentProps<typeof ShadcnDialogDescription>) {
	if (isMobile) {
		return <DrawerDescription {...(props as ComponentProps<typeof DrawerDescription>)} />
	}
	return <ShadcnDialogDescription {...props} />
}

export { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle }
