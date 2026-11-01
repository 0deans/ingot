import { Newspaper, RefreshCw, Search, Sparkles, X } from "lucide-react"
import { memo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupInput,
} from "@/components/ui/input-group"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"

export type NewsFilterCategory = "all" | "releases" | "snapshots" | "blogs"

export interface NewsHeaderProps {
	searchQuery: string
	onSearchChange: (query: string) => void
	activeFilter: NewsFilterCategory
	onFilterChange: (filter: NewsFilterCategory) => void
	isRefreshing: boolean
	onRefresh: () => void
}

const NewsHeader = ({
	searchQuery,
	onSearchChange,
	activeFilter,
	onFilterChange,
	isRefreshing,
	onRefresh,
}: NewsHeaderProps) => {
	const { t } = useTranslation()

	const handleClearSearch = useCallback(() => {
		onSearchChange("")
	}, [onSearchChange])

	const handleFilterChange = useCallback(
		(value: readonly string[]) => {
			if (value.length > 0 && value[0]) {
				onFilterChange(value[0] as NewsFilterCategory)
			}
		},
		[onFilterChange],
	)

	return (
		<div className="flex flex-col gap-4 border-b border-border/40 bg-background/60 p-6 backdrop-blur-md">
			<div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
				<div>
					<h1 className="text-2xl font-bold tracking-tight text-foreground">{t("news.title")}</h1>
					<p className="text-sm text-muted-foreground">{t("news.subtitle")}</p>
				</div>
				<Button
					variant="outline"
					size="sm"
					onClick={onRefresh}
					disabled={isRefreshing}
					className="self-start sm:self-auto"
				>
					<RefreshCw className={`size-3.5 ${isRefreshing ? "animate-spin" : ""}`} />
					<span>{t("news.actions.refresh")}</span>
				</Button>
			</div>

			<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
				<div className="w-full sm:max-w-md">
					<InputGroup>
						<InputGroupAddon>
							<Search className="size-4 text-muted-foreground" />
						</InputGroupAddon>
						<InputGroupInput
							value={searchQuery}
							onChange={(e) => {
								return onSearchChange(e.target.value)
							}}
							placeholder={t("news.searchPlaceholder")}
						/>
						{searchQuery && (
							<InputGroupButton onClick={handleClearSearch} aria-label="Clear search">
								<X className="size-3.5" />
							</InputGroupButton>
						)}
					</InputGroup>
				</div>

				<ToggleGroup
					value={[activeFilter]}
					onValueChange={handleFilterChange}
					variant="outline"
					size="sm"
					className="justify-start overflow-x-auto"
				>
					<ToggleGroupItem value="all" className="gap-1.5 text-xs">
						<Sparkles className="size-3.5" />
						<span>{t("news.filters.all")}</span>
					</ToggleGroupItem>
					<ToggleGroupItem value="releases" className="gap-1.5 text-xs">
						<span className="size-2 rounded-full bg-emerald-500" />
						<span>{t("news.filters.releases")}</span>
					</ToggleGroupItem>
					<ToggleGroupItem value="snapshots" className="gap-1.5 text-xs">
						<span className="size-2 rounded-full bg-amber-500" />
						<span>{t("news.filters.snapshots")}</span>
					</ToggleGroupItem>
					<ToggleGroupItem value="blogs" className="gap-1.5 text-xs">
						<Newspaper className="size-3.5" />
						<span>{t("news.filters.blogs")}</span>
					</ToggleGroupItem>
				</ToggleGroup>
			</div>
		</div>
	)
}

NewsHeader.displayName = "NewsHeader"

export default memo(NewsHeader)
