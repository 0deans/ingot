import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { createMemoryHistory, createRouter, RouterProvider } from "@tanstack/react-router"
import { getCurrentWindow } from "@tauri-apps/api/window"
import React from "react"
import ReactDOM from "react-dom/client"
import "@/i18n"
import "@/lib/theme"
import "./index.css"
import { isMobileEnvironment } from "./lib/platform"
import { recordLocation } from "./lib/tab-history"
import { routeTree } from "./routeTree.gen"

// Lets CSS tell the desktop window (with its custom title bar) from the phone layout
document.documentElement.dataset.platform = isMobileEnvironment() ? "mobile" : "desktop"

export const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 1000 * 60 * 5, // 5 minutes
			gcTime: 1000 * 60 * 30, // 30 minutes
			refetchOnWindowFocus: false,
		},
	},
})

const memoryHistory = createMemoryHistory({
	initialEntries: ["/"],
})

export const router = createRouter({
	routeTree,
	history: memoryHistory,
	scrollRestoration: true,
	defaultPreload: "intent",
	defaultPreloadDelay: 50,
	defaultPreloadStaleTime: 1000 * 60 * 5, // 5 minutes
	context: {
		queryClient,
	},
})

// Automatically track section locations across all navigations
recordLocation(router.history.location.href, router.history.location.pathname)
router.history.subscribe((event) => {
	recordLocation(event.location.href, event.location.pathname)
})

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router
	}
}

const rootElement = document.getElementById("root")
if (rootElement) {
	ReactDOM.createRoot(rootElement).render(
		<React.StrictMode>
			<QueryClientProvider client={queryClient}>
				<RouterProvider router={router} />
			</QueryClientProvider>
		</React.StrictMode>,
	)
}

router.load().finally(() => {
	requestAnimationFrame(() => {
		const appWindow = getCurrentWindow()
		appWindow
			.show()
			.then(() => {
				appWindow.setFocus().catch(() => {})
			})
			.catch(() => {})
	})
})
