import { useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import NewsArticleCard, {
	type NewsArticle,
	type NewsArticleCategory,
} from "@/features/news/components/NewsArticleCard"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"

type NewsCategory = "all" | NewsArticleCategory

const NEWS_ARTICLES: NewsArticle[] = [
	{
		id: "news-horizon-094",
		titleKey: "news.articles.news-horizon-094.title",
		category: "Release",
		dateKey: "news.articles.news-horizon-094.date",
		versionBadge: "v0.9.4",
		summaryKey: "news.articles.news-horizon-094.summary",
		detailsKey: "news.articles.news-horizon-094.details",
		featured: true,
	},
	{
		id: "news-1215-snapshot",
		titleKey: "news.articles.news-1215-snapshot.title",
		category: "Snapshot",
		dateKey: "news.articles.news-1215-snapshot.date",
		versionBadge: "25w14a",
		summaryKey: "news.articles.news-1215-snapshot.summary",
		detailsKey: "news.articles.news-1215-snapshot.details",
	},
	{
		id: "news-modrinth-sync",
		titleKey: "news.articles.news-modrinth-sync.title",
		category: "Community",
		dateKey: "news.articles.news-modrinth-sync.date",
		versionBadge: "Ingot Core",
		summaryKey: "news.articles.news-modrinth-sync.summary",
		detailsKey: "news.articles.news-modrinth-sync.details",
	},
	{
		id: "news-1214-patch",
		titleKey: "news.articles.news-1214-patch.title",
		category: "Release",
		dateKey: "news.articles.news-1214-patch.date",
		versionBadge: "1.21.4",
		summaryKey: "news.articles.news-1214-patch.summary",
		detailsKey: "news.articles.news-1214-patch.details",
	},
]

const NewsPage = () => {
	const { t } = useTranslation()

	const [category, setCategory] = useState<NewsCategory>("all")
	const [expandedId, setExpandedId] = useState<string | null>("news-horizon-094")

	const newsFilters: SegmentedOption<NewsCategory>[] = useMemo(() => {
		return [
			{ value: "all", label: t("news.filters.all") },
			{ value: "Release", label: t("news.filters.releases") },
			{ value: "Snapshot", label: t("news.filters.snapshots") },
			{ value: "Community", label: t("news.filters.community") },
		]
	}, [t])

	const visibleArticles = useMemo(() => {
		if (category === "all") {
			return NEWS_ARTICLES
		}
		return NEWS_ARTICLES.filter((article) => {
			return article.category === category
		})
	}, [category])

	const handleToggle = useCallback((id: string) => {
		setExpandedId((prev) => {
			if (prev === id) {
				return null
			}

			return id
		})
	}, [])

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">
						{t("news.title")}
					</h1>
					<p className="text-[12px] text-muted-foreground">{t("news.subtitle")}</p>
				</div>
				<SegmentedControl
					layoutId="news-category-filter"
					options={newsFilters}
					value={category}
					onChange={setCategory}
					tone="emerald"
				/>
			</div>

			<div className="flex flex-col gap-3">
				{visibleArticles.map((article) => {
					return (
						<NewsArticleCard
							key={article.id}
							article={article}
							isExpanded={expandedId === article.id}
							onToggle={handleToggle}
						/>
					)
				})}
			</div>
		</div>
	)
}

export default NewsPage
