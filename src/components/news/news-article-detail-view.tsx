import { useQuery } from "@tanstack/react-query"
import { openUrl } from "@tauri-apps/plugin-opener"
import { AlertCircle, ArrowLeft, Calendar, ExternalLink, Newspaper, Tag } from "lucide-react"
import { memo, useCallback, useMemo } from "react"
import { useTranslation } from "react-i18next"
import type { NewsArticle } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
} from "@/components/ui/empty"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Skeleton } from "@/components/ui/skeleton"
import { sanitizeHtml } from "@/lib/sanitize-html"
import { newsQueries } from "@/services/news-service"

export interface NewsArticleDetailViewProps {
	articleId: string
	initialArticle?: NewsArticle
	onBack: () => void
}

const formatDate = (dateString: string): string => {
	try {
		const parsed = new Date(dateString)
		if (Number.isNaN(parsed.getTime())) {
			return dateString
		}
		return new Intl.DateTimeFormat(undefined, {
			year: "numeric",
			month: "long",
			day: "numeric",
		}).format(parsed)
	} catch {
		return dateString
	}
}

const NewsArticleDetailView = ({
	articleId,
	initialArticle,
	onBack,
}: NewsArticleDetailViewProps) => {
	const { t } = useTranslation()

	const { data: details, isLoading, isError, refetch } = useQuery(newsQueries.article(articleId))

	const title = details?.title || initialArticle?.title || "Minecraft Article"
	const category = details?.category || initialArticle?.category || "Minecraft"
	const tag = details?.tag || initialArticle?.tag
	const dateString = details?.date || initialArticle?.date || ""
	const formattedDate = dateString ? formatDate(dateString) : ""
	const imageUrl = details?.imageUrl || initialArticle?.imageUrl
	const summaryText = details?.text || initialArticle?.text || ""
	const readMoreUrl = details?.readMoreUrl || initialArticle?.readMoreUrl || ""
	const bodyHtml = details?.bodyHtml

	const renderedBodyHtml = useMemo(() => {
		if (!bodyHtml) {
			return ""
		}
		return sanitizeHtml(bodyHtml)
	}, [bodyHtml])

	const handleOpenInBrowser = useCallback(() => {
		if (readMoreUrl) {
			openUrl(readMoreUrl).catch((err) => {
				console.error("Failed to open article link in browser:", err)
			})
		}
	}, [readMoreUrl])

	const handleHtmlClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
		const target = e.target as HTMLElement
		const anchor = target.closest("a")
		if (anchor && anchor.href) {
			e.preventDefault()
			e.stopPropagation()
			openUrl(anchor.href).catch((err) => {
				console.error("Failed to open link:", err)
			})
		}
	}, [])

	return (
		<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
			{/* Top sticky navigation bar */}
			<div className="flex items-center justify-between border-b border-border/40 bg-background/80 px-6 py-3.5 backdrop-blur-md">
				<Button
					variant="ghost"
					size="sm"
					onClick={onBack}
					className="gap-2 px-2.5 text-xs font-medium text-muted-foreground hover:text-foreground"
				>
					<ArrowLeft className="size-4" />
					<span>{t("news.actions.back")}</span>
				</Button>

				{readMoreUrl && (
					<Button
						variant="outline"
						size="sm"
						onClick={handleOpenInBrowser}
						className="gap-1.5 text-xs text-muted-foreground hover:text-foreground"
					>
						<ExternalLink className="size-3.5" />
						<span>{t("news.actions.openInBrowser")}</span>
					</Button>
				)}
			</div>

			<ScrollArea className="flex-1">
				<div className="mx-auto w-full max-w-4xl px-6 py-8 sm:px-10">
					{isLoading && !initialArticle && (
						<div className="flex flex-col gap-6">
							<div className="flex items-center gap-3">
								<Skeleton className="h-6 w-24 rounded-full" />
								<Skeleton className="h-5 w-32" />
							</div>
							<Skeleton className="h-10 w-3/4 rounded-lg" />
							<Skeleton className="aspect-[21/9] max-h-96 w-full rounded-2xl" />
							<div className="mt-4 flex flex-col gap-3">
								<Skeleton className="h-4 w-full" />
								<Skeleton className="h-4 w-5/6" />
								<Skeleton className="h-4 w-4/6" />
							</div>
						</div>
					)}

					{isError && !initialArticle && (
						<Empty className="my-16 border border-border/40 bg-card/30">
							<EmptyMedia>
								<AlertCircle className="size-10 text-destructive/80" />
							</EmptyMedia>
							<EmptyHeader>
								<h2 className="text-lg font-semibold">{t("news.detail.loadError")}</h2>
							</EmptyHeader>
							<EmptyContent>
								<EmptyDescription>{t("news.detail.loadErrorDescription")}</EmptyDescription>
								<div className="mt-4 flex items-center justify-center gap-3">
									<Button
										variant="outline"
										size="sm"
										onClick={() => {
											return refetch()
										}}
									>
										{t("common.tryAgain")}
									</Button>
									<Button variant="ghost" size="sm" onClick={onBack}>
										{t("news.actions.back")}
									</Button>
								</div>
							</EmptyContent>
						</Empty>
					)}

					{(details || initialArticle) && (
						<article className="flex flex-col">
							{/* Header metadata */}
							<div className="flex flex-col gap-3">
								<div className="flex flex-wrap items-center gap-2.5">
									<Badge variant="secondary" className="font-semibold">
										{category}
									</Badge>
									{tag && (
										<Badge variant="outline" className="text-xs text-muted-foreground">
											<Tag className="mr-1 size-3" />
											{tag}
										</Badge>
									)}
									{formattedDate && (
										<span className="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Calendar className="size-3.5" />
											{formattedDate}
										</span>
									)}
								</div>

								<h1 className="text-2xl font-black tracking-tight text-foreground sm:text-3xl md:text-4xl">
									{title}
								</h1>
							</div>

							{/* Hero Banner Image */}
							{imageUrl ? (
								<div className="relative my-6 aspect-[21/9] max-h-96 w-full overflow-hidden rounded-2xl border border-border/40 bg-muted/40 shadow-sm">
									<img src={imageUrl} alt={title} className="size-full object-cover" />
								</div>
							) : (
								<div className="relative my-6 flex aspect-[21/9] max-h-96 w-full items-center justify-center rounded-2xl border border-border/40 bg-muted/20">
									<Newspaper className="size-16 text-muted-foreground/30" />
								</div>
							)}

							{/* Summary / Lead paragraph */}
							{summaryText && (
								<div className="rounded-xl border border-primary/20 bg-primary/5 p-4 text-sm leading-relaxed font-medium text-foreground">
									{summaryText}
								</div>
							)}

							{/* Full rich article body */}
							{renderedBodyHtml ? (
								<div
									onClick={handleHtmlClick}
									dangerouslySetInnerHTML={{ __html: renderedBodyHtml }}
									className="news-prose mt-6 max-w-none border-t border-border/40 pt-6"
								/>
							) : isLoading ? (
								<div className="mt-8 flex flex-col gap-3">
									<Skeleton className="h-4 w-full" />
									<Skeleton className="h-4 w-5/6" />
									<Skeleton className="h-4 w-4/6" />
									<Skeleton className="mt-4 h-8 w-1/3" />
									<Skeleton className="h-4 w-full" />
									<Skeleton className="h-4 w-3/4" />
								</div>
							) : null}

							{/* Bottom Action Card */}
							{readMoreUrl && (
								<div className="mt-12 flex flex-col items-center justify-between gap-4 rounded-2xl border border-border/40 bg-card/60 p-6 sm:flex-row">
									<div className="flex flex-col gap-1 text-center sm:text-left">
										<h3 className="text-base font-semibold text-foreground">
											{t("news.detail.enjoyedPrompt")}
										</h3>
										<p className="text-xs text-muted-foreground">
											{t("news.detail.enjoyedDescription")}
										</p>
									</div>
									<Button
										variant="outline"
										size="sm"
										onClick={handleOpenInBrowser}
										className="shrink-0 gap-1.5 text-xs font-medium"
									>
										<ExternalLink className="size-3.5" />
										<span>{t("news.actions.openExternal")}</span>
									</Button>
								</div>
							)}
						</article>
					)}
				</div>
			</ScrollArea>
		</div>
	)
}

NewsArticleDetailView.displayName = "NewsArticleDetailView"

export default memo(NewsArticleDetailView)
