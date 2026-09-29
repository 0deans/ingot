import { Dialog as DialogPrimitive } from "@base-ui/react/dialog"
import i18n from "i18next"
import { XIcon } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogClose,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogOverlay,
	DialogPortal,
	DialogTitle,
} from "@/components/ui/dialog"
import { isMobileEnvironment } from "@/lib/platform"
import { cn } from "@/lib/utils"

/*
 * shadcn's Dialog with the app's DialogContent, which differs from shadcn's in these ways:
 * - the backdrop stays below the custom desktop title bar
 * - a dialog opened from another dialog gets its own backdrop (Base UI skips it by default)
 * - it fits the screen (below the title bar on desktop) and scrolls when taller
 * - the close button's label is translated
 */

/** Classes of shadcn's DialogContent popup, unchanged */
const DIALOG_POPUP =
	"fixed top-1/2 left-1/2 z-50 grid w-full max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-4 rounded-xl bg-popover p-4 text-sm text-popover-foreground ring-1 ring-foreground/10 duration-100 outline-none sm:max-w-sm data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95"

/** The desktop app draws its own 2.5rem title bar (components/layout/titlebar); phones have none */
const isMobile = isMobileEnvironment()
const BELOW_TITLEBAR = isMobile ? undefined : "top-10"
const FIT_SCREEN = isMobile
	? "max-h-[calc(100dvh-2rem)] overflow-y-auto"
	: "top-[calc(50%+1.25rem)] max-h-[calc(100dvh-3.5rem)] overflow-y-auto"

function DialogContent({
	className,
	children,
	showCloseButton = true,
	...props
}: DialogPrimitive.Popup.Props & { showCloseButton?: boolean }) {
	return (
		<DialogPortal>
			<DialogOverlay forceRender className={BELOW_TITLEBAR} />
			<DialogPrimitive.Popup
				data-slot="dialog-content"
				className={cn(DIALOG_POPUP, FIT_SCREEN, className)}
				{...props}
			>
				{children}
				{showCloseButton && (
					<DialogClose
						render={<Button variant="ghost" size="icon-sm" className="absolute top-2 right-2" />}
					>
						<XIcon />
						<span className="sr-only">{i18n.t("common.close")}</span>
					</DialogClose>
				)}
			</DialogPrimitive.Popup>
		</DialogPortal>
	)
}

export { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle }
