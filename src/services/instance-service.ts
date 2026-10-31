import i18n from "i18next"
import { useEffect, useState } from "react"
import { create } from "zustand"
import type {
	HorizonRelease,
	InstanceConfig,
	InstanceStatusEvent,
	InstanceWorldSummary,
	LaunchProgressEvent,
	ModLoaderType,
	QuickPlayOptions,
	RunningInstanceSummary,
	VersionManifestEntry,
} from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

interface InstanceState {
	instances: InstanceConfig[]
	running: Map<string, RunningInstanceSummary>
	progress: Map<string, LaunchProgressEvent>
}

const useInstanceStore = create<InstanceState>(() => {
	return {
		instances: [],
		running: new Map(),
		progress: new Map(),
	}
})

// Maps are replaced (never mutated) so selectors see a new reference on every change.
function setRunning(update: (running: Map<string, RunningInstanceSummary>) => void) {
	const running = new Map(useInstanceStore.getState().running)
	update(running)
	useInstanceStore.setState({ running })
}

function setProgress(update: (progress: Map<string, LaunchProgressEvent>) => void) {
	const progress = new Map(useInstanceStore.getState().progress)
	update(progress)
	useInstanceStore.setState({ progress })
}

let isInitialized = false

async function initListeners() {
	if (isInitialized) return
	isInitialized = true

	try {
		const running = await unwrapRpc(rpc.instances.get_running_instances())
		useInstanceStore.setState({
			running: new Map(
				running.map((r) => {
					return [r.instanceId, r]
				}),
			),
		})
	} catch (e) {
		console.error("Failed to load initial running instances:", e)
	}

	try {
		await rpc.events.on_instance_status_changed.on((event: InstanceStatusEvent) => {
			if (!event?.instanceId) return
			setRunning((running) => {
				if (event.isRunning) {
					running.set(event.instanceId, {
						instanceId: event.instanceId,
						pid: event.pid,
						startedAt: event.startedAt,
					})
				} else {
					running.delete(event.instanceId)
				}
			})
			setProgress((progress) => {
				progress.delete(event.instanceId)
			})
			instanceService.refreshInstances()
		})
	} catch (e) {
		console.error("Failed to setup on_instance_status_changed listener:", e)
	}

	try {
		await rpc.events.on_launch_progress.on((event: LaunchProgressEvent) => {
			if (!event?.instanceId) return
			setProgress((progress) => {
				progress.set(event.instanceId, event)
			})
		})
	} catch (e) {
		console.error("Failed to setup on_launch_progress listener:", e)
	}
}

let refreshInstancesPromise: Promise<InstanceConfig[]> | null = null

export const instanceService = {
	async getInstances(): Promise<InstanceConfig[]> {
		return this.refreshInstances()
	},

	async refreshInstances(): Promise<InstanceConfig[]> {
		if (refreshInstancesPromise) return refreshInstancesPromise

		refreshInstancesPromise = (async () => {
			try {
				await initListeners()
				const instances = await unwrapRpc(rpc.instances.get_instances())
				useInstanceStore.setState({ instances })
				return instances
			} catch (e) {
				console.error("Failed to fetch instances:", e)
				return useInstanceStore.getState().instances
			} finally {
				refreshInstancesPromise = null
			}
		})()

		return refreshInstancesPromise
	},

	async createInstance(
		name: string,
		gameVersion: string,
		loader: ModLoaderType,
		loaderVersion: string | null = null,
	): Promise<InstanceConfig> {
		const created = await unwrapRpc(
			rpc.instances.create_instance(name, gameVersion, loader, loaderVersion),
		)
		await this.refreshInstances()
		return created
	},

	async duplicateInstance(
		instanceId: string,
		name: string,
		includeWorlds: boolean,
	): Promise<InstanceConfig> {
		const copy = await unwrapRpc(rpc.instances.duplicate_instance(instanceId, name, includeWorlds))
		await this.refreshInstances()
		return copy
	},

	async deleteInstance(instanceId: string): Promise<void> {
		useInstanceStore.setState((state) => {
			return {
				instances: state.instances.filter((i) => {
					return i.id !== instanceId
				}),
			}
		})
		try {
			await unwrapRpc(rpc.instances.delete_instance(instanceId))
		} finally {
			await this.refreshInstances()
		}
	},

	async updateInstance(instance: InstanceConfig): Promise<void> {
		await unwrapRpc(rpc.instances.update_instance(instance))
		await this.refreshInstances()
	},

	async openInstanceFolder(instanceId: string): Promise<void> {
		await unwrapRpc(rpc.instances.open_instance_folder(instanceId))
	},

	async launchInstance(instanceId: string, quickPlay?: QuickPlayOptions | null): Promise<number> {
		setProgress((progress) => {
			progress.set(instanceId, {
				instanceId,
				phase: "Preparing launch",
				currentStep: 0,
				totalSteps: 10,
				percentage: 0,
				detail: quickPlay?.server
					? i18n.t("backend.launch.connecting", { server: quickPlay.server })
					: quickPlay?.world
						? i18n.t("backend.launch.loadingWorld", { world: quickPlay.world })
						: i18n.t("backend.launch.initializing"),
			})
		})
		try {
			const pid = await unwrapRpc(rpc.instances.launch_instance(instanceId, quickPlay || null))
			return pid
		} catch (e) {
			setProgress((progress) => {
				progress.delete(instanceId)
			})
			throw e
		}
	},

	async getInstanceWorlds(instanceId: string): Promise<InstanceWorldSummary[]> {
		return unwrapRpc(rpc.instances.get_instance_worlds(instanceId))
	},

	async killInstance(instanceId: string): Promise<void> {
		await unwrapRpc(rpc.instances.kill_instance(instanceId))
		setRunning((running) => {
			running.delete(instanceId)
		})
		setProgress((progress) => {
			progress.delete(instanceId)
		})
	},

	async getAvailableGameVersions(): Promise<VersionManifestEntry[]> {
		return unwrapRpc(rpc.versions.get_available_game_versions())
	},

	async getAvailableLoaderVersions(gameVersion: string, loader: ModLoaderType): Promise<string[]> {
		return unwrapRpc(rpc.versions.get_available_loader_versions(gameVersion, loader))
	},

	async getAvailableHorizonReleases(): Promise<HorizonRelease[]> {
		return unwrapRpc(rpc.versions.get_available_horizon_releases())
	},
}

export function useInstances() {
	const instances = useInstanceStore((s) => {
		return s.instances
	})
	const [isLoading, setIsLoading] = useState(instances.length === 0)

	useEffect(() => {
		instanceService.refreshInstances().finally(() => {
			setIsLoading(false)
		})
	}, [])

	return {
		instances,
		isLoading,
		refresh: () => {
			return instanceService.refreshInstances()
		},
	}
}

export function useCachedInstances(): InstanceConfig[] {
	return useInstanceStore((s) => {
		return s.instances
	})
}

export function useRunningInstances() {
	const running = useInstanceStore((s) => {
		return s.running
	})

	useEffect(() => {
		initListeners()
	}, [])

	return {
		runningMap: running,
		runningList: Array.from(running.values()),
		isRunning: (instanceId: string) => {
			return running.has(instanceId)
		},
		getRunningInfo: (instanceId: string) => {
			return running.get(instanceId)
		},
	}
}

export function useAllInstancesProgress(): Map<string, LaunchProgressEvent> {
	return useInstanceStore((s) => {
		return s.progress
	})
}

export function useInstanceProgress(instanceId: string | undefined) {
	return useInstanceStore((s) => {
		return instanceId ? s.progress.get(instanceId) : undefined
	})
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

export { rpc }
