import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type TargetKind } from "@/bindings"
import { serverKeys } from "@/features/servers/api/serversApi"
import { rpc } from "@/shared/api/rpc"

export const appKeys = {
	all: ["app"] as const,
	leftoverServers: () => {
		return [...appKeys.all, "leftoverServers"] as const
	},
	quitBlockers: () => {
		return [...appKeys.all, "quitBlockers"] as const
	},
	shouldShowWindowOnStartup: () => {
		return [...appKeys.all, "shouldShowWindowOnStartup"] as const
	},
	greet: (name: string) => {
		return [...appKeys.all, "greet", name] as const
	},
}

export const leftoverServersQueryOptions = () => {
	return queryOptions({
		queryKey: appKeys.leftoverServers(),
		queryFn: () => {
			return rpc.app.get_leftover_servers()
		},
		staleTime: 0,
	})
}

export const quitBlockersQueryOptions = () => {
	return queryOptions({
		queryKey: appKeys.quitBlockers(),
		queryFn: () => {
			return rpc.app.get_quit_blockers()
		},
		staleTime: 0,
	})
}

export const shouldShowWindowOnStartupQueryOptions = () => {
	return queryOptions({
		queryKey: appKeys.shouldShowWindowOnStartup(),
		queryFn: () => {
			return rpc.app.should_show_window_on_startup()
		},
		staleTime: Number.POSITIVE_INFINITY,
	})
}

export const useLeftoverServers = () => {
	return useQuery(leftoverServersQueryOptions())
}

export const useQuitBlockers = () => {
	return useQuery(quitBlockersQueryOptions())
}

export const useShouldShowWindowOnStartup = () => {
	return useQuery(shouldShowWindowOnStartupQueryOptions())
}

export const useGreet = () => {
	return useMutation({
		mutationFn: (name: string) => {
			return rpc.app.greet(name)
		},
	})
}

export const useQuitApp = () => {
	return useMutation({
		mutationFn: () => {
			return rpc.app.quit_app()
		},
	})
}

export const useStopAllServers = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: () => {
			return rpc.app.stop_all_servers()
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
			void queryClient.invalidateQueries({ queryKey: appKeys.leftoverServers() })
		},
	})
}

export const useStopLeftoverServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.app.stop_leftover_server(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: appKeys.leftoverServers() })
			void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
		},
	})
}

export const useTurnOffAfterVersionChange = () => {
	return useMutation({
		mutationFn: (input: {
			targetKind: TargetKind
			targetId: string
			folder: string
			fileName: string
		}) => {
			return rpc.app.turn_off_after_version_change(
				input.targetKind,
				input.targetId,
				input.folder,
				input.fileName,
			)
		},
	})
}
