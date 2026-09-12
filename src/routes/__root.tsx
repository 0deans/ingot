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
				<main className="flex flex-1 flex-col overflow-y-auto p-6">
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
