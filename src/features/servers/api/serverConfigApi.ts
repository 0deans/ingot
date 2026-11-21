import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type AccessListKind, type PropertyEntry, type ServerProperties } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const serverConfigKeys = {
	all: ["serverConfig"] as const,
	properties: (serverId: string) => {
		return [...serverConfigKeys.all, serverId, "properties"] as const
	},
	propertiesAll: (serverId: string) => {
		return [...serverConfigKeys.all, serverId, "propertiesAll"] as const
	},
	whitelist: (serverId: string) => {
		return [...serverConfigKeys.all, serverId, "whitelist"] as const
	},
	accessList: (serverId: string, kind: AccessListKind) => {
		return [...serverConfigKeys.all, serverId, "accessList", kind] as const
	},
	files: (serverId: string) => {
		return [...serverConfigKeys.all, serverId, "files"] as const
	},
	fileContent: (serverId: string, path: string) => {
		return [...serverConfigKeys.all, serverId, "fileContent", path] as const
	},
}

export const serverPropertiesQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverConfigKeys.properties(serverId),
		queryFn: () => {
			return rpc.servers.config.get_server_properties(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverPropertiesAllQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverConfigKeys.propertiesAll(serverId),
		queryFn: () => {
			return rpc.servers.config.get_server_properties_all(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverWhitelistQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverConfigKeys.whitelist(serverId),
		queryFn: () => {
			return rpc.servers.config.get_server_whitelist(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverAccessListQueryOptions = (serverId: string, kind: AccessListKind) => {
	return queryOptions({
		queryKey: serverConfigKeys.accessList(serverId, kind),
		queryFn: () => {
			return rpc.servers.config.get_access_list(serverId, kind)
		},
		enabled: serverId.length > 0,
	})
}

export const serverConfigFilesQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverConfigKeys.files(serverId),
		queryFn: () => {
			return rpc.servers.config.list_server_config_files(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverConfigFileContentQueryOptions = (serverId: string, path: string) => {
	return queryOptions({
		queryKey: serverConfigKeys.fileContent(serverId, path),
		queryFn: () => {
			return rpc.servers.config.read_server_config_file(serverId, path)
		},
		enabled: serverId.length > 0 && path.length > 0,
	})
}

export const useServerProperties = (serverId: string) => {
	return useQuery(serverPropertiesQueryOptions(serverId))
}

export const useServerPropertiesAll = (serverId: string) => {
	return useQuery(serverPropertiesAllQueryOptions(serverId))
}

export const useServerWhitelist = (serverId: string) => {
	return useQuery(serverWhitelistQueryOptions(serverId))
}

export const useServerAccessList = (serverId: string, kind: AccessListKind) => {
	return useQuery(serverAccessListQueryOptions(serverId, kind))
}

export const useServerConfigFiles = (serverId: string) => {
	return useQuery(serverConfigFilesQueryOptions(serverId))
}

export const useServerConfigFileContent = (serverId: string, path: string) => {
	return useQuery(serverConfigFileContentQueryOptions(serverId, path))
}

export const useSetServerProperties = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; properties: ServerProperties }) => {
			return rpc.servers.config.set_server_properties(input.serverId, input.properties)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverConfigKeys.properties(variables.serverId),
			})
			void queryClient.invalidateQueries({
				queryKey: serverConfigKeys.propertiesAll(variables.serverId),
			})
		},
	})
}

export const useSetServerPropertiesAll = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; entries: PropertyEntry[] }) => {
			return rpc.servers.config.set_server_properties_all(input.serverId, input.entries)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverConfigKeys.properties(variables.serverId),
			})
			void queryClient.invalidateQueries({
				queryKey: serverConfigKeys.propertiesAll(variables.serverId),
			})
		},
	})
}

export const useAddToServerWhitelist = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; username: string }) => {
			return rpc.servers.config.add_to_server_whitelist(input.serverId, input.username)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverConfigKeys.whitelist(variables.serverId),
			})
		},
	})
}

export const useRemoveFromServerWhitelist = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; username: string }) => {
			return rpc.servers.config.remove_from_server_whitelist(input.serverId, input.username)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverConfigKeys.whitelist(variables.serverId),
			})
		},
	})
}

export const useAddAccessEntry = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; kind: AccessListKind; name: string }) => {
			return rpc.servers.config.add_access_entry(input.serverId, input.kind, input.name)
		},
		onSuccess: (updated, variables) => {
			queryClient.setQueryData(
				serverConfigKeys.accessList(variables.serverId, variables.kind),
				updated,
			)
		},
	})
}

export const useRemoveAccessEntry = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; kind: AccessListKind; name: string }) => {
			return rpc.servers.config.remove_access_entry(input.serverId, input.kind, input.name)
		},
		onSuccess: (updated, variables) => {
			queryClient.setQueryData(
				serverConfigKeys.accessList(variables.serverId, variables.kind),
				updated,
			)
		},
	})
}

export const useWriteServerConfigFile = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; path: string; content: string }) => {
			return rpc.servers.config.write_server_config_file(input.serverId, input.path, input.content)
		},
		onSuccess: (_data, variables) => {
			queryClient.setQueryData(
				serverConfigKeys.fileContent(variables.serverId, variables.path),
				variables.content,
			)
		},
	})
}
