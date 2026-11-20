import type {
	ColorTagMeta,
	CreateInstanceInput,
	GameInstance,
	InstanceEngine,
	TagColor,
} from "@/shared/stores/useInstancesStore"

export type { ColorTagMeta, CreateInstanceInput, GameInstance, InstanceEngine, TagColor }

export type EngineFilter = "all" | InstanceEngine
export type TagFilter = "all" | TagColor
export type GroupByOption = "tag" | "engine" | "all"

export type EngineLabelKey = "play.engines.java" | "play.engines.horizon" | "play.engines.bedrock"

export type TagLabelKey =
	| "play.tags.emerald"
	| "play.tags.amber"
	| "play.tags.rose"
	| "play.tags.violet"
	| "play.tags.cyan"

export const ENGINE_LABEL_KEYS: Record<InstanceEngine, EngineLabelKey> = {
	Java: "play.engines.java",
	"Horizon (Rust)": "play.engines.horizon",
	Bedrock: "play.engines.bedrock",
}

export const TAG_LABEL_KEYS: Record<TagColor, TagLabelKey> = {
	emerald: "play.tags.emerald",
	amber: "play.tags.amber",
	rose: "play.tags.rose",
	violet: "play.tags.violet",
	cyan: "play.tags.cyan",
}

export interface InstanceSection {
	key: string
	title: string
	dotClass?: string
	items: GameInstance[]
}
