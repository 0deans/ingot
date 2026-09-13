import { createRootRoute, Outlet } from "@tanstack/react-router"
import { memo } from "react"
import Sidebar from "@/components/layout/sidebar"
import WindowFrame from "@/components/layout/window-frame"
import { ScrollArea } from "@/components/ui/scroll-area"
import { TooltipProvider } from "@/components/ui/tooltip"

const RootLayout = () => {
	return (
		<TooltipProvider>
			<WindowFrame title="Ingot">
				<Sidebar />
				<ScrollArea className="size-full flex-1" scrollFade>
					<main className="flex size-full min-h-full flex-1 flex-col p-4 sm:p-5 lg:p-6">
						<Outlet />
					</main>
				</ScrollArea>
			</WindowFrame>
		</TooltipProvider>
	)
}

RootLayout.displayName = "RootLayout"

const MemoizedRootLayout = memo(RootLayout)

export const Route = createRootRoute({
	component: MemoizedRootLayout,
})
