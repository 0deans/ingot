import { createFileRoute } from "@tanstack/react-router"
import NewsPage from "@/features/news/NewsPage"

export const Route = createFileRoute("/news")({
	component: NewsPage,
})
