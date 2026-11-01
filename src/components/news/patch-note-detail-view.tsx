import { useQuery } from "@tanstack/react-query"
import { openUrl } from "@tauri-apps/plugin-opener"
import { AlertCircle, ArrowLeft, Calendar, ExternalLink, Plus } from "lucide-react"
import { marked } from "marked"
import { memo, useCallback, useMemo } from "react"
import { useTranslation } from "react-i18next"
import type { PatchNoteSummary } from "@/bindings"
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
import { getMinecraftNetArticleUrl, newsQueries } from "@/services/news-service"

export interface PatchNoteDetailViewProps {
	contentPath: string
	summary?: PatchNoteSummary
	onBack: () => void
	onCreateInstance: (version: string) => void
}

const formatDate = (isoString: string): string => {
	try {
		const parsed = new Date(isoString)
		if (Number.isNaN(parsed.getTime())) {
			return isoString
		}
		return new Intl.DateTimeFormat(undefined, {
			year: "numeric",
			month: "long",
			day: "numeric",
		}).format(parsed)
	} catch {
		return isoString
	}
}

const PatchNoteDetailView = ({
	contentPath,
	summary,
	onBack,
	onCreateInstance,
}: PatchNoteDetailViewProps) => {
	const { t } = useTranslation()

	const {
		data: details,
		isLoading,
		isError,
		refetch,
	} = useQuery(newsQueries.patchNote(contentPath))

	const title = details?.title || summary?.title || "Minecraft Update"
	const version = details?.version || summary?.version || ""
	const patchType = details?.patchType || summary?.patchType || "release"
	const isRelease = patchType.toLowerCase() === "release"
	const dateString = details?.date || summary?.date || ""
	const formattedDate = dateString ? formatDate(dateString) : ""
	const imageUrl = details?.imageUrl || summary?.imageUrl

	const officialArticleUrl = useMemo(() => {
		return getMinecraftNetArticleUrl(title, version)
	}, [title, version])

	const renderedBodyHtml = useMemo(() => {
		if (!details?.bodyHtml) {
			return ""
		}
		try {
			const parsed = marked.parse(details.bodyHtml)
			return sanitizeHtml(typeof parsed === "string" ? parsed : details.bodyHtml)
		} catch (e) {
			console.error("Failed to parse patch note markdown/HTML:", e)
			return sanitizeHtml(details.bodyHtml)
		}
	}, [details])

	const handleCreateInstance = useCallback(() => {
		if (version) {
			onCreateInstance(version)
		}
	}, [version, onCreateInstance])

	const handleOpenOriginalSite = useCallback(() => {
		if (officialArticleUrl) {
			openUrl(officialArticleUrl).catch((err) => {
				console.error("Failed to open Minecraft.net link:", err)
			})
		}
	}, [officialArticleUrl])

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

				<div className="flex items-center gap-2">
					<Button
						variant="outline"
						size="sm"
						onClick={handleOpenOriginalSite}
						className="gap-1.5 text-xs text-muted-foreground hover:text-foreground"
					>
						<ExternalLink className="size-3.5" />
						<span>{t("news.actions.openExternal")}</span>
					</Button>

					{version && (
						<Button
							variant="default"
							size="sm"
							onClick={handleCreateInstance}
							className="gap-1.5 font-medium shadow-sm"
						>
							<Plus className="size-3.5" />
							<span>{t("news.actions.createInstanceFull")}</span>
						</Button>
					)}
				</div>
			</div>

			<ScrollArea className="flex-1">
				<div className="mx-auto w-full max-w-4xl px-6 py-8 sm:px-10">
					{isLoading && (
						<div className="flex flex-col gap-6">
							<div className="flex items-center gap-3">
								<Skeleton className="h-6 w-20 rounded-full" />
								<Skeleton className="h-5 w-32" />
							</div>
							<Skeleton className="h-10 w-3/4 rounded-lg" />
							<Skeleton className="aspect-[21/9] max-h-96 w-full rounded-2xl" />
							<div className="mt-4 flex flex-col gap-3">
								<Skeleton className="h-4 w-full" />
								<Skeleton className="h-4 w-5/6" />
								<Skeleton className="h-4 w-4/6" />
								<Skeleton className="mt-4 h-8 w-1/3" />
								<Skeleton className="h-4 w-full" />
								<Skeleton className="h-4 w-3/4" />
							</div>
						</div>
					)}

					{isError && (
						<Empty className="my-16 border border-border/40 bg-card/30">
							<EmptyMedia>
								<AlertCircle className="size-10 text-destructive/80" />
							</EmptyMedia>
							<EmptyHeader>
								<h2 className="text-lg font-semibold">Failed to load patch note details</h2>
							</EmptyHeader>
							<EmptyContent>
								<EmptyDescription>
									Unable to retrieve the full article from Mojang servers. Please check your
									connection and try again.
								</EmptyDescription>
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

					{details && (
						<article className="flex flex-col">
							{/* Header metadata */}
							<div className="flex flex-col gap-3">
								<div className="flex flex-wrap items-center gap-2.5">
									<Badge
										variant="outline"
										className={
											isRelease
												? "border-emerald-500/30 bg-emerald-500/10 font-semibold text-emerald-500"
												: "border-amber-500/30 bg-amber-500/10 font-semibold text-amber-500"
										}
									>
										{isRelease ? t("news.badges.release") : t("news.badges.snapshot")}
									</Badge>
									{formattedDate && (
										<span className="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Calendar className="size-3.5" />
											{formattedDate}
										</span>
									)}
									{version && (
										<Badge variant="secondary" className="font-mono text-xs">
											{version}
										</Badge>
									)}
								</div>

								<h1 className="text-2xl font-black tracking-tight text-foreground sm:text-3xl md:text-4xl">
									{title}
								</h1>
							</div>

							{/* Hero Banner Image */}
							{imageUrl && (
								<div className="relative my-6 aspect-[21/9] max-h-96 w-full overflow-hidden rounded-2xl border border-border/40 bg-muted/40 shadow-sm">
									<img src={imageUrl} alt={title} className="size-full object-cover" />
								</div>
							)}

							{/* Rich Markdown & HTML body content */}
							<div
								onClick={handleHtmlClick}
								dangerouslySetInnerHTML={{ __html: renderedBodyHtml }}
								className="prose prose-invert prose-sm mt-6 max-w-none space-y-4 border-t border-border/40 pt-6 text-sm leading-relaxed text-muted-foreground [&_a]:cursor-pointer [&_a]:text-primary [&_a]:underline hover:[&_a]:text-primary/80 [&_blockquote]:my-4 [&_blockquote]:rounded-r-lg [&_blockquote]:border-l-4 [&_blockquote]:border-primary/70 [&_blockquote]:bg-primary/5 [&_blockquote]:py-1.5 [&_blockquote]:pl-4 [&_code]:rounded [&_code]:bg-muted/80 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-xs [&_code]:text-primary [&_details]:my-3 [&_details]:rounded-lg [&_details]:border [&_details]:border-border/40 [&_details]:bg-card/50 [&_details]:p-3 [&_h1]:mt-8 [&_h1]:mb-4 [&_h1]:border-b [&_h1]:border-border/30 [&_h1]:pb-2 [&_h1]:text-2xl [&_h1]:font-bold [&_h1]:tracking-tight [&_h1]:text-foreground [&_h2]:mt-6 [&_h2]:mb-3 [&_h2]:text-xl [&_h2]:font-bold [&_h2]:tracking-tight [&_h2]:text-foreground [&_h3]:mt-4 [&_h3]:mb-2 [&_h3]:text-lg [&_h3]:font-semibold [&_h3]:text-foreground [&_h4]:mt-3 [&_h4]:mb-1 [&_h4]:text-base [&_h4]:font-semibold [&_h4]:text-foreground [&_img]:my-4 [&_img]:max-w-full [&_img]:rounded-xl [&_img]:border [&_img]:border-border/40 [&_li]:my-1.5 [&_p]:my-3 [&_pre]:my-4 [&_pre]:overflow-x-auto [&_pre]:rounded-xl [&_pre]:bg-card/90 [&_pre]:p-4 [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_summary]:cursor-pointer [&_summary]:font-semibold [&_summary]:text-foreground hover:[&_summary]:text-primary [&_table]:my-4 [&_table]:w-full [&_table]:border-collapse [&_td]:border [&_td]:border-border/60 [&_td]:p-3 [&_th]:border [&_th]:border-border [&_th]:bg-muted/80 [&_th]:p-3 [&_th]:text-left [&_th]:font-semibold"
							/>

							{/* Bottom Action Card */}
							<div className="mt-12 flex flex-col items-center justify-between gap-4 rounded-2xl border border-border/40 bg-card/60 p-6 sm:flex-row">
								<div className="flex flex-col gap-1 text-center sm:text-left">
									<h3 className="text-base font-semibold text-foreground">
										Ready to experience this update?
									</h3>
									<p className="text-xs text-muted-foreground">
										Create a dedicated instance with {version || title} in one click, or read the
										full article on Minecraft.net.
									</p>
								</div>
								<div className="flex flex-wrap items-center gap-2.5">
									<Button
										variant="outline"
										size="sm"
										onClick={handleOpenOriginalSite}
										className="gap-1.5 text-xs"
									>
										<ExternalLink className="size-3.5" />
										<span>{t("news.actions.openExternal")}</span>
									</Button>
									<Button
										variant="default"
										size="sm"
										onClick={handleCreateInstance}
										className="gap-2 font-medium shadow-sm"
									>
										<Plus className="size-4" />
										<span>{t("news.actions.createInstanceFull")}</span>
									</Button>
								</div>
							</div>
						</article>
					)}
				</div>
			</ScrollArea>
		</div>
	)
}

PatchNoteDetailView.displayName = "PatchNoteDetailView"

export default memo(PatchNoteDetailView)
