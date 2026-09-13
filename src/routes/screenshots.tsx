import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import ScreenshotsView from "@/components/screenshots/screenshots-view"

const ScreenshotsPage = () => {
	return <ScreenshotsView />
}

ScreenshotsPage.displayName = "ScreenshotsPage"

const MemoizedScreenshotsPage = memo(ScreenshotsPage)

export const Route = createFileRoute("/screenshots")({
	component: MemoizedScreenshotsPage,
})
