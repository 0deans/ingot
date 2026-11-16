import { createFileRoute } from "@tanstack/react-router"
import JavaSettingsPage from "@/features/settings/JavaSettingsPage"

export const Route = createFileRoute("/settings/java")({
	component: JavaSettingsPage,
})
