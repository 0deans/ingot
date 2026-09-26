/**
 * React Query hooks for live server data: players, map, configs and access lists.
 * Live data is polled only while the server is running.
 */
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { useEffect, useState } from "react"
import type { AccessEntry, AccessListKind, PropertyEntry, ServerStatus } from "@/bindings"
import { isValidServerIcon, toServerIcon } from "@/lib/server-icon"
import { rpc, useRunningServers } from "@/services/server-service"

export const serverKeys = {
	online: (id: string) => ["server", id, "online-players"] as const,
	known: (id: string) => ["server", id, "known-players"] as const,
	player: (id: string, name: string) => ["server", id, "player", name] as const,
	access: (id: string, kind: AccessListKind) => ["server", id, "access", kind] as const,
	properties: (id: string) => ["server", id, "properties"] as const,
	dimensions: (id: string) => ["server", id, "map-dimensions"] as const,
	tile: (id: string, dim: string, x: number, z: number, rev: number) =>
		["server", id, "map-tile", dim, x, z, rev] as const,
	configFiles: (id: string) => ["server", id, "config-files"] as const,
	configFile: (id: string, path: string) => ["server", id, "config-file", path] as const,
	icon: (id: string) => ["server", id, "icon"] as const,
}

/** Status of one server: "stopped" when it has no process */
export function useServerStatus(serverId: string | null | undefined): {
	status: ServerStatus
	isRunning: boolean
	uptimeSeconds: number
} {
	const { runningMap } = useRunningServers()
	const info = serverId ? runningMap.get(serverId) : undefined
	const status = info?.status ?? "stopped"
	return { status, isRunning: status === "running", uptimeSeconds: info?.uptimeSeconds ?? 0 }
}

export function useOnlinePlayers(serverId: string, isRunning: boolean) {
	return useQuery({
		queryKey: serverKeys.online(serverId),
		queryFn: () => rpc.get_online_players(serverId),
		enabled: isRunning,
		refetchInterval: 3000,
		retry: false,
		placeholderData: keepPreviousData,
	})
}

export function useKnownPlayers(serverId: string, isRunning: boolean) {
	return useQuery({
		queryKey: serverKeys.known(serverId),
		queryFn: () => rpc.get_known_players(serverId),
		refetchInterval: isRunning ? 10_000 : false,
		placeholderData: keepPreviousData,
	})
}

export function usePlayerDetails(serverId: string, name: string | null, live: boolean) {
	return useQuery({
		queryKey: serverKeys.player(serverId, name ?? ""),
		queryFn: () => rpc.get_player_details(serverId, name ?? ""),
		enabled: Boolean(name),
		refetchInterval: live ? 2000 : false,
		retry: false,
		placeholderData: keepPreviousData,
	})
}

export function useAccessList(serverId: string, kind: AccessListKind) {
	const queryClient = useQueryClient()
	const key = serverKeys.access(serverId, kind)
	const query = useQuery({
		queryKey: key,
		queryFn: () => rpc.get_access_list(serverId, kind),
	})
	const setList = (list: AccessEntry[]) => queryClient.setQueryData(key, list)
	const add = useMutation({
		mutationFn: (name: string) => rpc.add_access_entry(serverId, kind, name),
		onSuccess: setList,
	})
	const remove = useMutation({
		mutationFn: (name: string) => rpc.remove_access_entry(serverId, kind, name),
		onSuccess: setList,
	})
	return { ...query, add, remove }
}

export function useServerPropertiesAll(serverId: string) {
	const queryClient = useQueryClient()
	const query = useQuery({
		queryKey: serverKeys.properties(serverId),
		queryFn: () => rpc.get_server_properties_all(serverId),
	})
	const save = useMutation({
		mutationFn: (entries: PropertyEntry[]) => rpc.set_server_properties_all(serverId, entries),
		onSuccess: () => queryClient.invalidateQueries({ queryKey: serverKeys.properties(serverId) }),
	})
	/** key -> value lookup */
	const values = new Map((query.data ?? []).map((e) => [e.key, e.value]))
	return { ...query, values, save }
}

export function useMapDimensions(serverId: string) {
	return useQuery({
		queryKey: serverKeys.dimensions(serverId),
		queryFn: () => rpc.get_map_dimensions(serverId),
	})
}

/** `revision` busts the cache after the world was saved */
export function useMapTile(
	serverId: string,
	dimension: string,
	x: number,
	z: number,
	revision: number,
) {
	return useQuery({
		queryKey: serverKeys.tile(serverId, dimension, x, z, revision),
		queryFn: () => rpc.get_map_tile(serverId, dimension, x, z),
		staleTime: Number.POSITIVE_INFINITY,
		gcTime: 5 * 60_000,
	})
}

export function useConfigFiles(serverId: string) {
	return useQuery({
		queryKey: serverKeys.configFiles(serverId),
		queryFn: () => rpc.list_server_config_files(serverId),
	})
}

export function useConfigFile(serverId: string, path: string | null) {
	return useQuery({
		queryKey: serverKeys.configFile(serverId, path ?? ""),
		queryFn: () => rpc.read_server_config_file(serverId, path ?? ""),
		enabled: Boolean(path),
		staleTime: 0,
	})
}

/**
 * The server icon. Icons saved by older versions (any size, sometimes JPEG bytes)
 * are converted to the 64×64 PNG Minecraft requires, so they show up in-game.
 */
export function useServerIcon(serverId: string) {
	return useQuery({
		queryKey: serverKeys.icon(serverId),
		queryFn: async () => {
			const icon = await rpc.get_server_icon(serverId)
			if (!icon || (await isValidServerIcon(icon))) return icon
			try {
				const fixed = await toServerIcon(icon)
				await rpc.set_server_icon(serverId, fixed)
				return fixed
			} catch {
				return icon
			}
		},
	})
}

/** When each running server started (ms); module-level so it survives remounts */
const startTimes = new Map<string, number>()

/** Seconds since the server started, ticking every second while it runs */
export function useLiveUptime(serverId: string): number {
	const { isRunning, uptimeSeconds } = useServerStatus(serverId)
	const [now, setNow] = useState(Date.now())

	if (!isRunning) startTimes.delete(serverId)
	else if (!startTimes.has(serverId)) startTimes.set(serverId, Date.now() - uptimeSeconds * 1000)

	useEffect(() => {
		if (!isRunning) return
		const timer = setInterval(() => setNow(Date.now()), 1000)
		return () => clearInterval(timer)
	}, [isRunning])

	const startedAt = startTimes.get(serverId)
	return startedAt === undefined ? 0 : Math.max(0, (now - startedAt) / 1000)
}
