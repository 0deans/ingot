import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"

const playSearchSchema = z.object({
	q: z.string().catch(""),
	engine: z.enum(["all", "Java", "Horizon (Rust)", "Bedrock"]).catch("all"),
	tag: z.enum(["all", "emerald", "amber", "rose", "violet", "cyan"]).catch("all"),
	group: z.enum(["tag", "engine", "all"]).catch("tag"),
})

const IndexHelloWorldPage = () => {
	const search = Route.useSearch()
	const navigate = Route.useNavigate()

	return (
		<div className="flex h-full flex-col items-center justify-center gap-4 p-6 text-center">
			<div className="h-12 w-12 rounded-2xl bg-gradient-to-br from-emerald-400 to-teal-600 flex items-center justify-center font-black text-zinc-950 text-xl shadow-lg shadow-emerald-500/20">
				I
			</div>
			<div className="space-y-1">
				<h1 className="text-2xl font-bold tracking-tight text-white">
					Hello World — Ingot Launcher
				</h1>
				<p className="text-xs text-zinc-400">
					TanStack Router + Tauri v2 Shell is mounted and ready.
				</p>
			</div>
			<div className="flex items-center gap-2">
				{(["all", "Java", "Horizon (Rust)", "Bedrock"] as const).map((eng) => {
					return (
						<button
							key={eng}
							type="button"
							onClick={() => {
								void navigate({
									search: (prev) => {
										return { ...prev, engine: eng }
									},
								})
							}}
							className={`px-3 py-1.5 rounded-xl text-xs font-medium border transition-colors ${
								search.engine === eng
									? "bg-emerald-500/20 border-emerald-500/40 text-emerald-300"
									: "bg-white/[0.04] border-white/[0.08] text-zinc-400 hover:text-zinc-200"
							}`}
						>
							{eng === "all" ? "All Engines" : eng}
						</button>
					)
				})}
			</div>
		</div>
	)
}

export const Route = createFileRoute("/")({
	validateSearch: playSearchSchema,
	component: IndexHelloWorldPage,
})
