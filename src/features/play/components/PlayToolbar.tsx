import { Plus, Search, X } from "lucide-react"
import { motion } from "motion/react"
import { useMemo, type ChangeEvent } from "react"
import { useTranslation } from "react-i18next"
import {
	TAG_LABEL_KEYS,
	type EngineFilter,
	type GroupByOption,
	type TagFilter,
} from "@/features/play/types"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { COLOR_TAGS, TAG_ORDER } from "@/shared/stores/useInstancesStore"
import { Input } from "@/shared/ui/input"

interface PlayToolbarProps {
	query: string
	engine: EngineFilter
	tag: TagFilter
	groupBy: GroupByOption
	onQueryChange: (q: string) => void
	onEngineChange: (engine: EngineFilter) => void
	onTagChange: (tag: TagFilter) => void
	onGroupByChange: (groupBy: GroupByOption) => void
	onToggleCreate: () => void
}

const PlayToolbar = ({
	query,
	engine,
	tag,
	groupBy,
	onQueryChange,
	onEngineChange,
	onTagChange,
	onGroupByChange,
	onToggleCreate,
}: PlayToolbarProps) => {
	const { t } = useTranslation()

	const groupByOptions: SegmentedOption<GroupByOption>[] = useMemo(() => {
		return [
			{ value: "tag", label: t("play.groupBy.tag") },
			{ value: "engine", label: t("play.groupBy.engine") },
			{ value: "all", label: t("play.groupBy.all") },
		]
	}, [t])

	const engineOptions: SegmentedOption<EngineFilter>[] = useMemo(() => {
		return [
			{ value: "all", label: t("play.engines.all") },
			{ value: "Java", label: t("play.engines.java") },
			{ value: "Horizon (Rust)", label: t("play.engines.horizon") },
			{ value: "Bedrock", label: t("play.engines.bedrock") },
		]
	}, [t])

	const tagOptions: SegmentedOption<TagFilter>[] = useMemo(() => {
		const mappedTags = TAG_ORDER.map((tagId) => {
			const meta = COLOR_TAGS[tagId]
			const fullLabel = t(TAG_LABEL_KEYS[tagId])
			return {
				value: tagId,
				label: fullLabel.split(" ")[0] ?? fullLabel,
				dotClass: meta.dot,
			}
		})

		return [{ value: "all", label: t("play.allTags") }, ...mappedTags]
	}, [t])

	const handleQueryInputChange = (e: ChangeEvent<HTMLInputElement>) => {
		onQueryChange(e.target.value)
	}

	const handleClearQuery = () => {
		onQueryChange("")
	}

	return (
		<div className="space-y-2.5 rounded-2xl border border-border bg-card/65 p-3 backdrop-blur-md">
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="relative min-w-40 flex-1">
					<Search className="pointer-events-none absolute top-1/2 left-3 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
					<Input
						type="text"
						value={query}
						onChange={handleQueryInputChange}
						placeholder={t("play.searchPlaceholder")}
						className="h-8 rounded-xl border-border bg-background/80 pr-7 pl-8 text-xs md:text-xs"
					/>
					{query && (
						<button
							type="button"
							aria-label={t("common.clearSearch")}
							onClick={handleClearQuery}
							className="absolute top-1/2 right-2 -translate-y-1/2 cursor-pointer text-muted-foreground hover:text-foreground"
						>
							<X className="h-3.5 w-3.5" />
						</button>
					)}
				</div>

				<SegmentedControl
					layoutId="play-group-pill"
					value={groupBy}
					onChange={onGroupByChange}
					options={groupByOptions}
				/>

				<motion.button
					type="button"
					whileTap={{ scale: 0.96 }}
					onClick={onToggleCreate}
					className="flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl border border-primary/30 bg-primary/15 px-3 text-xs font-semibold text-primary transition-colors hover:bg-primary/25"
				>
					<Plus className="h-3.5 w-3.5" />
					<span>{t("play.newInstance")}</span>
				</motion.button>
			</div>

			<div className="flex flex-wrap items-center justify-between gap-2 border-t border-border pt-2">
				<SegmentedControl
					layoutId="play-engine-pill"
					value={engine}
					onChange={onEngineChange}
					options={engineOptions}
					tone="emerald"
				/>

				<SegmentedControl
					layoutId="play-tag-pill"
					value={tag}
					onChange={onTagChange}
					options={tagOptions}
				/>
			</div>
		</div>
	)
}

export default PlayToolbar
