import { queryOptions, useQuery } from "@tanstack/react-query"
import { STATIC_STALE_TIME_MS } from "@/shared/api/queryClient"
import { rpc } from "@/shared/api/rpc"

export const newsKeys = {
	all: ["news"] as const,
	feed: (forceRefresh: boolean) => {
		return [...newsKeys.all, "feed", forceRefresh] as const
	},
	article: (articleId: string) => {
		return [...newsKeys.all, "article", articleId] as const
	},
	patchNote: (contentPath: string) => {
		return [...newsKeys.all, "patchNote", contentPath] as const
	},
}

export const newsFeedQueryOptions = (forceRefresh = false) => {
	return queryOptions({
		queryKey: newsKeys.feed(forceRefresh),
		queryFn: () => {
			return rpc.news.get_news_feed(forceRefresh)
		},
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const articleDetailsQueryOptions = (articleId: string) => {
	return queryOptions({
		queryKey: newsKeys.article(articleId),
		queryFn: () => {
			return rpc.news.get_article_details(articleId)
		},
		enabled: articleId.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const patchNoteDetailsQueryOptions = (contentPath: string) => {
	return queryOptions({
		queryKey: newsKeys.patchNote(contentPath),
		queryFn: () => {
			return rpc.news.get_patch_note_details(contentPath)
		},
		enabled: contentPath.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const useNewsFeed = (forceRefresh = false) => {
	return useQuery(newsFeedQueryOptions(forceRefresh))
}

export const useArticleDetails = (articleId: string) => {
	return useQuery(articleDetailsQueryOptions(articleId))
}

export const usePatchNoteDetails = (contentPath: string) => {
	return useQuery(patchNoteDetailsQueryOptions(contentPath))
}
