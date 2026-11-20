import type { TFunction } from "i18next"
import { create } from "zustand"
import { i18n } from "@/shared/i18n/i18n"

export type InstanceEngine = "Java" | "Horizon (Rust)" | "Bedrock"
export type TagColor = "emerald" | "amber" | "rose" | "violet" | "cyan"

export interface ColorTagMeta {
	id: TagColor
	name: string
	dot: string
	badge: string
	cardBorder: string
	activeRing: string
}

export interface GameInstance {
	id: string
	name: string
	engine: InstanceEngine
	loader: string
	version: string
	colorTag: TagColor
	playtime: string
	lastPlayed: string
	jvmArgs: string
}

export const ENGINE_ORDER: readonly InstanceEngine[] = ["Java", "Horizon (Rust)", "Bedrock"]
export const TAG_ORDER: readonly TagColor[] = ["emerald", "amber", "rose", "violet", "cyan"]

export const COLOR_TAGS: Record<TagColor, ColorTagMeta> = {
	emerald: {
		id: "emerald",
		name: "Survival & Vanilla+",
		dot: "bg-emerald-400",
		badge: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-500/30",
		cardBorder: "hover:border-emerald-500/40",
		activeRing: "border-emerald-500/60 bg-emerald-500/7",
	},
	amber: {
		id: "amber",
		name: "Modded Tech",
		dot: "bg-amber-400",
		badge: "bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-500/30",
		cardBorder: "hover:border-amber-500/40",
		activeRing: "border-amber-500/60 bg-amber-500/7",
	},
	rose: {
		id: "rose",
		name: "PvP & Rust Client",
		dot: "bg-rose-400",
		badge: "bg-rose-500/15 text-rose-700 dark:text-rose-300 border-rose-500/30",
		cardBorder: "hover:border-rose-500/40",
		activeRing: "border-rose-500/60 bg-rose-500/7",
	},
	violet: {
		id: "violet",
		name: "Creative & Shaders",
		dot: "bg-violet-400",
		badge: "bg-violet-500/15 text-violet-700 dark:text-violet-300 border-violet-500/30",
		cardBorder: "hover:border-violet-500/40",
		activeRing: "border-violet-500/60 bg-violet-500/7",
	},
	cyan: {
		id: "cyan",
		name: "Bedrock & Crossplay",
		dot: "bg-cyan-400",
		badge: "bg-cyan-500/15 text-cyan-700 dark:text-cyan-300 border-cyan-500/30",
		cardBorder: "hover:border-cyan-500/40",
		activeRing: "border-cyan-500/60 bg-cyan-500/7",
	},
}

