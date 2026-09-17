import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import * as v from "valibot"
import ServerListView from "@/components/servers/server-list-view"
import { serverService } from "@/services/server-service"

export const serverCoreSchema = v.picklist(["all", "paper", "purpur", "fabric", "folia", "vanilla"])

export const serversSearchSchema = v.object({
	q: v.optional(v.fallback(v.string(), ""), ""),
	core: v.optional(v.fallback(serverCoreSchema, "all"), "all"),
	action: v.optional(v.picklist(["new"])),
	console: v.optional(v.string()),
	settings: v.optional(v.string()),
	delete: v.optional(v.string()),
})

export type ServersSearchParams = v.InferOutput<typeof serversSearchSchema>

const ServersPage = () => {
	return <ServerListView />
}

ServersPage.displayName = "ServersPage"

const MemoizedServersPage = memo(ServersPage)

export const Route = createFileRoute("/servers")({
	validateSearch: serversSearchSchema,
	loader: async ({ context }) => {
		await context.queryClient.ensureQueryData({
			queryKey: ["servers"],
			queryFn: () => serverService.getServers(),
		})
	},
	component: MemoizedServersPage,
})
