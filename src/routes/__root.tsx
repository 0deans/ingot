import { createRootRoute, Outlet } from "@tanstack/react-router"
import { memo } from "react"
import Sidebar from "@/components/layout/sidebar"
import WindowFrame from "@/components/layout/window-frame"
import { TooltipProvider } from "@/components/ui/tooltip"

const RootLayout = () => {
	return (
		<TooltipProvider>
			<WindowFrame title="Ingot">
				<Sidebar />
				<main className="flex size-full min-h-0 flex-1 flex-col overflow-hidden">
					<Outlet />
				</main>
			</WindowFrame>
		</TooltipProvider>
	)
}

RootLayout.displayName = "RootLayout"

const MemoizedRootLayout = memo(RootLayout)

export const Route = createRootRoute({
	component: MemoizedRootLayout,
})
