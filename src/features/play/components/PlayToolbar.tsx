import { Plus, Search, X } from "lucide-react"
import { motion } from "motion/react"
import type { ChangeEvent } from "react"
import type { EngineFilter, GroupByOption, TagFilter } from "@/features/play/types"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { COLOR_TAGS, TAG_ORDER } from "@/shared/stores/useInstancesStore"

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

const GROUP_BY_OPTIONS: SegmentedOption<GroupByOption>[] = [
	{ value: "tag", label: "By Tag" },
	{ value: "engine", label: "By Engine" },
	{ value: "all", label: "Grid" },
]

const ENGINE_OPTIONS: SegmentedOption<EngineFilter>[] = [
	{ value: "all", label: "All Engines" },
	{ value: "Java", label: "Java" },
	{ value: "Horizon (Rust)", label: "Horizon (Rust)" },
	{ value: "Bedrock", label: "Bedrock" },
]

const TAG_OPTIONS: SegmentedOption<TagFilter>[] = [
	{ value: "all", label: "All Tags" },
	...TAG_ORDER.map((tagId) => {
		const meta = COLOR_TAGS[tagId]
		return {
			value: tagId,
			label: meta.name.split(" ")[0] ?? meta.name,
			dotClass: meta.dot,
		}
	}),
]

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
					<input
						type="text"
						value={query}
						onChange={handleQueryInputChange}
						placeholder="Search instances, versions, loaders (Java, Horizon, Bedrock)..."
						className="h-8 w-full rounded-xl border border-border bg-background/80 pr-7 pl-8 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary/50 focus:outline-none"
					/>
					{query && (
						<button
							type="button"
							aria-label="Clear search"
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
					options={GROUP_BY_OPTIONS}
				/>

				<motion.button
					type="button"
					whileTap={{ scale: 0.96 }}
					onClick={onToggleCreate}
					className="flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl border border-primary/30 bg-primary/15 px-3 text-xs font-semibold text-primary transition-colors hover:bg-primary/25"
				>
					<Plus className="h-3.5 w-3.5" />
					<span>New</span>
				</motion.button>
			</div>

			<div className="flex flex-wrap items-center justify-between gap-2 border-t border-border pt-2">
				<SegmentedControl
					layoutId="play-engine-pill"
					value={engine}
					onChange={onEngineChange}
					options={ENGINE_OPTIONS}
					tone="emerald"
				/>

				<SegmentedControl
					layoutId="play-tag-pill"
					value={tag}
					onChange={onTagChange}
					options={TAG_OPTIONS}
				/>
			</div>
		</div>
	)
}

export default PlayToolbar
