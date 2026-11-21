import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type PluginSource } from "@/bindings"
import { serverKeys } from "@/features/servers/api/serversApi"
import { STATIC_STALE_TIME_MS } from "@/shared/api/queryClient"
import { rpc } from "@/shared/api/rpc"

export interface ServerPluginSearchParams {
	serverId: string
	source: PluginSource
	query: string
	sort: string
	compatibleOnly: boolean
	page: number
}

export const serverPluginKeys = {
	all: ["serverPlugins"] as const,
	search: (params: ServerPluginSearchParams) => {
		return [...serverPluginKeys.all, "search", params] as const
	},
	page: (source: PluginSource, projectId: string) => {
		return [...serverPluginKeys.all, "page", source, projectId] as const
	},
	versions: (
		serverId: string,
		source: PluginSource,
		projectId: string,
		compatibleOnly: boolean,
	) => {
		return [
			...serverPluginKeys.all,
			"versions",
			serverId,
			source,
			projectId,
			compatibleOnly,
		] as const
	},
}

export const serverPluginSearchQueryOptions = (params: ServerPluginSearchParams) => {
	return queryOptions({
		queryKey: serverPluginKeys.search(params),
		queryFn: () => {
			return rpc.servers.plugins.search_server_plugins(
				params.serverId,
				params.source,
				params.query,
				params.sort,
				params.compatibleOnly,
				params.page,
			)
		},
		enabled: params.serverId.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const serverPluginPageQueryOptions = (source: PluginSource, projectId: string) => {
	return queryOptions({
		queryKey: serverPluginKeys.page(source, projectId),
		queryFn: () => {
			return rpc.servers.plugins.get_server_plugin_page(source, projectId)
		},
		enabled: projectId.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const serverPluginVersionsQueryOptions = (
	serverId: string,
	source: PluginSource,
	projectId: string,
	compatibleOnly: boolean,
) => {
	return queryOptions({
		queryKey: serverPluginKeys.versions(serverId, source, projectId, compatibleOnly),
		queryFn: () => {
			return rpc.servers.plugins.get_server_plugin_versions(
				serverId,
				source,
				projectId,
				compatibleOnly,
			)
		},
		enabled: serverId.length > 0 && projectId.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const useSearchServerPlugins = (params: ServerPluginSearchParams) => {
	return useQuery(serverPluginSearchQueryOptions(params))
}

export const useServerPluginPage = (source: PluginSource, projectId: string) => {
	return useQuery(serverPluginPageQueryOptions(source, projectId))
}

export const useServerPluginVersions = (
	serverId: string,
	source: PluginSource,
	projectId: string,
	compatibleOnly: boolean,
) => {
	return useQuery(serverPluginVersionsQueryOptions(serverId, source, projectId, compatibleOnly))
}

export const useCheckServerPluginUpdates = () => {
	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.plugins.check_server_plugin_updates(serverId)
		},
	})
}

export const useInstallServerPlugin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			serverId: string
			source: PluginSource
			projectId: string
			versionId: string | null
		}) => {
			return rpc.servers.plugins.install_server_plugin(
				input.serverId,
				input.source,
				input.projectId,
				input.versionId,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.plugins(variables.serverId) })
		},
	})
}

export const useSetServerPluginEnabled = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; fileName: string; enabled: boolean }) => {
			return rpc.servers.plugins.set_server_plugin_enabled(
				input.serverId,
				input.fileName,
				input.enabled,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.plugins(variables.serverId) })
		},
	})
}

export const useRemoveServerPlugin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; fileName: string }) => {
			return rpc.servers.plugins.remove_server_plugin(input.serverId, input.fileName)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.plugins(variables.serverId) })
		},
	})
}
