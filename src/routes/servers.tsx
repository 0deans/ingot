import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import ServerListView from "@/components/servers/server-list-view"

const ServersPage = () => {
	return <ServerListView />
}

ServersPage.displayName = "ServersPage"

const MemoizedServersPage = memo(ServersPage)

export const Route = createFileRoute("/servers")({
	component: MemoizedServersPage,
})
