import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type ModLoaderType, type VersionPlan } from "@/bindings"
import { instanceKeys } from "@/features/play/api/instancesApi"
import { rpc } from "@/shared/api/rpc"

export const instanceVersionChangeKeys = {
	all: ["instanceVersionChange"] as const,
	backup: (instanceId: string) => {
		return [...instanceVersionChangeKeys.all, "backup", instanceId] as const
	},
	transactionStatus: (instanceId: string) => {
		return [...instanceVersionChangeKeys.all, "transactionStatus", instanceId] as const
	},
}

export const instanceVersionBackupQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceVersionChangeKeys.backup(instanceId),
		queryFn: () => {
			return rpc.instances.version_change.get_instance_version_backup(instanceId)
		},
		enabled: instanceId.length > 0,
	})
}

export const instanceVersionTransactionStatusQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceVersionChangeKeys.transactionStatus(instanceId),
		queryFn: () => {
			return rpc.instances.version_change.get_instance_version_transaction_status(instanceId)
		},
		enabled: instanceId.length > 0,
		staleTime: 0,
	})
}

export const useInstanceVersionBackup = (instanceId: string) => {
	return useQuery(instanceVersionBackupQueryOptions(instanceId))
}

export const useInstanceVersionTransactionStatus = (instanceId: string) => {
	return useQuery(instanceVersionTransactionStatusQueryOptions(instanceId))
}

export const useCheckInstanceVersionChange = () => {
	return useMutation({
		mutationFn: (input: {
			instanceId: string
			gameVersion: string
			loader: ModLoaderType
			loaderVersion: string | null
		}) => {
			return rpc.instances.version_change.check_instance_version_change(
				input.instanceId,
				input.gameVersion,
				input.loader,
				input.loaderVersion,
			)
		},
	})
}

export const useApplyInstanceVersionChange = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { plan: VersionPlan; backupWorlds: boolean }) => {
			return rpc.instances.version_change.apply_instance_version_change(
				input.plan,
				input.backupWorlds,
			)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
			void queryClient.invalidateQueries({ queryKey: instanceVersionChangeKeys.all })
		},
	})
}

export const useDiscardInstanceVersionBackup = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.version_change.discard_instance_version_backup(instanceId)
		},
		onSuccess: (_data, instanceId) => {
			void queryClient.invalidateQueries({
				queryKey: instanceVersionChangeKeys.backup(instanceId),
			})
		},
	})
}

export const useRecoverInstanceVersionChange = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.version_change.recover_instance_version_change(instanceId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
			void queryClient.invalidateQueries({ queryKey: instanceVersionChangeKeys.all })
		},
	})
}

export const useUndoInstanceVersionChange = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.version_change.undo_instance_version_change(instanceId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
			void queryClient.invalidateQueries({ queryKey: instanceVersionChangeKeys.all })
		},
	})
}
