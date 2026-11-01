import { openUrl } from "@tauri-apps/plugin-opener"
import { Calendar, ExternalLink, Newspaper, Tag } from "lucide-react"
import { motion } from "motion/react"
import { memo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import type { NewsArticle } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"

export interface NewsArticleCardProps {
	article: NewsArticle
}

const formatDate = (dateString: string): string => {
	try {
		const parsed = new Date(dateString)
		if (Number.isNaN(parsed.getTime())) {
			return dateString
		}
		return new Intl.DateTimeFormat(undefined, {
			year: "numeric",
			month: "short",
			day: "numeric",
		}).format(parsed)
	} catch {
		return dateString
	}
}

const NewsArticleCard = ({ article }: NewsArticleCardProps) => {
	const { t } = useTranslation()
	const formattedDate = formatDate(article.date)

	const handleOpen = useCallback(() => {
		if (article.readMoreUrl) {
			openUrl(article.readMoreUrl).catch((err) => {
				console.error("Failed to open article link:", err)
			})
		}
	}, [article.readMoreUrl])

	return (
		<motion.div
			layout
			initial={{ opacity: 0, y: 12 }}
			animate={{ opacity: 1, y: 0 }}
			transition={{ duration: 0.2 }}
			onClick={handleOpen}
			className="group relative flex cursor-pointer flex-col overflow-hidden rounded-2xl border border-border/50 bg-card/60 transition-all duration-200 hover:border-primary/40 hover:bg-card/90 hover:shadow-lg"
		>
			{article.imageUrl ? (
				<div className="relative aspect-video w-full overflow-hidden bg-muted/40">
					<img
						src={article.imageUrl}
						alt={article.title}
						loading="lazy"
						className="size-full object-cover transition-transform duration-300 group-hover:scale-105"
					/>
					<div className="absolute inset-0 bg-gradient-to-t from-background/90 via-background/20 to-transparent" />
				</div>
			) : (
				<div className="flex aspect-video w-full items-center justify-center bg-muted/30 text-muted-foreground">
					<Newspaper className="size-10 opacity-40" />
				</div>
			)}

			<div className="flex flex-1 flex-col justify-between p-5">
				<div className="flex flex-col gap-2.5">
					<div className="flex items-center justify-between gap-2">
						<div className="flex flex-wrap items-center gap-1.5">
							<Badge variant="secondary" className="text-xs font-medium">
								{article.category}
							</Badge>
							{article.tag && (
								<Badge variant="outline" className="text-[10px] text-muted-foreground">
									<Tag className="mr-1 size-2.5" />
									{article.tag}
								</Badge>
							)}
						</div>
						<span className="flex items-center gap-1 text-xs text-muted-foreground">
							<Calendar className="size-3" />
							{formattedDate}
						</span>
					</div>

					<h2 className="line-clamp-2 text-base font-bold text-foreground transition-colors group-hover:text-primary">
						{article.title}
					</h2>

					{article.text && (
						<p className="line-clamp-3 text-xs leading-relaxed text-muted-foreground">
							{article.text}
						</p>
					)}
				</div>

				<div className="mt-4 flex items-center justify-end border-t border-border/40 pt-3">
					<Button
						variant="ghost"
						size="sm"
						onClick={handleOpen}
						className="h-8 gap-1.5 px-2.5 text-xs text-muted-foreground group-hover:text-primary"
					>
						<span>{t("news.actions.openExternal")}</span>
						<ExternalLink className="size-3" />
					</Button>
				</div>
			</div>
		</motion.div>
	)
}

NewsArticleCard.displayName = "NewsArticleCard"

export default memo(NewsArticleCard)
