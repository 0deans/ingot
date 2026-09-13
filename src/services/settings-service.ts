import { useEffect, useState } from "react"
import {
	createTauRPCProxy,
	type MemorySettings,
	type SharedSyncStatus,
	type SyncConflictInfo,
	type SyncReport,
	type SyncSettings,
	type SystemMemoryInfo,
	type WindowSettings,
} from "@/bindings"

const rpc = createTauRPCProxy()

let cachedMemory: MemorySettings = {
	minRamMb: 2048,
	maxRamMb: 4096,
}
let cachedSystemMemory: SystemMemoryInfo | null = null
const memoryListeners = new Set<(mem: MemorySettings) => void>()

function notifyMemory(mem: MemorySettings) {
	cachedMemory = mem
	for (const listener of memoryListeners) {
		listener(mem)
	}
}

export type LauncherBehavior = "keepOpen" | "hideToTray" | "close"

let cachedBehavior: LauncherBehavior = "keepOpen"
const behaviorListeners = new Set<(b: LauncherBehavior) => void>()

function notifyBehavior(b: LauncherBehavior) {
	cachedBehavior = b
	for (const listener of behaviorListeners) {
		listener(b)
	}
}

let cachedWindowSettings: WindowSettings = {
	fullscreen: false,
	width: 854,
	height: 480,
}
const windowSettingsListeners = new Set<(ws: WindowSettings) => void>()

function notifyWindowSettings(ws: WindowSettings) {
	cachedWindowSettings = ws
	for (const listener of windowSettingsListeners) {
		listener(ws)
	}
}

let cachedSyncSettings: SyncSettings = {
	syncOptions: false,
	syncServers: false,
	syncResourcePacks: false,
	syncCommandHistory: false,
	syncCreativeHotbars: false,
	initializedCategories: [],
}
const syncSettingsListeners = new Set<(ss: SyncSettings) => void>()

function notifySyncSettings(ss: SyncSettings) {
	cachedSyncSettings = ss
	for (const listener of syncSettingsListeners) {
		listener(ss)
	}
}

