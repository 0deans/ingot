import { useQuery, useQueryClient } from "@tanstack/react-query"
import { createFileRoute, useNavigate } from "@tanstack/react-router"
import { AlertCircle, Newspaper } from "lucide-react"
import { memo, useCallback, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import * as v from "valibot"
import type { ModLoaderType, NewsArticle, PatchNoteSummary } from "@/bindings"
import NewInstanceDialog, {
	type NewInstanceCustomization,
} from "@/components/instances/new-instance-dialog"
import NewsArticleCard from "@/components/news/news-article-card"
import NewsArticleDetailView from "@/components/news/news-article-detail-view"
import NewsHeader, { type NewsFilterCategory } from "@/components/news/news-header"
import NewsSkeletonGrid from "@/components/news/news-skeleton"
import PatchNoteCard from "@/components/news/patch-note-card"
import PatchNoteDetailView from "@/components/news/patch-note-detail-view"
import { Button } from "@/components/ui/button"
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
} from "@/components/ui/empty"
import { ScrollArea } from "@/components/ui/scroll-area"
import { instanceService } from "@/services/instance-service"
import { newsQueries, newsService } from "@/services/news-service"

export const newsFilterSchema = v.picklist(["all", "releases", "snapshots", "blogs"])

export const newsSearchSchema = v.object({
	query: v.optional(v.fallback(v.string(), ""), ""),
	filter: v.optional(v.fallback(newsFilterSchema, "all"), "all"),
	details: v.optional(v.string()),
	articleId: v.optional(v.string()),
})

export type NewsSearchParams = v.InferOutput<typeof newsSearchSchema>

