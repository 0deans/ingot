import { createFileRoute } from "@tanstack/react-router"
import { z } from "zod"
import PlayPage from "@/features/play/PlayPage"

const playSearchSchema = z.object({
	q: z.string().default("").catch(""),
	engine: z.enum(["all", "Java", "Horizon (Rust)", "Bedrock"]).default("all").catch("all"),
	tag: z.enum(["all", "emerald", "amber", "rose", "violet", "cyan"]).default("all").catch("all"),
	groupBy: z.enum(["tag", "engine", "all"]).default("tag").catch("tag"),
})

export const Route = createFileRoute("/")({
	validateSearch: playSearchSchema,
	component: PlayPage,
})
