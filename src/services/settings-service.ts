import { useEffect, useState } from "react"
import { create } from "zustand"
import type {
	MemorySettings,
	SharedSyncStatus,
	SyncConflictInfo,
	SyncReport,
	SyncSettings,
	SystemMemoryInfo,
	WindowSettings,
} from "@/bindings"
import { rpc } from "@/lib/rpc"

export type LauncherBehavior = "keepOpen" | "hideToTray" | "close"

interface SettingsState {
	memory: MemorySettings
	systemMemory: SystemMemoryInfo | null
	behavior: LauncherBehavior
	windowSettings: WindowSettings
	syncSettings: SyncSettings
}

const useSettingsStore = create<SettingsState>(() => ({
	memory: { minRamMb: 2048, maxRamMb: 4096 },
	systemMemory: null,
	behavior: "keepOpen",
	windowSettings: { fullscreen: false, width: 854, height: 480 },
	syncSettings: {
		syncOptions: false,
		syncServers: false,
		syncResourcePacks: false,
		syncCommandHistory: false,
		syncCreativeHotbars: false,
		initializedCategories: [],
	},
}))

const state = () => useSettingsStore.getState()
const set = (patch: Partial<SettingsState>) => useSettingsStore.setState(patch)

let launcherBehaviorPromise: Promise<LauncherBehavior> | null = null
let systemMemoryPromise: Promise<SystemMemoryInfo> | null = null
let memorySettingsPromise: Promise<MemorySettings> | null = null
let windowSettingsPromise: Promise<WindowSettings> | null = null
let syncSettingsPromise: Promise<SyncSettings> | null = null

export const settingsService = {
	async preloadSettings(): Promise<void> {
		await Promise.all([
			this.getLauncherBehavior(),
			this.getMemorySettings(),
			this.getSystemMemory(),
			this.getWindowSettings(),
			this.getSyncSettings(),
		])
	},

	async getLauncherBehavior(): Promise<LauncherBehavior> {
		if (launcherBehaviorPromise) return launcherBehaviorPromise
		launcherBehaviorPromise = (async () => {
			try {
				const b = (await rpc.settings.get_launcher_behavior()) as LauncherBehavior
				set({ behavior: b })
				return b
			} catch (error) {
				console.error("Failed to load launcher behavior:", error)
				return state().behavior
			} finally {
				launcherBehaviorPromise = null
			}
		})()
		return launcherBehaviorPromise
	},

	async setLauncherBehavior(behavior: LauncherBehavior): Promise<LauncherBehavior> {
		try {
			const b = (await rpc.settings.set_launcher_behavior(behavior)) as LauncherBehavior
			set({ behavior: b })
			return b
		} catch (error) {
			console.error("Failed to save launcher behavior:", error)
			throw error
		}
	},

	async getSystemMemory(): Promise<SystemMemoryInfo> {
		if (systemMemoryPromise) return systemMemoryPromise
		systemMemoryPromise = (async () => {
			try {
				const info = await rpc.settings.get_system_memory()
				set({ systemMemory: info })
				return info
			} catch (error) {
				console.error("Failed to query system memory:", error)
				const fallback: SystemMemoryInfo = {
					totalBytes: 16 * 1024 * 1024 * 1024,
					availableBytes: 10 * 1024 * 1024 * 1024,
					usedBytes: 6 * 1024 * 1024 * 1024,
					totalMb: 16384,
					availableMb: 10240,
					usedMb: 6144,
				}
				set({ systemMemory: fallback })
				return fallback
			} finally {
				systemMemoryPromise = null
			}
		})()
		return systemMemoryPromise
	},

	async getMemorySettings(): Promise<MemorySettings> {
		if (memorySettingsPromise) return memorySettingsPromise
		memorySettingsPromise = (async () => {
			try {
				const mem = await rpc.settings.get_memory_settings()
				set({ memory: mem })
				return mem
			} catch (error) {
				console.error("Failed to load memory settings:", error)
				return state().memory
			} finally {
				memorySettingsPromise = null
			}
		})()
		return memorySettingsPromise
	},

	async setMemorySettings(minRamMb: number, maxRamMb: number): Promise<MemorySettings> {
		try {
			const mem = await rpc.settings.set_memory_settings(minRamMb, maxRamMb)
			set({ memory: mem })
			return mem
		} catch (error) {
			console.error("Failed to save memory settings:", error)
			throw error
		}
	},

	async getWindowSettings(): Promise<WindowSettings> {
		if (windowSettingsPromise) return windowSettingsPromise
		windowSettingsPromise = (async () => {
			try {
				const ws = await rpc.settings.get_window_settings()
				set({ windowSettings: ws })
				return ws
			} catch (error) {
				console.error("Failed to load window settings:", error)
				return state().windowSettings
			} finally {
				windowSettingsPromise = null
			}
		})()
		return windowSettingsPromise
	},

	async setWindowSettings(settings: WindowSettings): Promise<WindowSettings> {
		try {
			const ws = await rpc.settings.set_window_settings(settings)
			set({ windowSettings: ws })
			return ws
		} catch (error) {
			console.error("Failed to save window settings:", error)
			throw error
		}
	},

	async getSyncSettings(): Promise<SyncSettings> {
		if (syncSettingsPromise) return syncSettingsPromise
		syncSettingsPromise = (async () => {
			try {
				const ss = await rpc.settings.get_sync_settings()
				set({ syncSettings: ss })
				return ss
			} catch (error) {
				console.error("Failed to load sync settings:", error)
				return state().syncSettings
			} finally {
				syncSettingsPromise = null
			}
		})()
		return syncSettingsPromise
	},

	async setSyncSettings(settings: SyncSettings): Promise<SyncSettings> {
		try {
			const ss = await rpc.settings.set_sync_settings(settings)
			set({ syncSettings: ss })
			return ss
		} catch (error) {
			console.error("Failed to save sync settings:", error)
			throw error
		}
	},

	async pushInstanceSync(instanceId: string): Promise<SyncReport> {
		try {
			return await rpc.push_instance_sync(instanceId)
		} catch (error) {
			console.error("Failed to push instance sync:", error)
			throw error
		}
	},

	async exportInstanceCategory(instanceId: string, category: string): Promise<SyncReport> {
		try {
			return await rpc.export_instance_category_to_shared(instanceId, category)
		} catch (error) {
			console.error("Failed to export instance category:", error)
			throw error
		}
	},

	async pullInstanceSync(instanceId: string): Promise<SyncReport> {
		try {
			return await rpc.pull_instance_sync(instanceId)
		} catch (error) {
			console.error("Failed to pull instance sync:", error)
			throw error
		}
	},

	async getSharedSyncStatus(): Promise<SharedSyncStatus> {
		try {
			return await rpc.get_shared_sync_status()
		} catch (error) {
			console.error("Failed to get shared sync status:", error)
			throw error
		}
	},

	async checkSyncConflict(instanceId: string): Promise<SyncConflictInfo | null> {
		try {
			return await rpc.check_sync_conflict(instanceId)
		} catch (error) {
			console.error("Failed to check sync conflict:", error)
			return null
		}
	},

	async resolveSyncConflict(
		instanceId: string,
		resolution: "use_shared" | "use_instance" | "disable_sync",
	): Promise<SyncReport> {
		try {
			return await rpc.resolve_sync_conflict(instanceId, resolution)
		} catch (error) {
			console.error("Failed to resolve sync conflict:", error)
			throw error
		}
	},
}

