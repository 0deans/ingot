import { createFileRoute } from "@tanstack/react-router"
import GeneralSettingsPage from "@/features/settings/GeneralSettingsPage"

export const Route = createFileRoute("/settings/")({
	component: GeneralSettingsPage,
})
