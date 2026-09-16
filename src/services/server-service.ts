import { useEffect, useState } from "react"
import {
	createTauRPCProxy,
	type RunningServerSummary,
	type ServerConfig,
	type ServerCoreType,
	type ServerLogEvent,
	type ServerPingResponse,
	type ServerProperties,
	type ServerStatusEvent,
} from "@/bindings"

export const rpc = createTauRPCProxy()

let cachedServers: ServerConfig[] = []
let cachedRunning: Map<string, RunningServerSummary> = new Map()
const logSubscribers = new Map<string, Set<(event: ServerLogEvent) => void>>()

const serverListeners = new Set<(servers: ServerConfig[]) => void>()
const runningListeners = new Set<(running: Map<string, RunningServerSummary>) => void>()

let isInitialized = false

async function initListeners() {
	if (isInitialized) return
	isInitialized = true

	try {
		const running = await rpc.get_running_servers()
		cachedRunning = new Map(running.map((r) => [r.serverId, r]))
		notifyRunning()
	} catch (e) {
		console.error("Failed to load initial running servers:", e)
	}

	try {
		await rpc.on_server_status_changed.on((event: ServerStatusEvent) => {
			if (!event?.serverId) return
			if (event.status === "running" && event.pid) {
				const existing = cachedRunning.get(event.serverId)
				cachedRunning.set(event.serverId, {
					serverId: event.serverId,
					pid: event.pid,
					port: existing?.port || 25565,
					uptimeSeconds: 0,
					status: "running",
				})
			} else if (event.status === "stopped") {
				cachedRunning.delete(event.serverId)
			} else {
				const existing = cachedRunning.get(event.serverId)
				if (existing) {
					cachedRunning.set(event.serverId, {
						...existing,
						status: event.status,
					})
				}
			}
			notifyRunning()
			serverService.refreshServers()
		})
	} catch (e) {
		console.error("Failed to setup on_server_status_changed listener:", e)
	}

	try {
		await rpc.on_server_log.on((event: ServerLogEvent) => {
			if (!event?.serverId) return
			const subs = logSubscribers.get(event.serverId)
			if (subs) {
				for (const sub of subs) {
					sub(event)
				}
			}
		})
	} catch (e) {
		console.error("Failed to setup on_server_log listener:", e)
	}
}

function notifyServers() {
	for (const listener of serverListeners) {
		listener([...cachedServers])
	}
}

function notifyRunning() {
	const copy = new Map(cachedRunning)
	for (const listener of runningListeners) {
		listener(copy)
	}
}

export const serverService = {
	async getServers(): Promise<ServerConfig[]> {
		return this.refreshServers()
	},

	async refreshServers(): Promise<ServerConfig[]> {
		try {
			await initListeners()
			const servers = await rpc.get_servers()
			cachedServers = servers
			notifyServers()
			return servers
		} catch (e) {
			console.error("Failed to load servers:", e)
			return []
		}
	},

	async createServer(
		name: string,
		core: ServerCoreType,
		gameVersion: string,
		buildNumber: string | null = null,
		port: number | null = null,
		memoryMinMb: number | null = null,
		memoryMaxMb: number | null = null,
	): Promise<ServerConfig> {
		const created = await rpc.create_server(
			name,
			core,
			gameVersion,
			buildNumber,
			port,
			memoryMinMb,
			memoryMaxMb,
		)
		await this.refreshServers()
		return created
	},

	async deleteServer(serverId: string, deleteFiles = true): Promise<void> {
		await rpc.delete_server(serverId, deleteFiles)
		await this.refreshServers()
	},

	async updateServer(server: ServerConfig): Promise<void> {
		await rpc.update_server(server)
		await this.refreshServers()
	},

	async getServerProperties(serverId: string): Promise<ServerProperties> {
		return rpc.get_server_properties(serverId)
	},

	async setServerProperties(serverId: string, properties: ServerProperties): Promise<void> {
		await rpc.set_server_properties(serverId, properties)
	},

	async openServerFolder(serverId: string): Promise<void> {
		await rpc.open_server_folder(serverId)
	},

	async startServer(serverId: string): Promise<number> {
		return rpc.start_server(serverId)
	},

	async stopServer(serverId: string): Promise<void> {
		await rpc.stop_server(serverId)
	},

	async killServer(serverId: string): Promise<void> {
		await rpc.kill_server(serverId)
	},

	async sendCommand(serverId: string, command: string): Promise<void> {
		await rpc.send_server_command(serverId, command)
	},

	async getServerLogs(serverId: string): Promise<string[]> {
		return rpc.get_server_logs(serverId)
	},

	async getAvailableServerCoreVersions(core: ServerCoreType): Promise<string[]> {
		return rpc.get_available_server_core_versions(core)
	},

	async getServerOnlinePlayers(serverId: string): Promise<string[]> {
		return rpc.get_server_online_players(serverId)
	},

	async pingServer(port: number): Promise<ServerPingResponse> {
		return rpc.ping_server(port)
	},

	async getServerIcon(serverId: string): Promise<string | null> {
		return rpc.get_server_icon(serverId)
	},

	async setServerIcon(serverId: string, base64Data: string): Promise<void> {
		await rpc.set_server_icon(serverId, base64Data)
	},

	async getServerWhitelist(serverId: string) {
		return rpc.get_server_whitelist(serverId)
	},

	async addToServerWhitelist(serverId: string, username: string): Promise<void> {
		await rpc.add_to_server_whitelist(serverId, username)
	},

	async removeFromServerWhitelist(serverId: string, username: string): Promise<void> {
		await rpc.remove_from_server_whitelist(serverId, username)
	},

	isPortInUse(port: number, excludeServerId?: string): boolean {
		return cachedServers.some((s) => s.port === port && s.id !== excludeServerId)
	},

	getNextAvailablePort(): number {
		const usedPorts = new Set(cachedServers.map((s) => s.port))
		let port = 25565
		while (usedPorts.has(port)) {
			port += 1
		}
		return port
	},
}