const INITIAL_INSTANCES: GameInstance[] = [
	{
		id: "inst-1",
		name: "1.21.4 Fabric (Sodium)",
		engine: "Java",
		loader: "Fabric",
		version: "1.21.4",
		colorTag: "emerald",
		playtime: "142h",
		lastPlayed: "2h ago",
		jvmArgs: "G1GC • 6GB",
	},
	{
		id: "inst-2",
		name: "Horizon 0.4.2-nightly",
		engine: "Horizon (Rust)",
		loader: "Horizon Native",
		version: "1.21.4-rs",
		colorTag: "rose",
		playtime: "68h",
		lastPlayed: "Today",
		jvmArgs: "Native Rust • 0.8GB",
	},
	{
		id: "inst-3",
		name: "Horizon PvP Low-Latency",
		engine: "Horizon (Rust)",
		loader: "Horizon WGPU",
		version: "1.8.9 / 1.21",
		colorTag: "rose",
		playtime: "215h",
		lastPlayed: "Yesterday",
		jvmArgs: "tokio-rt • 1000 FPS",
	},
	{
		id: "inst-4",
		name: "All the Mods 10 (ATM10)",
		engine: "Java",
		loader: "NeoForge",
		version: "1.21.1",
		colorTag: "amber",
		playtime: "89h",
		lastPlayed: "3d ago",
		jvmArgs: "ZGC • 10GB",
	},
	{
		id: "inst-5",
		name: "GT: New Horizons (GTNH)",
		engine: "Java",
		loader: "Forge",
		version: "1.7.10",
		colorTag: "amber",
		playtime: "612h",
		lastPlayed: "1w ago",
		jvmArgs: "Java 21 • 8GB",
	},
	{
		id: "inst-6",
		name: "Bedrock 1.21.50 RenderDragon",
		engine: "Bedrock",
		loader: "UWP / GDK",
		version: "1.21.50",
		colorTag: "cyan",
		playtime: "34h",
		lastPlayed: "4d ago",
		jvmArgs: "DX12 Native",
	},
	{
		id: "inst-7",
		name: "Create: Perfect Contraptions",
		engine: "Java",
		loader: "NeoForge",
		version: "1.20.4",
		colorTag: "amber",
		playtime: "54h",
		lastPlayed: "5d ago",
		jvmArgs: "G1GC • 8GB",
	},
	{
		id: "inst-8",
		name: "Iris Pathtraced Shaders",
		engine: "Java",
		loader: "Fabric",
		version: "1.21.4",
		colorTag: "violet",
		playtime: "29h",
		lastPlayed: "2d ago",
		jvmArgs: "SEUS PTGI • 8GB",
	},
	{
		id: "inst-9",
		name: "Horizon WGPU Shader Lab",
		engine: "Horizon (Rust)",
		loader: "Horizon Exp",
		version: "0.5.0-dev",
		colorTag: "violet",
		playtime: "19h",
		lastPlayed: "6d ago",
		jvmArgs: "Vulkan / WGPU",
	},
	{
		id: "inst-10",
		name: "Bedrock Preview (RTX Beta)",
		engine: "Bedrock",
		loader: "GDK Preview",
		version: "1.21.60.21",
		colorTag: "cyan",
		playtime: "11h",
		lastPlayed: "2w ago",
		jvmArgs: "DXR Raytracing",
	},
	{
		id: "inst-11",
		name: "Vanilla Hermitcraft Seed",
		engine: "Java",
		loader: "Quilt",
		version: "1.21.4",
		colorTag: "emerald",
		playtime: "77h",
		lastPlayed: "Yesterday",
		jvmArgs: "G1GC • 4GB",
	},
	{
		id: "inst-12",
		name: "RLCraft Hardcore v2.9.3",
		engine: "Java",
		loader: "Forge",
		version: "1.12.2",
		colorTag: "emerald",
		playtime: "46h",
		lastPlayed: "3w ago",
		jvmArgs: "G1GC • 6GB",
	},
]

export interface CreateInstanceInput {
	name: string
	engine: InstanceEngine
	loader: string
	version: string
	colorTag: TagColor
}

export interface InstancesState {
	instances: GameInstance[]
	selectedId: string
	selectInstance: (id: string) => void
	selectOrCreateByName: (input: CreateInstanceInput) => void
	cycleTag: (id: string) => void
	addInstance: (input: CreateInstanceInput) => void
}

export const findInstanceById = (
	instances: GameInstance[],
	instanceId: string,
): GameInstance | undefined => {
	return instances.find((item) => {
		return item.id === instanceId
	})
}

export const selectInstanceById = (instanceId: string) => {
	return (state: InstancesState): GameInstance | undefined => {
		return findInstanceById(state.instances, instanceId)
	}
}

export const selectSelectedInstance = (state: InstancesState): GameInstance | undefined => {
	const foundInstance = findInstanceById(state.instances, state.selectedId)
	return foundInstance ?? state.instances[0]
}

const HOURS_SHORT_REGEX = /^(\d+)h ago$/
const DAYS_SHORT_REGEX = /^(\d+)d ago$/
const WEEKS_SHORT_REGEX = /^(\d+)w ago$/
const HOURS_LONG_REGEX = /^(\d+) hours? ago$/
const DAYS_LONG_REGEX = /^(\d+) days? ago$/
const WEEKS_LONG_REGEX = /^(\d+) weeks? ago$/

