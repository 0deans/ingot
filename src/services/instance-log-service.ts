import { useCallback, useEffect, useState } from "react"
import { create } from "zustand"
import type {
	InstanceCrashEvent,
	InstanceLogEvent,
	InstanceLogFileContent,
	InstanceLogFileSummary,
	InstanceLogLine,
	LogFileKind,
	PasteProvider,
	PasteUploadResult,
} from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

const MAX_CLIENT_LOG_LINES = 2500

interface InstanceLogStoreState {
	crashesByInstance: Map<string, InstanceCrashEvent>
	latestCrashAlert: InstanceCrashEvent | null
}

const useInstanceLogStore = create<InstanceLogStoreState>(() => {
	return {
		crashesByInstance: new Map(),
		latestCrashAlert: null,
	}
})

const logSubscribers = new Map<string, Set<(event: InstanceLogEvent) => void>>()
let listenersInitialized = false

async function ensureLogListeners(): Promise<void> {
	if (listenersInitialized) {
		return
	}
	listenersInitialized = true

	try {
		await rpc.events.on_instance_log.on((event: InstanceLogEvent) => {
			if (!event?.instanceId) {
				return
			}
			const subscribers = logSubscribers.get(event.instanceId)
			if (!subscribers) {
				return
			}
			for (const callback of subscribers) {
				callback(event)
			}
		})
	} catch (error) {
		console.error("Failed to subscribe to on_instance_log:", error)
	}

	try {
		await rpc.events.on_instance_crash.on((event: InstanceCrashEvent) => {
			if (!event?.instanceId) {
				return
			}
			const nextMap = new Map(useInstanceLogStore.getState().crashesByInstance)
			nextMap.set(event.instanceId, event)
			useInstanceLogStore.setState({
				crashesByInstance: nextMap,
				latestCrashAlert: event,
			})
		})
	} catch (error) {
		console.error("Failed to subscribe to on_instance_crash:", error)
	}
}

export const instanceLogService = {
	async getLiveLogs(instanceId: string): Promise<InstanceLogLine[]> {
		await ensureLogListeners()
		return unwrapRpc(rpc.instances.logs.get_live_instance_logs(instanceId))
	},

	async clearLiveLogs(instanceId: string): Promise<void> {
		await unwrapRpc(rpc.instances.logs.clear_live_instance_logs(instanceId))
	},

	async listLogFiles(instanceId: string): Promise<InstanceLogFileSummary[]> {
		return unwrapRpc(rpc.instances.logs.list_instance_log_files(instanceId))
	},

	async readLogFile(
		instanceId: string,
		kind: LogFileKind,
		fileName: string,
	): Promise<InstanceLogFileContent> {
		return unwrapRpc(rpc.instances.logs.read_instance_log_file(instanceId, kind, fileName))
	},

	async sanitizeLogText(content: string): Promise<string> {
		return unwrapRpc(rpc.instances.logs.sanitize_log_text(content))
	},

	async uploadLogPaste(content: string, provider: PasteProvider): Promise<PasteUploadResult> {
		return unwrapRpc(rpc.instances.logs.upload_log_paste(content, provider))
	},

	async openLogLocation(
		instanceId: string,
		kind?: LogFileKind | null,
		fileName?: string | null,
	): Promise<void> {
		await unwrapRpc(
			rpc.instances.logs.open_instance_log_location(instanceId, kind ?? null, fileName ?? null),
		)
	},

	dismissCrashAlert(): void {
		useInstanceLogStore.setState({ latestCrashAlert: null })
	},

	clearInstanceCrash(instanceId: string): void {
		const nextMap = new Map(useInstanceLogStore.getState().crashesByInstance)
		nextMap.delete(instanceId)
		useInstanceLogStore.setState({ crashesByInstance: nextMap })
	},
}

export const useInstanceLiveLogs = (instanceId: string | null) => {
	const [logs, setLogs] = useState<InstanceLogLine[]>([])
	const [isLoading, setIsLoading] = useState(Boolean(instanceId))
	const [prevInstanceId, setPrevInstanceId] = useState(instanceId)

	if (instanceId !== prevInstanceId) {
		setPrevInstanceId(instanceId)
		setLogs([])
		setIsLoading(Boolean(instanceId))
	}

	useEffect(() => {
		if (!instanceId) {
			return
		}

		let cancelled = false

		instanceLogService
			.getLiveLogs(instanceId)
			.then((history) => {
				if (!cancelled) {
					setLogs(history)
					setIsLoading(false)
				}
			})
			.catch((error) => {
				console.error("Failed to load live instance logs:", error)
				if (!cancelled) {
					setIsLoading(false)
				}
			})

		const listener = (event: InstanceLogEvent) => {
			if (event.instanceId !== instanceId) {
				return
			}
			setLogs((prev) => {
				if (prev.length > 0 && prev[prev.length - 1].seq >= event.line.seq) {
					return prev
				}
				const appended = [...prev, event.line]
				if (appended.length > MAX_CLIENT_LOG_LINES) {
					return appended.slice(appended.length - MAX_CLIENT_LOG_LINES)
				}
				return appended
			})
		}

		let set = logSubscribers.get(instanceId)
		if (!set) {
			set = new Set()
			logSubscribers.set(instanceId, set)
		}
		set.add(listener)

		return () => {
			cancelled = true
			const currentSet = logSubscribers.get(instanceId)
			if (currentSet) {
				currentSet.delete(listener)
				if (currentSet.size === 0) {
					logSubscribers.delete(instanceId)
				}
			}
		}
	}, [instanceId])

	const clearLogs = useCallback(async () => {
		if (!instanceId) {
			return
		}
		setLogs([])
		try {
			await instanceLogService.clearLiveLogs(instanceId)
		} catch (error) {
			console.error("Failed to clear instance logs:", error)
		}
	}, [instanceId])

	return {
		logs,
		isLoading,
		clearLogs,
	}
}

export const useInstanceCrashState = (instanceId?: string | null) => {
	useEffect(() => {
		ensureLogListeners()
	}, [])

	const latestCrashAlert = useInstanceLogStore((state) => {
		return state.latestCrashAlert
	})
	const instanceCrash = useInstanceLogStore((state) => {
		if (!instanceId) {
			return undefined
		}
		return state.crashesByInstance.get(instanceId)
	})

	return {
		latestCrashAlert,
		instanceCrash,
		dismissCrashAlert: instanceLogService.dismissCrashAlert,
		clearInstanceCrash: instanceLogService.clearInstanceCrash,
	}
}
