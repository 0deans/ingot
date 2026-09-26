/**
 * React Query hooks for live server data: players, map, configs and access lists.
 * Live data is polled only while the server is running.
 */
import {
	keepPreviousData,
	useInfiniteQuery,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query"
import { useEffect, useState } from "react"
import type {
	AccessEntry,
	AccessListKind,
	PluginSearchResult,
	PluginSource,
	PropertyEntry,
	ServerStatus,
} from "@/bindings"
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

/** `interval`: the map polls faster so player heads move smoothly */
export function useOnlinePlayers(serverId: string, isRunning: boolean, interval = 3000) {
	return useQuery({
		queryKey: serverKeys.online(serverId),
		queryFn: () => rpc.get_online_players(serverId),
		enabled: isRunning,
		refetchInterval: interval,
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

/** `modified` (the region file's change time) makes changed regions refetch */
export function useMapTile(
	serverId: string,
	dimension: string,
	x: number,
	z: number,
	modified: number,
) {
	return useQuery({
		queryKey: serverKeys.tile(serverId, dimension, x, z, modified),
		queryFn: () => rpc.get_map_tile(serverId, dimension, x, z),
		// Keep showing the previous image while a redrawn tile loads (no flicker)
		placeholderData: keepPreviousData,
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

// ─── Plugins / mods ───────────────────────────────────────────────────────────

export const pluginKeys = {
	installed: (id: string) => ["server", id, "plugins", "installed"] as const,
	updates: (id: string) => ["server", id, "plugins", "updates"] as const,
	search: (id: string, source: PluginSource, query: string, sort: string, compatible: boolean) =>
		["server", id, "plugins", "search", source, query, sort, compatible] as const,
	versions: (id: string, source: PluginSource, project: string, compatible: boolean) =>
		["server", id, "plugins", "versions", source, project, compatible] as const,
	page: (source: PluginSource, project: string) => ["plugin-page", source, project] as const,
}

export function useInstalledPlugins(serverId: string) {
	return useQuery({
		queryKey: pluginKeys.installed(serverId),
		queryFn: () => rpc.list_server_plugins(serverId),
	})
}

export function usePluginUpdates(serverId: string, enabled: boolean) {
	return useQuery({
		queryKey: pluginKeys.updates(serverId),
		queryFn: () => rpc.check_server_plugin_updates(serverId),
		enabled,
		staleTime: 10 * 60_000,
	})
}

export function usePluginSearch(
	serverId: string,
	source: PluginSource,
	query: string,
	sort: string,
	compatibleOnly: boolean,
) {
	return useInfiniteQuery({
		queryKey: pluginKeys.search(serverId, source, query, sort, compatibleOnly),
		queryFn: ({ pageParam }) =>
			rpc.search_server_plugins(serverId, source, query, sort, compatibleOnly, pageParam),
		initialPageParam: 0,
		getNextPageParam: (last: PluginSearchResult, pages) => {
			const loaded = pages.reduce((n, p) => n + p.items.length, 0)
			return last.items.length > 0 && loaded < last.total ? pages.length : undefined
		},
		staleTime: 5 * 60_000,
	})
}

export function usePluginVersions(
	serverId: string,
	source: PluginSource,
	projectId: string | null,
	compatibleOnly: boolean,
) {
	return useQuery({
		queryKey: pluginKeys.versions(serverId, source, projectId ?? "", compatibleOnly),
		queryFn: () =>
			rpc.get_server_plugin_versions(serverId, source, projectId ?? "", compatibleOnly),
		enabled: Boolean(projectId),
		staleTime: 5 * 60_000,
	})
}

export function usePluginPage(source: PluginSource, projectId: string | null) {
	return useQuery({
		queryKey: pluginKeys.page(source, projectId ?? ""),
		queryFn: () => rpc.get_server_plugin_page(source, projectId ?? ""),
		enabled: Boolean(projectId),
		staleTime: 30 * 60_000,
	})
}

/** Install/remove/toggle, refreshing the installed list afterwards */
export function usePluginActions(serverId: string) {
	const queryClient = useQueryClient()
	const refresh = () =>
		Promise.all([
			queryClient.invalidateQueries({ queryKey: pluginKeys.installed(serverId) }),
			queryClient.invalidateQueries({ queryKey: pluginKeys.updates(serverId) }),
		])
	const install = useMutation({
		mutationFn: (args: { source: PluginSource; projectId: string; versionId?: string | null }) =>
			rpc.install_server_plugin(serverId, args.source, args.projectId, args.versionId ?? null),
		onSettled: refresh,
	})
	const remove = useMutation({
		mutationFn: (fileName: string) => rpc.remove_server_plugin(serverId, fileName),
		onSettled: refresh,
	})
	const toggle = useMutation({
		mutationFn: (args: { fileName: string; enabled: boolean }) =>
			rpc.set_server_plugin_enabled(serverId, args.fileName, args.enabled),
		onSettled: refresh,
	})
	return { install, remove, toggle }
}

// ─── Performance ──────────────────────────────────────────────────────────────

export function useServerStats(serverId: string, isRunning: boolean) {
	return useQuery({
		queryKey: ["server", serverId, "stats"],
		queryFn: () => rpc.get_server_stats(serverId),
		enabled: isRunning,
		refetchInterval: 2000,
		retry: false,
	})
}

/** Disk usage of the server folder; directory walks are cheap but not free, so poll slowly */
export function useServerStorage(serverId: string) {
	return useQuery({
		queryKey: ["server", serverId, "storage"],
		queryFn: () => rpc.get_server_storage(serverId),
		refetchInterval: 60_000,
		staleTime: 15_000,
	})
}

export function useLanAddress() {
	return useQuery({
		queryKey: ["lan-address"],
		queryFn: () => rpc.get_lan_address(),
		staleTime: 60_000,
	})
}
