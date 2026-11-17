import { createFileRoute } from "@tanstack/react-router"
import LanguageSettingsPage from "@/features/settings/LanguageSettingsPage"

export const Route = createFileRoute("/settings/language")({
	component: LanguageSettingsPage,
})
