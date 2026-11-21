import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type SyncSettings, type WindowSettings } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const settingsKeys = {
	all: ["settings"] as const,
	systemMemory: () => {
		return [...settingsKeys.all, "systemMemory"] as const
	},
	memory: () => {
		return [...settingsKeys.all, "memory"] as const
	},
	launcherBehavior: () => {
		return [...settingsKeys.all, "launcherBehavior"] as const
	},
	window: () => {
		return [...settingsKeys.all, "window"] as const
	},
	sync: () => {
		return [...settingsKeys.all, "sync"] as const
	},
}

export const systemMemoryQueryOptions = () => {
	return queryOptions({
		queryKey: settingsKeys.systemMemory(),
		queryFn: () => {
			return rpc.settings.get_system_memory()
		},
		staleTime: Number.POSITIVE_INFINITY,
	})
}

export const memorySettingsQueryOptions = () => {
	return queryOptions({
		queryKey: settingsKeys.memory(),
		queryFn: () => {
			return rpc.settings.get_memory_settings()
		},
	})
}

export const launcherBehaviorQueryOptions = () => {
	return queryOptions({
		queryKey: settingsKeys.launcherBehavior(),
		queryFn: () => {
			return rpc.settings.get_launcher_behavior()
		},
	})
}

export const windowSettingsQueryOptions = () => {
	return queryOptions({
		queryKey: settingsKeys.window(),
		queryFn: () => {
			return rpc.settings.get_window_settings()
		},
	})
}

export const syncSettingsQueryOptions = () => {
	return queryOptions({
		queryKey: settingsKeys.sync(),
		queryFn: () => {
			return rpc.settings.get_sync_settings()
		},
	})
}

export const useSystemMemory = () => {
	return useQuery(systemMemoryQueryOptions())
}

export const useMemorySettings = () => {
	return useQuery(memorySettingsQueryOptions())
}

export const useSetMemorySettings = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { minRamMb: number; maxRamMb: number }) => {
			return rpc.settings.set_memory_settings(input.minRamMb, input.maxRamMb)
		},
		onSuccess: (updated) => {
			queryClient.setQueryData(settingsKeys.memory(), updated)
		},
	})
}

export const useLauncherBehavior = () => {
	return useQuery(launcherBehaviorQueryOptions())
}

export const useSetLauncherBehavior = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (behavior: string) => {
			return rpc.settings.set_launcher_behavior(behavior)
		},
		onSuccess: (updated) => {
			queryClient.setQueryData(settingsKeys.launcherBehavior(), updated)
		},
	})
}

export const useWindowSettings = () => {
	return useQuery(windowSettingsQueryOptions())
}

export const useSetWindowSettings = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (settings: WindowSettings) => {
			return rpc.settings.set_window_settings(settings)
		},
		onSuccess: (updated) => {
			queryClient.setQueryData(settingsKeys.window(), updated)
		},
	})
}

export const useSyncSettings = () => {
	return useQuery(syncSettingsQueryOptions())
}

export const useSetSyncSettings = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (settings: SyncSettings) => {
			return rpc.settings.set_sync_settings(settings)
		},
		onSuccess: (updated) => {
			queryClient.setQueryData(settingsKeys.sync(), updated)
		},
	})
}
