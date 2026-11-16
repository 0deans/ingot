import { createFileRoute } from "@tanstack/react-router"
import WorldsPage from "@/features/worlds/WorldsPage"

export const Route = createFileRoute("/worlds")({
	component: WorldsPage,
})
