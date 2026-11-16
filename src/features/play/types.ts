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

export interface InstanceSection {
	key: string
	title: string
	dotClass?: string
	items: GameInstance[]
}
