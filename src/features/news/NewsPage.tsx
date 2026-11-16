import { useCallback, useMemo, useState } from "react"
import NewsArticleCard, {
	type NewsArticle,
	type NewsArticleCategory,
} from "@/features/news/components/NewsArticleCard"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"

type NewsCategory = "all" | NewsArticleCategory

const NEWS_FILTERS: SegmentedOption<NewsCategory>[] = [
	{ value: "all", label: "All Updates" },
	{ value: "Release", label: "Releases" },
	{ value: "Snapshot", label: "Snapshots" },
	{ value: "Community", label: "Community" },
]

const NEWS_ARTICLES: NewsArticle[] = [
	{
		id: "news-horizon-094",
		title: "Ingot Horizon 0.9.4: Parallel Chunk Meshing & Metal/Vulkan Parity",
		category: "Release",
		date: "Today",
		versionBadge: "v0.9.4",
		summary:
			"Our Rust-native runtime cuts cold chunk upload latency by 42% and introduces unified frame pacing.",
		details:
			"Horizon 0.9.4 ships lock-free staging buffers on Windows Vulkan, improved entity culling for dense redstone districts, and automatic profile migration from Java resource packs.",
		featured: true,
	},
	{
		id: "news-1215-snapshot",
		title: "Snapshot 25w14a Brings New Ambient Fireflies & Fallen Logs",
		category: "Snapshot",
		date: "2 days ago",
		versionBadge: "25w14a",
		summary:
			"Forest and swamp biomes gain atmospheric particle clusters and mossy fallen timber variants.",
		details:
			"Test worlds created on 25w14a include toggleable Lodestone recipes and subtle leaf-fall particles beneath dense birch canopies.",
	},
	{
		id: "news-modrinth-sync",
		title: "Modrinth Dependency Resolver Now Caches Lockfiles Offline",
		category: "Community",
		date: "5 days ago",
		versionBadge: "Ingot Core",
		summary:
			"Export and share reproducible .ingotpack bundles with deterministic SHA-512 verification.",
		details:
			"Team servers can pin exact transitive mod dependencies so every player joins with zero mismatch errors.",
	},
	{
		id: "news-1214-patch",
		title: "Java Edition 1.21.4 Hotfix Notes: Resin Bricks & Pale Garden Polish",
		category: "Release",
		date: "1 week ago",
		versionBadge: "1.21.4",
		summary:
			"Fixes creaking heart light sensitivity edge cases and smooths winter lighting transitions.",
		details: "Recommended upgrade for all Fabric 0.16.10 and NeoForge 21.4 survival worlds.",
	},
]

const NewsPage = () => {
	const [category, setCategory] = useState<NewsCategory>("all")
	const [expandedId, setExpandedId] = useState<string | null>("news-horizon-094")

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
					<h1 className="text-lg font-semibold tracking-tight text-foreground">News & Events</h1>
					<p className="text-[12px] text-muted-foreground">
						Release notes, Horizon engine milestones, and community highlights.
					</p>
				</div>
				<SegmentedControl
					layoutId="news-category-filter"
					options={NEWS_FILTERS}
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
