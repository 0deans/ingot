import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { rpc } from "@/shared/api/rpc"

export const instanceSyncKeys = {
	all: ["instanceSync"] as const,
	sharedStatus: () => {
		return [...instanceSyncKeys.all, "sharedStatus"] as const
	},
	conflict: (instanceId: string) => {
		return [...instanceSyncKeys.all, "conflict", instanceId] as const
	},
}

export const sharedSyncStatusQueryOptions = () => {
	return queryOptions({
		queryKey: instanceSyncKeys.sharedStatus(),
		queryFn: () => {
			return rpc.instances.sync.get_shared_sync_status()
		},
	})
}

export const syncConflictQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceSyncKeys.conflict(instanceId),
		queryFn: () => {
			return rpc.instances.sync.check_sync_conflict(instanceId)
		},
		enabled: instanceId.length > 0,
		staleTime: 0,
	})
}

export const useSharedSyncStatus = () => {
	return useQuery(sharedSyncStatusQueryOptions())
}

export const useSyncConflict = (instanceId: string) => {
	return useQuery(syncConflictQueryOptions(instanceId))
}

export const useExportInstanceCategoryToShared = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; category: string }) => {
			return rpc.instances.sync.export_instance_category_to_shared(input.instanceId, input.category)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceSyncKeys.all })
		},
	})
}

export const usePullInstanceSync = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.sync.pull_instance_sync(instanceId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceSyncKeys.all })
		},
	})
}

export const usePushInstanceSync = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.sync.push_instance_sync(instanceId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceSyncKeys.all })
		},
	})
}

export const useResolveSyncConflict = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; resolution: string }) => {
			return rpc.instances.sync.resolve_sync_conflict(input.instanceId, input.resolution)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceSyncKeys.all })
		},
	})
}
