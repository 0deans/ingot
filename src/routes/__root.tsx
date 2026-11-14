import { createRootRoute, Outlet, useRouterState } from "@tanstack/react-router"
import { useEffect } from "react"

const RootShellComponent = () => {
	const pathname = useRouterState({
		select: (s) => {
			return s.location.pathname
		},
	})

	useEffect(() => {
		import("@tauri-apps/api/window")
			.then(({ getCurrentWindow }) => {
				return getCurrentWindow().show()
			})
			.catch(() => {
				// Ignored when running in a standard web browser
			})
	}, [])

	return (
		<div className="h-screen w-screen overflow-hidden bg-[#09090b] text-zinc-100 flex flex-col select-none [-webkit-tap-highlight-color:transparent] pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)]">
			<header
				data-tauri-drag-region
				className="h-9 shrink-0 bg-zinc-950/80 backdrop-blur-xl border-b border-white/[0.07] px-3 flex items-center justify-between"
			>
				<div className="flex items-center gap-2 pointer-events-none">
					<div className="w-4 h-4 rounded-md bg-gradient-to-br from-emerald-400 to-teal-600 flex items-center justify-center font-black text-zinc-950 text-[10px]">
						I
					</div>
					<span className="text-xs font-semibold tracking-tight text-zinc-100">Ingot</span>
				</div>
				<span className="text-[10px] font-mono text-zinc-500 pointer-events-none">{pathname}</span>
			</header>
			<main id="ingot-scroll-viewport" className="flex-1 overflow-y-auto overflow-x-clip min-h-0">
				<Outlet />
			</main>
		</div>
	)
}

export const Route = createRootRoute({
	component: RootShellComponent,
})
