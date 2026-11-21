import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type InstanceConfig, type ModLoaderType, type ShortcutLocation } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const instanceKeys = {
	all: ["instances"] as const,
	list: () => {
		return [...instanceKeys.all, "list"] as const
	},
	running: () => {
		return [...instanceKeys.all, "running"] as const
	},
	worlds: (instanceId: string) => {
		return [...instanceKeys.all, instanceId, "worlds"] as const
	},
}

export const instancesQueryOptions = () => {
	return queryOptions({
		queryKey: instanceKeys.list(),
		queryFn: () => {
			return rpc.instances.get_instances()
		},
	})
}

export const runningInstancesQueryOptions = () => {
	return queryOptions({
		queryKey: instanceKeys.running(),
		queryFn: () => {
			return rpc.instances.get_running_instances()
		},
		staleTime: 0,
	})
}

export const instanceWorldsQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceKeys.worlds(instanceId),
		queryFn: () => {
			return rpc.instances.get_instance_worlds(instanceId)
		},
		enabled: instanceId.length > 0,
	})
}

export const useInstances = () => {
	return useQuery(instancesQueryOptions())
}

export const useRunningInstances = () => {
	return useQuery(runningInstancesQueryOptions())
}

export const useInstanceWorlds = (instanceId: string) => {
	return useQuery(instanceWorldsQueryOptions(instanceId))
}

export const useCreateInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			name: string
			gameVersion: string
			loader: ModLoaderType
			loaderVersion: string | null
		}) => {
			return rpc.instances.create_instance(
				input.name,
				input.gameVersion,
				input.loader,
				input.loaderVersion,
			)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		},
	})
}

export const useUpdateInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instance: InstanceConfig) => {
			return rpc.instances.update_instance(instance)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		},
	})
}

export const useDeleteInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.delete_instance(instanceId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.all })
		},
	})
}

export const useDuplicateInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; name: string; includeWorlds: boolean }) => {
			return rpc.instances.duplicate_instance(input.instanceId, input.name, input.includeWorlds)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		},
	})
}

export const useLaunchInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			instanceId: string
			quickPlay?: { server: string | null; world: string | null } | null
		}) => {
			return rpc.instances.launch_instance(input.instanceId, input.quickPlay ?? null)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.running() })
		},
	})
}

export const useKillInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.kill_instance(instanceId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.running() })
		},
	})
}

export const useOpenInstanceFolder = () => {
	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.open_instance_folder(instanceId)
		},
	})
}

export const useCreateInstanceShortcut = () => {
	return useMutation({
		mutationFn: (input: { instanceId: string; location: ShortcutLocation }) => {
			return rpc.instances.create_instance_shortcut(input.instanceId, input.location)
		},
	})
}
