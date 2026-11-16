import { createFileRoute } from "@tanstack/react-router"
import ScreenshotsPage from "@/features/screenshots/ScreenshotsPage"

export const Route = createFileRoute("/screenshots")({
	component: ScreenshotsPage,
})
