/** Server lifecycle, the playit.gg tunnel and the Android foreground service */
import { useQuery, useQueryClient } from "@tanstack/react-query"
import { useEffect, useState } from "react"
import type { PlayitTunnelStatus, ServerConfig } from "@/bindings"
import { serverService, useRunningServers, useServerLogs } from "@/services/server-service"

/** Start/stop with instant feedback and a readable error */
export function useServerControls(serverId: string | null | undefined) {
	const [isStartRequested, setIsStartRequested] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [isStopping, setIsStopping] = useState(false)

	const start = async () => {
		if (!serverId || isStartRequested) return
		setError(null)
		setIsStartRequested(true)
		try {
			await serverService.startServer(serverId)
		} catch (e) {
			setError(String(e))
		} finally {
			setIsStartRequested(false)
		}
	}

	const stop = async () => {
		if (!serverId) return
		setIsStopping(true)
		try {
			await serverService.stopServer(serverId)
		} catch (e) {
			setError(String(e))
		} finally {
			setIsStopping(false)
		}
	}

	const sleep = async () => {
		if (!serverId) return
		try {
			await serverService.putServerToSleep(serverId)
		} catch (e) {
			setError(String(e))
		}
	}

	return {
		start,
		stop,
		sleep,
		isStartRequested,
		isStopping,
		error,
		clearError: () => setError(null),
	}
}

/** The most recent "[Ingot] ..." setup step, shown while a server starts */
export function useStartupStep(serverId: string | null | undefined): string | undefined {
	const { logs } = useServerLogs(serverId ?? null)
	for (let i = logs.length - 1; i >= 0; i--) {
		if (logs[i].startsWith("[Ingot]")) return logs[i].replace("[Ingot] ", "")
	}
	return undefined
}

const STOPPED_TUNNEL: PlayitTunnelStatus = {
	isRunning: false,
	status: "stopped",
	claimUrl: null,
	publicAddress: null,
	pingMs: null,
	message: null,
}

/** App-wide playit.gg tunnel (one agent for all servers) */
export function usePlayitTunnel() {
	const queryClient = useQueryClient()
	const [isBusy, setIsBusy] = useState(false)
	const { data = STOPPED_TUNNEL } = useQuery({
		queryKey: ["playit"],
		queryFn: () => serverService.getPlayitStatus(),
		// Poll quickly while connecting so each step shows up promptly
		refetchInterval: (q) => {
			const s = q.state.data
			return s?.isRunning && s.status !== "connected" ? 1000 : 5000
		},
	})

	const toggle = async (secretKey?: string | null) => {
		setIsBusy(true)
		try {
			if (data.isRunning) {
				await serverService.stopPlayitTunnel()
			} else {
				queryClient.setQueryData(["playit"], {
					...STOPPED_TUNNEL,
					isRunning: true,
					status: "starting",
				})
				await serverService.startPlayitTunnel(secretKey)
			}
		} finally {
			await queryClient.invalidateQueries({ queryKey: ["playit"] })
			setIsBusy(false)
		}
	}

	return { tunnel: data, toggle, isBusy }
}

/** Injected by MainActivity.kt; controls the Android foreground service */
interface IngotHostBridge {
	start(title: string, detail: string): void
	stop(): void
}

/**
 * Keeps the Android foreground service up while anything is hosted. Without it the
 * app is "cached" as soon as it leaves the screen and Android cuts off its network.
 */
export function useAndroidHostService(servers: ServerConfig[]) {
	const { runningList } = useRunningServers()
	const { tunnel } = usePlayitTunnel()
	const active = runningList.filter((s) => s.status !== "stopped")
	const isHosting = active.length > 0 || tunnel.isRunning
	const title =
		active.length === 1
			? `Hosting ${servers.find((s) => s.id === active[0].serverId)?.name ?? "server"}`
			: active.length > 1
				? `Hosting ${active.length} servers`
				: "Ingot tunnel active"
	const detail = tunnel.publicAddress
		? `Public address: ${tunnel.publicAddress}`
		: tunnel.isRunning
			? "playit.gg tunnel connecting..."
			: "Local network only"

	useEffect(() => {
		const host = (window as Window & { IngotHost?: IngotHostBridge }).IngotHost
		if (!host) return
		if (isHosting) host.start(title, detail)
		else host.stop()
	}, [isHosting, title, detail])
}
