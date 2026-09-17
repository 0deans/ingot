import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { createMemoryHistory, createRouter, RouterProvider } from "@tanstack/react-router"
import { getCurrentWindow } from "@tauri-apps/api/window"
import React from "react"
import ReactDOM from "react-dom/client"
import "./index.css"
import { routeTree } from "./routeTree.gen"

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

const router = createRouter({
	routeTree,
	history: memoryHistory,
	context: {
		queryClient,
	},
})

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router
	}
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	</React.StrictMode>,
)

requestAnimationFrame(() => {
	const appWindow = getCurrentWindow()
	appWindow
		.show()
		.then(() => {
			appWindow.setFocus().catch(() => {})
		})
		.catch(() => {})
})
