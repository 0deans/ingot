import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import * as v from "valibot"
import ScreenshotsView from "@/components/screenshots/screenshots-view"
import { screenshotService } from "@/services/screenshot-service"

export const screenshotSortSchema = v.picklist([
	"date-desc",
	"date-asc",
	"name-asc",
	"name-desc",
	"size-desc",
	"size-asc",
])

export const screenshotsSearchSchema = v.object({
	q: v.optional(v.fallback(v.string(), ""), ""),
	instance: v.optional(v.fallback(v.string(), "all"), "all"),
	sort: v.optional(v.fallback(screenshotSortSchema, "date-desc"), "date-desc"),
	lightbox: v.optional(v.string()),
	delete: v.optional(v.string()),
})

export type ScreenshotsSearchParams = v.InferOutput<typeof screenshotsSearchSchema>

const ScreenshotsPage = () => {
	return <ScreenshotsView />
}

ScreenshotsPage.displayName = "ScreenshotsPage"

const MemoizedScreenshotsPage = memo(ScreenshotsPage)

export const Route = createFileRoute("/screenshots")({
	validateSearch: screenshotsSearchSchema,
	loader: async ({ context }) => {
		await context.queryClient.ensureQueryData({
			queryKey: ["screenshots"],
			queryFn: () => screenshotService.getScreenshots(),
		})
	},
	component: MemoizedScreenshotsPage,
})
