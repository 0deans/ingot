import { createFileRoute } from "@tanstack/react-router"
import ModpacksPage from "@/features/modpacks/ModpacksPage"

export const Route = createFileRoute("/modpacks/")({
	component: ModpacksPage,
})