export const formatRelativeLastPlayed = (raw: string, t: TFunction): string => {
	if (raw === "Just added") {
		return t("play.lastPlayed.justAdded")
	}
	if (raw === "Today") {
		return t("play.lastPlayed.today")
	}
	if (raw === "Yesterday") {
		return t("play.lastPlayed.yesterday")
	}
	if (raw === "Last week") {
		return t("play.lastPlayed.lastWeek")
	}

	const hoursShort = HOURS_SHORT_REGEX.exec(raw)
	if (hoursShort?.[1]) {
		return t("play.lastPlayed.hoursAgoShort", { count: Number(hoursShort[1]) })
	}

	const daysShort = DAYS_SHORT_REGEX.exec(raw)
	if (daysShort?.[1]) {
		return t("play.lastPlayed.daysAgoShort", { count: Number(daysShort[1]) })
	}

	const weeksShort = WEEKS_SHORT_REGEX.exec(raw)
	if (weeksShort?.[1]) {
		return t("play.lastPlayed.weeksAgoShort", { count: Number(weeksShort[1]) })
	}

	const hoursLong = HOURS_LONG_REGEX.exec(raw)
	if (hoursLong?.[1]) {
		return t("play.lastPlayed.hoursAgoLong", { count: Number(hoursLong[1]) })
	}

	const daysLong = DAYS_LONG_REGEX.exec(raw)
	if (daysLong?.[1]) {
		return t("play.lastPlayed.daysAgoLong", { count: Number(daysLong[1]) })
	}

	if (raw === "1 week ago") {
		return t("play.lastPlayed.weekAgoLong", { count: 1 })
	}

	const weeksLong = WEEKS_LONG_REGEX.exec(raw)
	if (weeksLong?.[1]) {
		return t("play.lastPlayed.weeksAgoLong", { count: Number(weeksLong[1]) })
	}

	return raw
}

const getDefaultJvmArgs = (engine: InstanceEngine): string => {
	if (engine === "Horizon (Rust)") {
		return "Native Rust"
	}
	if (engine === "Bedrock") {
		return "DX12 Native"
	}
	return "G1GC • 6GB"
}

export const useInstancesStore = create<InstancesState>((set) => {
	return {
		instances: INITIAL_INSTANCES,
		selectedId: "inst-1",
		selectInstance: (id) => {
			set({ selectedId: id })
		},
		selectOrCreateByName: (input) => {
			set((state) => {
				const existing = state.instances.find((inst) => {
					return inst.name === input.name
				})
				if (existing) {
					return { selectedId: existing.id }
				}
				const id = `inst-${crypto.randomUUID()}`
				const nextInstance: GameInstance = {
					id,
					name: input.name,
					engine: input.engine,
					loader: input.loader,
					version: input.version,
					colorTag: input.colorTag,
					playtime: "0h",
					lastPlayed: "Just added",
					jvmArgs: getDefaultJvmArgs(input.engine),
				}
				return {
					instances: [nextInstance, ...state.instances],
					selectedId: id,
				}
			})
		},
		cycleTag: (id) => {
			set((state) => {
				const nextInstances = state.instances.map((inst) => {
					if (inst.id !== id) {
						return inst
					}
					const nextIdx = (TAG_ORDER.indexOf(inst.colorTag) + 1) % TAG_ORDER.length
					const nextTag = TAG_ORDER[nextIdx] ?? "emerald"
					return { ...inst, colorTag: nextTag }
				})

				return { instances: nextInstances }
			})
		},
		addInstance: (input) => {
			set((state) => {
				const id = `inst-${crypto.randomUUID()}`
				const trimmedName = input.name.trim()
				const resolvedName =
					trimmedName ||
					i18n.t("play.customInstanceDefaultName", {
						engine: input.engine,
						version: input.version,
					})
				const nextInstance: GameInstance = {
					id,
					name: resolvedName,
					engine: input.engine,
					loader: input.loader,
					version: input.version,
					colorTag: input.colorTag,
					playtime: "0h",
					lastPlayed: "Just added",
					jvmArgs: getDefaultJvmArgs(input.engine),
				}
				return {
					instances: [nextInstance, ...state.instances],
					selectedId: id,
				}
			})
		},
	}
})
