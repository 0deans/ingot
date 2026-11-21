import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type ServerConfig, type ServerCoreType } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const serverKeys = {
	all: ["servers"] as const,
	list: () => {
		return [...serverKeys.all, "list"] as const
	},
	running: () => {
		return [...serverKeys.all, "running"] as const
	},
	logs: (serverId: string) => {
		return [...serverKeys.all, serverId, "logs"] as const
	},
	stats: (serverId: string) => {
		return [...serverKeys.all, serverId, "stats"] as const
	},
	storage: (serverId: string) => {
		return [...serverKeys.all, serverId, "storage"] as const
	},
	plugins: (serverId: string) => {
		return [...serverKeys.all, serverId, "plugins"] as const
	},
	companion: (serverId: string) => {
		return [...serverKeys.all, serverId, "companion"] as const
	},
	icon: (serverId: string) => {
		return [...serverKeys.all, serverId, "icon"] as const
	},
	ping: (port: number) => {
		return [...serverKeys.all, "ping", port] as const
	},
	lanAddress: () => {
		return [...serverKeys.all, "lanAddress"] as const
	},
}

export const serversQueryOptions = () => {
	return queryOptions({
		queryKey: serverKeys.list(),
		queryFn: () => {
			return rpc.servers.get_servers()
		},
	})
}

export const runningServersQueryOptions = () => {
	return queryOptions({
		queryKey: serverKeys.running(),
		queryFn: () => {
			return rpc.servers.get_running_servers()
		},
		staleTime: 0,
	})
}

export const serverLogsQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverKeys.logs(serverId),
		queryFn: () => {
			return rpc.servers.get_server_logs(serverId)
		},
		enabled: serverId.length > 0,
		staleTime: 0,
	})
}

export const serverStatsQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverKeys.stats(serverId),
		queryFn: () => {
			return rpc.servers.metrics.get_server_stats(serverId)
		},
		enabled: serverId.length > 0,
		staleTime: 5_000,
	})
}

export const serverStorageQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverKeys.storage(serverId),
		queryFn: () => {
			return rpc.servers.metrics.get_server_storage(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverPluginsQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverKeys.plugins(serverId),
		queryFn: () => {
			return rpc.servers.plugins.list_server_plugins(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const useServers = () => {
	return useQuery(serversQueryOptions())
}

export const useRunningServers = () => {
	return useQuery(runningServersQueryOptions())
}

export const useServerLogs = (serverId: string) => {
	return useQuery(serverLogsQueryOptions(serverId))
}

export const useServerStats = (serverId: string) => {
	return useQuery(serverStatsQueryOptions(serverId))
}

export const useServerStorage = (serverId: string) => {
	return useQuery(serverStorageQueryOptions(serverId))
}

export const useServerPlugins = (serverId: string) => {
	return useQuery(serverPluginsQueryOptions(serverId))
}

export const useCreateServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			name: string
			core: ServerCoreType
			gameVersion: string
			buildNumber: string | null
			port: number | null
			memoryMinMb: number | null
			memoryMaxMb: number | null
		}) => {
			return rpc.servers.create_server(
				input.name,
				input.core,
				input.gameVersion,
				input.buildNumber,
				input.port,
				input.memoryMinMb,
				input.memoryMaxMb,
			)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
		},
	})
}

export const useUpdateServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (server: ServerConfig) => {
			return rpc.servers.update_server(server)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
		},
	})
}

export const useDeleteServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; deleteFiles: boolean }) => {
			return rpc.servers.delete_server(input.serverId, input.deleteFiles)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.all })
		},
	})
}

export const useStartServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.start_server(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
		},
	})
}

export const useStopServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.stop_server(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
		},
	})
}

export const useKillServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.kill_server(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
		},
	})
}

export const useSendServerCommand = () => {
	return useMutation({
		mutationFn: (input: { serverId: string; command: string }) => {
			return rpc.servers.send_server_command(input.serverId, input.command)
		},
	})
}

export const useOpenServerFolder = () => {
	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.open_server_folder(serverId)
		},
	})
}

export const serverCompanionStatusQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverKeys.companion(serverId),
		queryFn: () => {
			return rpc.servers.get_companion_status(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverIconQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverKeys.icon(serverId),
		queryFn: () => {
			return rpc.servers.get_server_icon(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverPingQueryOptions = (port: number) => {
	return queryOptions({
		queryKey: serverKeys.ping(port),
		queryFn: () => {
			return rpc.servers.ping_server(port)
		},
		enabled: port > 0,
		staleTime: 0,
	})
}

export const lanAddressQueryOptions = () => {
	return queryOptions({
		queryKey: serverKeys.lanAddress(),
		queryFn: () => {
			return rpc.servers.metrics.get_lan_address()
		},
	})
}

export const useServerCompanionStatus = (serverId: string) => {
	return useQuery(serverCompanionStatusQueryOptions(serverId))
}

export const useInstallServerCompanion = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.install_companion(serverId)
		},
		onSuccess: (_data, serverId) => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.companion(serverId) })
		},
	})
}

export const useServerIcon = (serverId: string) => {
	return useQuery(serverIconQueryOptions(serverId))
}

export const useSetServerIcon = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; base64Data: string }) => {
			return rpc.servers.set_server_icon(input.serverId, input.base64Data)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.icon(variables.serverId) })
		},
	})
}

export const usePingServer = (port: number) => {
	return useQuery(serverPingQueryOptions(port))
}

export const useLanAddress = () => {
	return useQuery(lanAddressQueryOptions())
}

export const usePutServerToSleep = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.put_server_to_sleep(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
		},
	})
}

export const useSaveServerWorld = () => {
	return useMutation({
		mutationFn: (input: { serverId: string; flush: boolean }) => {
			return rpc.servers.save_server_world(input.serverId, input.flush)
		},
	})
}
