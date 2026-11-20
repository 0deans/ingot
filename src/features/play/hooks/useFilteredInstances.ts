import { useMemo } from "react"
import { useTranslation } from "react-i18next"
import {
	ENGINE_LABEL_KEYS,
	TAG_LABEL_KEYS,
	type EngineFilter,
	type GameInstance,
	type GroupByOption,
	type InstanceSection,
	type TagFilter,
} from "@/features/play/types"
import { COLOR_TAGS, ENGINE_ORDER, TAG_ORDER } from "@/shared/stores/useInstancesStore"

export type { EngineFilter, GroupByOption, InstanceSection, TagFilter }

interface FilterOptions {
	instances: GameInstance[]
	query: string
	engine: EngineFilter
	tag: TagFilter
	groupBy: GroupByOption
}

export const useFilteredInstances = ({
	instances,
	query,
	engine,
	tag,
	groupBy,
}: FilterOptions): InstanceSection[] => {
	const { t } = useTranslation()

	const filteredInstances = useMemo(() => {
		const trimmed = query.trim().toLowerCase()
		return instances.filter((inst) => {
			if (engine !== "all" && inst.engine !== engine) {
				return false
			}
			if (tag !== "all" && inst.colorTag !== tag) {
				return false
			}
			if (trimmed === "") {
				return true
			}
			const matchesName = inst.name.toLowerCase().includes(trimmed)
			const matchesVersion = inst.version.toLowerCase().includes(trimmed)
			const matchesLoader = inst.loader.toLowerCase().includes(trimmed)
			return matchesName || matchesVersion || matchesLoader
		})
	}, [engine, instances, query, tag])

	return useMemo((): InstanceSection[] => {
		if (groupBy === "all") {
			return [
				{
					key: "all",
					title: t("play.allInstancesHeading", { count: filteredInstances.length }),
					items: filteredInstances,
				},
			]
		}

		if (groupBy === "engine") {
			const engineSections = ENGINE_ORDER.map((eng) => {
				const items = filteredInstances.filter((inst) => {
					return inst.engine === eng
				})

				return {
					key: eng,
					title: t(ENGINE_LABEL_KEYS[eng]),
					items,
				}
			})

			return engineSections.filter((section) => {
				return section.items.length > 0
			})
		}

		const tagSections = TAG_ORDER.map((tagId) => {
			const meta = COLOR_TAGS[tagId]
			const items = filteredInstances.filter((inst) => {
				return inst.colorTag === tagId
			})

			return {
				key: tagId,
				title: t(TAG_LABEL_KEYS[tagId]),
				dotClass: meta.dot,
				items,
			}
		})

		return tagSections.filter((section) => {
			return section.items.length > 0
		})
	}, [filteredInstances, groupBy, t])
}
