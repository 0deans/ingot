import type { QueryClient } from "@tanstack/react-query"
import { createRootRouteWithContext, Outlet } from "@tanstack/react-router"
import { memo } from "react"
import Sidebar from "@/components/layout/sidebar"
import UpdateBanner from "@/components/layout/update-banner"
import WindowFrame from "@/components/layout/window-frame"
import { TooltipProvider } from "@/components/ui/tooltip"
import { useScrollRestoration } from "@/lib/scroll-restoration"

export interface RouterContext {
	queryClient: QueryClient
}

const RootLayout = () => {
	const mainRef = useScrollRestoration()

	return (
		<TooltipProvider>
			<WindowFrame title="Ingot">
				<Sidebar />
				<main ref={mainRef} className="flex size-full min-h-0 flex-1 flex-col overflow-hidden">
					<UpdateBanner />
					<Outlet />
				</main>
			</WindowFrame>
		</TooltipProvider>
	)
}

RootLayout.displayName = "RootLayout"

const MemoizedRootLayout = memo(RootLayout)

export const Route = createRootRouteWithContext<RouterContext>()({
	component: MemoizedRootLayout,
})
