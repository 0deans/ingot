import type { QueryClient } from "@tanstack/react-query"
import {
	createRootRouteWithContext,
	Outlet,
	useNavigate,
	useRouterState,
} from "@tanstack/react-router"
import { memo, useEffect } from "react"
import Sidebar from "@/components/layout/sidebar"
import UpdateBanner from "@/components/layout/update-banner"
import WindowFrame from "@/components/layout/window-frame"
import { TooltipProvider } from "@/components/ui/tooltip"
import { isMobileEnvironment } from "@/lib/platform"
import { useScrollRestoration } from "@/lib/scroll-restoration"

export interface RouterContext {
	queryClient: QueryClient
}

const RootLayout = () => {
	const mainRef = useScrollRestoration()
	const isMobile = isMobileEnvironment()
	const navigate = useNavigate()
	const currentPath = useRouterState({ select: (s) => s.location.pathname })

	useEffect(() => {
		if (isMobile && currentPath !== "/servers") {
			navigate({ to: "/servers" })
		}
	}, [isMobile, currentPath, navigate])

	if (isMobile) {
		return (
			<TooltipProvider>
				<main
					ref={mainRef}
					className="flex size-full min-h-screen flex-1 flex-col overflow-y-auto bg-zinc-950"
				>
					<Outlet />
				</main>
			</TooltipProvider>
		)
	}

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
