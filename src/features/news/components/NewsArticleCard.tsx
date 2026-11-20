import { Calendar, ChevronDown, ChevronUp, Newspaper, Sparkles } from "lucide-react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

export type NewsArticleCategory = "Release" | "Snapshot" | "Community"

export type NewsArticleId =
	| "news-horizon-094"
	| "news-1215-snapshot"
	| "news-modrinth-sync"
	| "news-1214-patch"

export interface NewsArticle {
	id: NewsArticleId
	titleKey: `news.articles.${NewsArticleId}.title`
	category: NewsArticleCategory
	dateKey: `news.articles.${NewsArticleId}.date`
	versionBadge: string
	summaryKey: `news.articles.${NewsArticleId}.summary`
	detailsKey: `news.articles.${NewsArticleId}.details`
	featured?: boolean
}

type CategoryLabelKey =
	| "news.categories.Release"
	| "news.categories.Snapshot"
	| "news.categories.Community"

const CATEGORY_LABEL_KEYS: Record<NewsArticleCategory, CategoryLabelKey> = {
	Release: "news.categories.Release",
	Snapshot: "news.categories.Snapshot",
	Community: "news.categories.Community",
}

const CATEGORY_BADGE_CLASS: Record<NewsArticleCategory, string> = {
	Release: "bg-primary/15 text-primary border-primary/30",
	Snapshot: "bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-500/30",
	Community: "bg-sky-500/15 text-sky-700 dark:text-sky-300 border-sky-500/30",
}

interface NewsArticleCardProps {
	article: NewsArticle
	isExpanded: boolean
	onToggle: (id: string) => void
}

const NewsArticleCard = ({ article, isExpanded, onToggle }: NewsArticleCardProps) => {
	const { t } = useTranslation()

	const ToggleIcon = isExpanded ? ChevronUp : ChevronDown
	const toggleLabel = isExpanded ? t("news.hide") : t("news.read")

	return (
		<article
			className={cn(
				"rounded-2xl border p-4 transition-colors",
				article.featured
					? "border-primary/30 bg-linear-to-br from-primary/12 via-card/70 to-card/50"
					: "border-border bg-card/65 hover:bg-card",
			)}
		>
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2">
					<span
						className={cn(
							"rounded-full border px-2 py-0.5 text-[10px] font-semibold",
							CATEGORY_BADGE_CLASS[article.category],
						)}
					>
						{t(CATEGORY_LABEL_KEYS[article.category])}
					</span>
					<span className="rounded-md bg-foreground/8 px-1.5 py-0.5 font-mono text-[10px] text-foreground/80">
						{article.versionBadge}
					</span>
					{article.featured ? (
						<span className="flex items-center gap-1 text-[10.5px] font-medium text-primary">
							<Sparkles className="size-3" />
							<span>{t("news.featured")}</span>
						</span>
					) : null}
				</div>
				<span className="flex items-center gap-1 text-[11px] text-muted-foreground">
					<Calendar className="size-3" />
					<span>{t(article.dateKey)}</span>
				</span>
			</div>

			<div className="mt-2.5 flex items-start justify-between gap-3">
				<div>
					<h2 className="text-[14.5px] font-semibold text-foreground">{t(article.titleKey)}</h2>
					<p className="mt-1 text-[12.5px] leading-relaxed text-foreground/80">
						{t(article.summaryKey)}
					</p>
				</div>
				<button
					type="button"
					onClick={() => {
						onToggle(article.id)
					}}
					aria-expanded={isExpanded}
					className="flex h-7 shrink-0 cursor-pointer items-center gap-1 rounded-lg border border-border bg-foreground/6 px-2.5 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/12"
				>
					<Newspaper className="size-3" />
					<span>{toggleLabel}</span>
					<ToggleIcon className="size-3" />
				</button>
			</div>

			{isExpanded ? (
				<div className="mt-3 border-t border-border pt-3 text-[12px] leading-relaxed text-muted-foreground">
					{t(article.detailsKey)}
				</div>
			) : null}
		</article>
	)
}

export default NewsArticleCard
