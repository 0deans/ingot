import { createRouter, type ErrorComponentProps } from "@tanstack/react-router"
import { routeTree } from "./routeTree.gen"

const NotFoundFallback = () => {
	return (
		<div className="flex h-full flex-col items-center justify-center gap-3 text-center p-6">
			<span className="text-xs font-mono uppercase tracking-wider text-amber-400">
				404 • Route Missing
			</span>
			<h2 className="text-lg font-bold text-white">Page Not Found</h2>
			<button
				type="button"
				onClick={() => {
					void router.navigate({ to: "/" })
				}}
				className="px-4 py-2 rounded-xl bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 text-xs font-semibold hover:bg-emerald-500/30 transition-colors"
			>
				Return to Play
			</button>
		</div>
	)
}

const ErrorFallback = ({ error, reset }: ErrorComponentProps) => {
	return (
		<div className="flex h-full flex-col items-center justify-center gap-3 text-center p-6">
			<span className="text-xs font-mono uppercase tracking-wider text-rose-400">
				Runtime Error
			</span>
			<h2 className="text-base font-bold text-white">Something went wrong rendering this view</h2>
			<p className="max-w-md text-xs font-mono text-zinc-400 bg-white/[0.04] border border-white/10 rounded-xl p-3">
				{error instanceof Error ? error.message : String(error)}
			</p>
			<button
				type="button"
				onClick={reset}
				className="px-4 py-2 rounded-xl bg-white/10 text-white text-xs font-semibold hover:bg-white/15 transition-colors"
			>
				Retry Route
			</button>
		</div>
	)
}

export const router = createRouter({
	routeTree,
	defaultPreload: "intent",
	defaultPreloadDelay: 50,
	defaultPreloadStaleTime: 30_000,
	scrollRestoration: true,
	scrollToTopSelectors: ["#ingot-scroll-viewport"],
	getScrollRestorationKey: (location) => {
		return location.pathname
	},
	defaultNotFoundComponent: NotFoundFallback,
	defaultErrorComponent: ErrorFallback,
})

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router
	}
}
