import { createFileRoute } from "@tanstack/react-router"
import NetworkSettingsPage from "@/features/settings/NetworkSettingsPage"

export const Route = createFileRoute("/settings/network")({
	component: NetworkSettingsPage,
})
