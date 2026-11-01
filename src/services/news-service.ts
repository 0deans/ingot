import { queryOptions } from "@tanstack/react-query"
import type { NewsArticle, NewsFeedResponse, PatchNoteDetails, PatchNoteSummary } from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

export const newsService = {
	async getNewsFeed(forceRefresh = false): Promise<NewsFeedResponse> {
		return await unwrapRpc(rpc.news.get_news_feed(forceRefresh))
	},

	async getPatchNoteDetails(contentPath: string): Promise<PatchNoteDetails> {
		return await unwrapRpc(rpc.news.get_patch_note_details(contentPath))
	},
}

export const getMinecraftNetArticleUrl = (title: string, version: string): string => {
	const cleaned = title
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-+|-+$/g, "")

	if (cleaned.startsWith("minecraft")) {
		return `https://www.minecraft.net/article/${cleaned}`
	}
	const versionSlug = version
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "-")
		.replace(/^-+|-+$/g, "")
	if (versionSlug) {
		return `https://www.minecraft.net/article/minecraft-${versionSlug}`
	}
	return "https://www.minecraft.net/category/news"
}

export const newsQueries = {
	feed: (forceRefresh = false) => {
		return queryOptions({
			queryKey: ["news", "feed", forceRefresh],
			queryFn: () => {
				return newsService.getNewsFeed(forceRefresh)
			},
			staleTime: 10 * 60 * 1000,
		})
	},
	patchNote: (contentPath: string | undefined) => {
		return queryOptions({
			queryKey: ["news", "patch-note", contentPath],
			queryFn: () => {
				if (!contentPath) {
					throw new Error("Missing contentPath for patch note")
				}
				return newsService.getPatchNoteDetails(contentPath)
			},
			enabled: Boolean(contentPath),
			staleTime: Infinity,
		})
	},
}

export type { NewsArticle, NewsFeedResponse, PatchNoteDetails, PatchNoteSummary }
