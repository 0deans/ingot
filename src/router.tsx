import { createRouter } from "@tanstack/react-router"
import ErrorFallbackView from "@/shared/components/errors/ErrorFallbackView"
import NotFoundView from "@/shared/components/errors/NotFoundView"
import { routeTree } from "./routeTree.gen"

export const router = createRouter({
	routeTree,
	scrollRestoration: true,
	defaultPreload: "intent",
	defaultPreloadStaleTime: 30_000,
	notFoundMode: "root",
	defaultNotFoundComponent: NotFoundView,
	defaultErrorComponent: ErrorFallbackView,
})

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router
	}
}
