import { useEffect, useState } from "react"
import { createTauRPCProxy, type MemorySettings, type SystemMemoryInfo } from "@/bindings"

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

export const settingsService = {
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
