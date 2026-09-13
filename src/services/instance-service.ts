import { useEffect, useState } from "react"
import {
	createTauRPCProxy,
	type InstanceConfig,
	type InstanceStatusEvent,
	type LaunchProgressEvent,
	type ModLoaderType,
	type RunningInstanceSummary,
	type VersionManifestEntry,
} from "@/bindings"

export const rpc = createTauRPCProxy()

let cachedInstances: InstanceConfig[] = []
let cachedRunning: Map<string, RunningInstanceSummary> = new Map()
const progressMap: Map<string, LaunchProgressEvent> = new Map()

const instanceListeners = new Set<(instances: InstanceConfig[]) => void>()
const runningListeners = new Set<(running: Map<string, RunningInstanceSummary>) => void>()
const progressListeners = new Set<(progress: Map<string, LaunchProgressEvent>) => void>()

let isInitialized = false

async function initListeners() {
	if (isInitialized) return
	isInitialized = true

	try {
		const running = await rpc.get_running_instances()
		cachedRunning = new Map(running.map((r) => [r.instanceId, r]))
		notifyRunning()
	} catch (e) {
		console.error("Failed to load initial running instances:", e)
	}

	try {
		await rpc.on_instance_status_changed.on((event: InstanceStatusEvent) => {
			if (!event?.instanceId) return
			if (event.isRunning) {
				cachedRunning.set(event.instanceId, {
					instanceId: event.instanceId,
					pid: event.pid,
					startedAt: event.startedAt,
				})
			} else {
				cachedRunning.delete(event.instanceId)
			}
			progressMap.delete(event.instanceId)
			notifyProgress()
			notifyRunning()
			instanceService.refreshInstances()
		})
	} catch (e) {
		console.error("Failed to setup on_instance_status_changed listener:", e)
	}

	try {
		await rpc.on_launch_progress.on((event: LaunchProgressEvent) => {
			if (!event?.instanceId) return
			progressMap.set(event.instanceId, event)
			notifyProgress()
		})
	} catch (e) {
		console.error("Failed to setup on_launch_progress listener:", e)
	}
}

function notifyInstances() {
	for (const listener of instanceListeners) {
		listener([...cachedInstances])
	}
}

function notifyRunning() {
	const copy = new Map(cachedRunning)
	for (const listener of runningListeners) {
		listener(copy)
	}
}

function notifyProgress() {
	const copy = new Map(progressMap)
	for (const listener of progressListeners) {
		listener(copy)
	}
}

export const instanceService = {
	async getInstances(): Promise<InstanceConfig[]> {
		return this.refreshInstances()
	},

	async refreshInstances(): Promise<InstanceConfig[]> {
		try {
			await initListeners()
			const instances = await rpc.get_instances()
			cachedInstances = instances
			notifyInstances()
			return instances
		} catch (e) {
			console.error("Failed to fetch instances:", e)
			return cachedInstances
		}
	},

	async createInstance(
		name: string,
		gameVersion: string,
		loader: ModLoaderType,
		loaderVersion: string | null = null,
	): Promise<InstanceConfig> {
		const created = await rpc.create_instance(name, gameVersion, loader, loaderVersion)
		await this.refreshInstances()
		return created
	},

	async deleteInstance(instanceId: string): Promise<void> {
		cachedInstances = cachedInstances.filter((i) => i.id !== instanceId)
		notifyInstances()
		try {
			await rpc.delete_instance(instanceId)
		} finally {
			await this.refreshInstances()
		}
	},

	async updateInstance(instance: InstanceConfig): Promise<void> {
		await rpc.update_instance(instance)
		await this.refreshInstances()
	},

	async openInstanceFolder(instanceId: string): Promise<void> {
		await rpc.open_instance_folder(instanceId)
	},

	async launchInstance(instanceId: string): Promise<number> {
		progressMap.set(instanceId, {
			instanceId,
			phase: "Preparing launch",
			currentStep: 0,
			totalSteps: 10,
			percentage: 0,
			detail: "Initializing launch sequence...",
		})
		notifyProgress()
		try {
			const pid = await rpc.launch_instance(instanceId)
			return pid
		} catch (e) {
			progressMap.delete(instanceId)
			notifyProgress()
			throw e
		}
	},

	async killInstance(instanceId: string): Promise<void> {
		await rpc.kill_instance(instanceId)
		cachedRunning.delete(instanceId)
		progressMap.delete(instanceId)
		notifyRunning()
		notifyProgress()
	},

	async getAvailableGameVersions(): Promise<VersionManifestEntry[]> {
		return rpc.get_available_game_versions()
	},

	async getAvailableLoaderVersions(gameVersion: string, loader: ModLoaderType): Promise<string[]> {
		return rpc.get_available_loader_versions(gameVersion, loader)
	},

	subscribeInstances(listener: (instances: InstanceConfig[]) => void): () => void {
		instanceListeners.add(listener)
		listener([...cachedInstances])
		return () => {
			instanceListeners.delete(listener)
		}
	},

	subscribeRunning(listener: (running: Map<string, RunningInstanceSummary>) => void): () => void {
		runningListeners.add(listener)
		listener(new Map(cachedRunning))
		return () => {
			runningListeners.delete(listener)
		}
	},

	subscribeProgress(listener: (progress: Map<string, LaunchProgressEvent>) => void): () => void {
		progressListeners.add(listener)
		listener(new Map(progressMap))
		return () => {
			progressListeners.delete(listener)
		}
	},
}