export const settingsService = {
	getCachedLauncherBehavior(): LauncherBehavior {
		return cachedBehavior
	},

	subscribeBehavior(listener: (b: LauncherBehavior) => void): () => void {
		behaviorListeners.add(listener)
		listener(cachedBehavior)
		return () => {
			behaviorListeners.delete(listener)
		}
	},

	async getLauncherBehavior(): Promise<LauncherBehavior> {
		try {
			const b = (await rpc.get_launcher_behavior()) as LauncherBehavior
			notifyBehavior(b)
			return b
		} catch (error) {
			console.error("Failed to load launcher behavior:", error)
			return cachedBehavior
		}
	},

	async setLauncherBehavior(behavior: LauncherBehavior): Promise<LauncherBehavior> {
		try {
			const b = (await rpc.set_launcher_behavior(behavior)) as LauncherBehavior
			notifyBehavior(b)
			return b
		} catch (error) {
			console.error("Failed to save launcher behavior:", error)
			throw error
		}
	},

	getCachedMemorySettings(): MemorySettings {
		return cachedMemory
	},

	getCachedSystemMemory(): SystemMemoryInfo | null {
		return cachedSystemMemory
	},

	subscribeMemory(listener: (mem: MemorySettings) => void): () => void {
		memoryListeners.add(listener)
		listener(cachedMemory)
		return () => {
			memoryListeners.delete(listener)
		}
	},

	async getSystemMemory(): Promise<SystemMemoryInfo> {
		try {
			const info = await rpc.get_system_memory()
			cachedSystemMemory = info
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
			cachedSystemMemory = fallback
			return fallback
		}
	},

	async getMemorySettings(): Promise<MemorySettings> {
		try {
			const mem = await rpc.get_memory_settings()
			notifyMemory(mem)
			return mem
		} catch (error) {
			console.error("Failed to load memory settings:", error)
			return cachedMemory
		}
	},

	async setMemorySettings(minRamMb: number, maxRamMb: number): Promise<MemorySettings> {
		try {
			const mem = await rpc.set_memory_settings(minRamMb, maxRamMb)
			notifyMemory(mem)
			return mem
		} catch (error) {
			console.error("Failed to save memory settings:", error)
			throw error
		}
	},

	getCachedWindowSettings(): WindowSettings {
		return cachedWindowSettings
	},

	subscribeWindowSettings(listener: (ws: WindowSettings) => void): () => void {
		windowSettingsListeners.add(listener)
		listener(cachedWindowSettings)
		return () => {
			windowSettingsListeners.delete(listener)
		}
	},

	async getWindowSettings(): Promise<WindowSettings> {
		try {
			const ws = await rpc.get_window_settings()
			notifyWindowSettings(ws)
			return ws
		} catch (error) {
			console.error("Failed to load window settings:", error)
			return cachedWindowSettings
		}
	},

	async setWindowSettings(settings: WindowSettings): Promise<WindowSettings> {
		try {
			const ws = await rpc.set_window_settings(settings)
			notifyWindowSettings(ws)
			return ws
		} catch (error) {
			console.error("Failed to save window settings:", error)
			throw error
		}
	},

	getCachedSyncSettings(): SyncSettings {
		return cachedSyncSettings
	},

	subscribeSyncSettings(listener: (ss: SyncSettings) => void): () => void {
		syncSettingsListeners.add(listener)
		listener(cachedSyncSettings)
		return () => {
			syncSettingsListeners.delete(listener)
		}
	},

	async getSyncSettings(): Promise<SyncSettings> {
		try {
			const ss = await rpc.get_sync_settings()
			notifySyncSettings(ss)
			return ss
		} catch (error) {
			console.error("Failed to load sync settings:", error)
			return cachedSyncSettings
		}
	},

	async setSyncSettings(settings: SyncSettings): Promise<SyncSettings> {
		try {
			const ss = await rpc.set_sync_settings(settings)
			notifySyncSettings(ss)
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
	const [memory, setMemory] = useState<MemorySettings>(settingsService.getCachedMemorySettings())
	const [systemMemory, setSystemMemory] = useState<SystemMemoryInfo | null>(
		settingsService.getCachedSystemMemory(),
	)
	const [isLoading, setIsLoading] = useState(!settingsService.getCachedSystemMemory())

	useEffect(() => {
		const unsubscribe = settingsService.subscribeMemory((updated) => {
			setMemory(updated)
		})

		Promise.all([settingsService.getMemorySettings(), settingsService.getSystemMemory()]).then(
			([, sysInfo]) => {
				setSystemMemory(sysInfo)
				setIsLoading(false)
			},
		)

		return unsubscribe
	}, [])

	return {
		memory,
		systemMemory,
		isLoading,
		setMemorySettings: settingsService.setMemorySettings.bind(settingsService),
		refreshSystemMemory: settingsService.getSystemMemory.bind(settingsService),
	}
}

export function useLauncherBehavior() {
	const [behavior, setBehavior] = useState<LauncherBehavior>(
		settingsService.getCachedLauncherBehavior(),
	)
	const [isLoading, setIsLoading] = useState(true)

	useEffect(() => {
		const unsubscribe = settingsService.subscribeBehavior((updated) => {
			setBehavior(updated)
		})

		settingsService.getLauncherBehavior().finally(() => {
			setIsLoading(false)
		})

		return unsubscribe
	}, [])

	return {
		behavior,
		isLoading,
		setLauncherBehavior: settingsService.setLauncherBehavior.bind(settingsService),
	}
}

export function useWindowSettings() {
	const [windowSettings, setWindowSettingsState] = useState<WindowSettings>(
		settingsService.getCachedWindowSettings(),
	)
	const [isLoading, setIsLoading] = useState(true)

	useEffect(() => {
		const unsubscribe = settingsService.subscribeWindowSettings((updated) => {
			setWindowSettingsState(updated)
		})

		settingsService.getWindowSettings().finally(() => {
			setIsLoading(false)
		})

		return unsubscribe
	}, [])

	return {
		windowSettings,
		isLoading,
		setWindowSettings: settingsService.setWindowSettings.bind(settingsService),
	}
}

export function useSyncSettings() {
	const [syncSettings, setSyncSettingsState] = useState<SyncSettings>(
		settingsService.getCachedSyncSettings(),
	)
	const [isLoading, setIsLoading] = useState(true)

	useEffect(() => {
		const unsubscribe = settingsService.subscribeSyncSettings((updated) => {
			setSyncSettingsState(updated)
		})

		settingsService.getSyncSettings().finally(() => {
			setIsLoading(false)
		})

		return unsubscribe
	}, [])

	return {
		syncSettings,
		isLoading,
		setSyncSettings: settingsService.setSyncSettings.bind(settingsService),
	}
}
