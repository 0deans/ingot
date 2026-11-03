import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import * as v from "valibot"
import WorldsView from "@/components/worlds/worlds-view"
import { worldService } from "@/services/world-service"

export const worldSortSchema = v.picklist([
	"lastPlayed-desc",
	"lastPlayed-asc",
	"name-asc",
	"name-desc",
	"size-desc",
	"size-asc",
])

export const worldsSearchSchema = v.object({
	q: v.optional(v.fallback(v.string(), ""), ""),
	instance: v.optional(v.fallback(v.string(), "all"), "all"),
	sort: v.optional(v.fallback(worldSortSchema, "lastPlayed-desc"), "lastPlayed-desc"),
})

export type WorldsSearchParams = v.InferOutput<typeof worldsSearchSchema>

const WorldsPage = () => {
	return <WorldsView />
}

WorldsPage.displayName = "WorldsPage"

const MemoizedWorldsPage = memo(WorldsPage)

export const Route = createFileRoute("/worlds")({
	validateSearch: worldsSearchSchema,
	loader: async ({ context }) => {
		await context.queryClient.ensureQueryData({
			queryKey: ["worlds", "all"],
			queryFn: () => {
				return worldService.getAllWorlds()
			},
		})
	},
	component: MemoizedWorldsPage,
})