export function useInstances() {
	const [instances, setInstances] = useState<InstanceConfig[]>(cachedInstances)
	const [isLoading, setIsLoading] = useState<boolean>(cachedInstances.length === 0)

	useEffect(() => {
		const unsub = instanceService.subscribeInstances((insts) => {
			setInstances(insts)
		})
		instanceService.refreshInstances().finally(() => {
			setIsLoading(false)
		})
		return unsub
	}, [])

	return {
		instances,
		isLoading,
		refresh: () => instanceService.refreshInstances(),
	}
}

export function useRunningInstances() {
	const [running, setRunning] = useState<Map<string, RunningInstanceSummary>>(
		new Map(cachedRunning),
	)

	useEffect(() => {
		initListeners()
		return instanceService.subscribeRunning((r) => {
			setRunning(r)
		})
	}, [])

	return {
		runningMap: running,
		runningList: Array.from(running.values()),
		isRunning: (instanceId: string) => running.has(instanceId),
		getRunningInfo: (instanceId: string) => running.get(instanceId),
	}
}

export function useAllInstancesProgress(): Map<string, LaunchProgressEvent> {
	const [progress, setProgress] = useState<Map<string, LaunchProgressEvent>>(new Map(progressMap))

	useEffect(() => {
		return instanceService.subscribeProgress((map) => {
			setProgress(map)
		})
	}, [])

	return progress
}

export function useInstanceProgress(instanceId: string | undefined) {
	const [progress, setProgress] = useState<LaunchProgressEvent | undefined>(
		instanceId ? progressMap.get(instanceId) : undefined,
	)

	useEffect(() => {
		if (!instanceId) {
			setProgress(undefined)
			return
		}
		return instanceService.subscribeProgress((map) => {
			setProgress(map.get(instanceId))
		})
	}, [instanceId])

	return progress
}

export function getRequiredJavaVersion(gameVersion: string): number {
	const parts = gameVersion.split(".")
	if (parts.length > 0) {
		const first = Number.parseInt(parts[0], 10)
		if (first >= 25) return 25
		if (first >= 20) return 21
		if (first === 1 && parts.length >= 2) {
			const minor = Number.parseInt(parts[1], 10)
			if (minor >= 21) return 21
			if (minor === 20) {
				const patch = parts[2] ? Number.parseInt(parts[2], 10) : 0
				return patch >= 5 ? 21 : 17
			}
			if (minor >= 18) return 17
			if (minor === 17) return 16
			return 8
		}
	}
	if (gameVersion.startsWith("26w") || gameVersion.startsWith("25w")) {
		return 25
	}
	return 21
}