const NewsPage = () => {
	const { t } = useTranslation()
	const navigate = useNavigate({ from: Route.fullPath })
	const search = Route.useSearch()
	const queryClient = useQueryClient()

	const mainViewportRef = useRef<HTMLDivElement>(null)

	const [isCreatingInstance, setIsCreatingInstance] = useState(false)
	const [createVersion, setCreateVersion] = useState<string | undefined>()
	const [selectedNote, setSelectedNote] = useState<PatchNoteSummary | null>(null)
	const [selectedArticle, setSelectedArticle] = useState<NewsArticle | null>(null)
	const [isManualRefreshing, setIsManualRefreshing] = useState(false)

	const { data: feed, isLoading, isError, refetch } = useQuery(newsQueries.feed(false))

	const activeFilter = search.filter as NewsFilterCategory
	const searchQuery = search.query

	const filteredPatchNotes = useMemo(() => {
		if (!feed?.patchNotes) {
			return []
		}
		let notes: PatchNoteSummary[] = feed.patchNotes
		if (activeFilter === "releases") {
			notes = notes.filter((n: PatchNoteSummary) => {
				return n.patchType.toLowerCase() === "release"
			})
		} else if (activeFilter === "snapshots") {
			notes = notes.filter((n: PatchNoteSummary) => {
				return n.patchType.toLowerCase() === "snapshot"
			})
		} else if (activeFilter === "blogs") {
			return []
		}

		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase().trim()
			notes = notes.filter((n: PatchNoteSummary) => {
				return (
					n.title.toLowerCase().includes(query) ||
					n.version.toLowerCase().includes(query) ||
					n.shortText.toLowerCase().includes(query)
				)
			})
		}

		return notes
	}, [feed, activeFilter, searchQuery])

	const filteredArticles = useMemo(() => {
		if (!feed?.articles) {
			return []
		}
		if (activeFilter === "releases" || activeFilter === "snapshots") {
			return []
		}
		let articles = feed.articles
		if (searchQuery.trim()) {
			const query = searchQuery.toLowerCase().trim()
			articles = articles.filter((a) => {
				return (
					a.title.toLowerCase().includes(query) ||
					a.category.toLowerCase().includes(query) ||
					(a.tag && a.tag.toLowerCase().includes(query)) ||
					a.text.toLowerCase().includes(query)
				)
			})
		}
		return articles
	}, [feed, activeFilter, searchQuery])

	const hasItems = filteredPatchNotes.length > 0 || filteredArticles.length > 0

	const handleSearchChange = useCallback(
		(query: string) => {
			navigate({
				search: (prev) => {
					return { ...prev, query: query || undefined }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleFilterChange = useCallback(
		(filter: NewsFilterCategory) => {
			navigate({
				search: (prev) => {
					return { ...prev, filter }
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleRefresh = useCallback(async () => {
		try {
			setIsManualRefreshing(true)
			await queryClient.fetchQuery({
				queryKey: ["news", "feed", true],
				queryFn: () => {
					return newsService.getNewsFeed(true)
				},
			})
			await refetch()
			toast.success("News feed updated")
		} catch (err) {
			console.error("Failed to refresh feed:", err)
			toast.error("Failed to update news feed")
		} finally {
			setIsManualRefreshing(false)
		}
	}, [queryClient, refetch])

	const handleReadNotes = useCallback(
		(note: PatchNoteSummary) => {
			setSelectedNote(note)
			navigate({
				search: (prev) => {
					return { ...prev, details: note.contentPath }
				},
			})
		},
		[navigate],
	)

	const handleBackFromDetails = useCallback(() => {
		setSelectedNote(null)
		navigate({
			search: (prev) => {
				return { ...prev, details: undefined }
			},
		})
	}, [navigate])

	const handleReadArticle = useCallback(
		(article: NewsArticle) => {
			setSelectedArticle(article)
			navigate({
				search: (prev) => {
					return { ...prev, articleId: article.id }
				},
			})
		},
		[navigate],
	)

	const handleBackFromArticle = useCallback(() => {
		setSelectedArticle(null)
		navigate({
			search: (prev) => {
				return { ...prev, articleId: undefined }
			},
		})
	}, [navigate])

	const handleCreateInstance = useCallback((version: string) => {
		setCreateVersion(version)
		setIsCreatingInstance(true)
	}, [])

	const handleConfirmCreateInstance = useCallback(
		async (
			name: string,
			gameVersion: string,
			loader: ModLoaderType,
			loaderVersion: string | null,
			customization?: NewInstanceCustomization,
		) => {
			try {
				await instanceService.createInstance(
					name,
					gameVersion,
					loader,
					loaderVersion,
					customization,
				)
				toast.success(`Instance "${name}" created successfully`)
				setIsCreatingInstance(false)
				navigate({ to: "/" })
			} catch (err) {
				console.error("Failed to create instance:", err)
				toast.error("Failed to create instance")
			}
		},
		[navigate],
	)

	if (search.details) {
		const matchedNote =
			selectedNote ??
			feed?.patchNotes.find((n: PatchNoteSummary) => {
				return n.contentPath === search.details
			})

		return (
			<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
				<PatchNoteDetailView
					contentPath={search.details}
					summary={matchedNote}
					onBack={handleBackFromDetails}
					onCreateInstance={handleCreateInstance}
				/>

				<NewInstanceDialog
					open={isCreatingInstance}
					onOpenChange={setIsCreatingInstance}
					onCreateInstance={handleConfirmCreateInstance}
					initialVersion={createVersion}
				/>
			</div>
		)
	}

	if (search.articleId) {
		const matchedArticle =
			selectedArticle ??
			feed?.articles.find((a: NewsArticle) => {
				return a.id === search.articleId
			})

		return (
			<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
				<NewsArticleDetailView
					articleId={search.articleId}
					initialArticle={matchedArticle}
					onBack={handleBackFromArticle}
				/>
			</div>
		)
	}

	return (
		<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
			<NewsHeader
				searchQuery={searchQuery}
				onSearchChange={handleSearchChange}
				activeFilter={activeFilter}
				onFilterChange={handleFilterChange}
				isRefreshing={isManualRefreshing}
				onRefresh={handleRefresh}
			/>

			<ScrollArea ref={mainViewportRef} className="flex-1">
				<div className="mx-auto w-full max-w-7xl p-6">
					{isLoading && <NewsSkeletonGrid count={6} />}

					{isError && !feed && (
						<Empty className="my-16 border border-border/40 bg-card/30">
							<EmptyMedia>
								<AlertCircle className="size-10 text-destructive/80" />
							</EmptyMedia>
							<EmptyHeader>
								<h2 className="text-lg font-semibold">{t("news.empty.offlineTitle")}</h2>
							</EmptyHeader>
							<EmptyContent>
								<EmptyDescription>{t("news.empty.offlineDescription")}</EmptyDescription>
								<Button
									variant="outline"
									size="sm"
									className="mt-4"
									onClick={() => {
										return refetch()
									}}
								>
									{t("common.tryAgain")}
								</Button>
							</EmptyContent>
						</Empty>
					)}

					{!isLoading && !hasItems && (
						<Empty className="my-16 border border-border/40 bg-card/30">
							<EmptyMedia>
								<Newspaper className="size-10 text-muted-foreground/60" />
							</EmptyMedia>
							<EmptyHeader>
								<h2 className="text-lg font-semibold">{t("news.empty.title")}</h2>
							</EmptyHeader>
							<EmptyContent>
								<EmptyDescription>{t("news.empty.description")}</EmptyDescription>
							</EmptyContent>
						</Empty>
					)}

					{!isLoading && hasItems && (
						<div className="flex flex-col gap-8">
							{filteredPatchNotes.length > 0 && (
								<section className="flex flex-col gap-4">
									<div className="flex items-center justify-between">
										<h2 className="text-lg font-bold tracking-tight text-foreground">
											{t("news.sections.patchNotes")}
										</h2>
										<span className="text-xs text-muted-foreground">
											{t("news.sections.patchNotesCount", {
												count: filteredPatchNotes.length,
											})}
										</span>
									</div>
									<div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
										{filteredPatchNotes.map((note: PatchNoteSummary) => {
											return (
												<PatchNoteCard
													key={note.id}
													note={note}
													onReadNotes={handleReadNotes}
													onCreateInstance={handleCreateInstance}
												/>
											)
										})}
									</div>
								</section>
							)}

							{filteredArticles.length > 0 && (
								<section className="flex flex-col gap-4">
									<div className="flex items-center justify-between">
										<h2 className="text-lg font-bold tracking-tight text-foreground">
											{t("news.sections.articles")}
										</h2>
										<span className="text-xs text-muted-foreground">
											{t("news.sections.articlesCount", { count: filteredArticles.length })}
										</span>
									</div>
									<div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
										{filteredArticles.map((article) => {
											return (
												<NewsArticleCard
													key={article.id}
													article={article}
													onReadArticle={handleReadArticle}
												/>
											)
										})}
									</div>
								</section>
							)}
						</div>
					)}
				</div>
			</ScrollArea>

			<NewInstanceDialog
				open={isCreatingInstance}
				onOpenChange={setIsCreatingInstance}
				onCreateInstance={handleConfirmCreateInstance}
				initialVersion={createVersion}
			/>
		</div>
	)
}

NewsPage.displayName = "NewsPage"

const MemoizedNewsPage = memo(NewsPage)

export const Route = createFileRoute("/news")({
	validateSearch: newsSearchSchema,
	loader: async ({ context }) => {
		await context.queryClient.ensureQueryData(newsQueries.feed(false))
	},
	component: MemoizedNewsPage,
})