export function useMemorySettings() {
	const memory = useSettingsStore((s) => s.memory)
	const systemMemory = useSettingsStore((s) => s.systemMemory)
	const [isLoading, setIsLoading] = useState(!systemMemory)

	useEffect(() => {
		Promise.all([settingsService.getMemorySettings(), settingsService.getSystemMemory()]).then(() =>
			setIsLoading(false),
		)
	}, [])

	return {
		memory,
		systemMemory,
		isLoading,
		setMemorySettings: settingsService.setMemorySettings.bind(settingsService),
		refreshSystemMemory: settingsService.getSystemMemory.bind(settingsService),
	}
}

/** Subscribes to one settings slice and reloads it from the backend on mount. */
function useLoadedSetting<K extends keyof SettingsState>(key: K, load: () => Promise<unknown>) {
	const value = useSettingsStore((s) => s[key])
	const [isLoading, setIsLoading] = useState(true)

	// biome-ignore lint/correctness/useExhaustiveDependencies: load once on mount
	useEffect(() => {
		load().finally(() => setIsLoading(false))
	}, [])

	return [value, isLoading] as const
}

export function useLauncherBehavior() {
	const [behavior, isLoading] = useLoadedSetting("behavior", () =>
		settingsService.getLauncherBehavior(),
	)
	return {
		behavior,
		isLoading,
		setLauncherBehavior: settingsService.setLauncherBehavior.bind(settingsService),
	}
}

export function useWindowSettings() {
	const [windowSettings, isLoading] = useLoadedSetting("windowSettings", () =>
		settingsService.getWindowSettings(),
	)
	return {
		windowSettings,
		isLoading,
		setWindowSettings: settingsService.setWindowSettings.bind(settingsService),
	}
}

export function useSyncSettings() {
	const [syncSettings, isLoading] = useLoadedSetting("syncSettings", () =>
		settingsService.getSyncSettings(),
	)
	return {
		syncSettings,
		isLoading,
		setSyncSettings: settingsService.setSyncSettings.bind(settingsService),
	}
}
