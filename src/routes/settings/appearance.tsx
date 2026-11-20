import { createFileRoute } from "@tanstack/react-router"
import AppearanceSettingsPage from "@/features/settings/AppearanceSettingsPage"

export const Route = createFileRoute("/settings/appearance")({
	component: AppearanceSettingsPage,
})
