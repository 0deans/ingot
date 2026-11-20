import { createFileRoute } from "@tanstack/react-router"
import GameSettingsPage from "@/features/settings/GameSettingsPage"

export const Route = createFileRoute("/settings/game")({
	component: GameSettingsPage,
})