export function useServers() {
	const [servers, setServers] = useState<ServerConfig[]>(cachedServers)
	const [isLoading, setIsLoading] = useState(cachedServers.length === 0)

	useEffect(() => {
		let isMounted = true

		const listener = (updated: ServerConfig[]) => {
			if (isMounted) {
				setServers(updated)
				setIsLoading(false)
			}
		}

		serverListeners.add(listener)

		serverService.refreshServers().finally(() => {
			if (isMounted) setIsLoading(false)
		})

		return () => {
			isMounted = false
			serverListeners.delete(listener)
		}
	}, [])

	return {
		servers,
		isLoading,
		refresh: () => serverService.refreshServers(),
	}
}

export function useRunningServers() {
	const [runningMap, setRunningMap] = useState<Map<string, RunningServerSummary>>(cachedRunning)

	useEffect(() => {
		let isMounted = true

		const listener = (updated: Map<string, RunningServerSummary>) => {
			if (isMounted) {
				setRunningMap(updated)
			}
		}

		runningListeners.add(listener)
		initListeners()

		return () => {
			isMounted = false
			runningListeners.delete(listener)
		}
	}, [])

	const runningList = Array.from(runningMap.values())

	return {
		runningMap,
		runningList,
	}
}

export function useServerLogs(serverId: string | null) {
	const [logs, setLogs] = useState<string[]>([])
	const [isLoadingHistory, setIsLoadingHistory] = useState(false)

	useEffect(() => {
		if (!serverId) {
			setLogs([])
			return
		}

		let isMounted = true
		setIsLoadingHistory(true)

		serverService
			.getServerLogs(serverId)
			.then((history) => {
				if (isMounted) {
					setLogs(history)
					setIsLoadingHistory(false)
				}
			})
			.catch((err) => {
				console.error("Failed to load initial server logs:", err)
				if (isMounted) setIsLoadingHistory(false)
			})

		const listener = (event: ServerLogEvent) => {
			if (event.serverId === serverId) {
				setLogs((prev) => [...prev, event.line])
			}
		}

		if (!logSubscribers.has(serverId)) {
			logSubscribers.set(serverId, new Set())
		}
		logSubscribers.get(serverId)?.add(listener)

		return () => {
			isMounted = false
			const set = logSubscribers.get(serverId)
			if (set) {
				set.delete(listener)
				if (set.size === 0) {
					logSubscribers.delete(serverId)
				}
			}
		}
	}, [serverId])

	const sendCommand = async (command: string) => {
		if (!serverId || !command.trim()) return
		await serverService.sendCommand(serverId, command)
	}

	const clearLogs = () => {
		setLogs([])
	}

	return {
		logs,
		isLoadingHistory,
		sendCommand,
		clearLogs,
	}
}
