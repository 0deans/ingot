import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type ServerCoreType, type VersionPlan } from "@/bindings"
import { serverKeys } from "@/features/servers/api/serversApi"
import { rpc } from "@/shared/api/rpc"

export const serverVersionChangeKeys = {
	all: ["serverVersionChange"] as const,
	backup: (serverId: string) => {
		return [...serverVersionChangeKeys.all, "backup", serverId] as const
	},
	transactionStatus: (serverId: string) => {
		return [...serverVersionChangeKeys.all, "transactionStatus", serverId] as const
	},
}

export const serverVersionBackupQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverVersionChangeKeys.backup(serverId),
		queryFn: () => {
			return rpc.servers.version_change.get_server_version_backup(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const serverVersionTransactionStatusQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverVersionChangeKeys.transactionStatus(serverId),
		queryFn: () => {
			return rpc.servers.version_change.get_server_version_transaction_status(serverId)
		},
		enabled: serverId.length > 0,
		staleTime: 0,
	})
}

export const useServerVersionBackup = (serverId: string) => {
	return useQuery(serverVersionBackupQueryOptions(serverId))
}

export const useServerVersionTransactionStatus = (serverId: string) => {
	return useQuery(serverVersionTransactionStatusQueryOptions(serverId))
}

export const useCheckServerVersionChange = () => {
	return useMutation({
		mutationFn: (input: { serverId: string; gameVersion: string; core: ServerCoreType }) => {
			return rpc.servers.version_change.check_server_version_change(
				input.serverId,
				input.gameVersion,
				input.core,
			)
		},
	})
}

export const useApplyServerVersionChange = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { plan: VersionPlan; backupWorlds: boolean }) => {
			return rpc.servers.version_change.apply_server_version_change(input.plan, input.backupWorlds)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
			void queryClient.invalidateQueries({ queryKey: serverVersionChangeKeys.all })
		},
	})
}

export const useDiscardServerVersionBackup = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.version_change.discard_server_version_backup(serverId)
		},
		onSuccess: (_data, serverId) => {
			void queryClient.invalidateQueries({ queryKey: serverVersionChangeKeys.backup(serverId) })
		},
	})
}

export const useRecoverServerVersionChange = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.version_change.recover_server_version_change(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
			void queryClient.invalidateQueries({ queryKey: serverVersionChangeKeys.all })
		},
	})
}

export const useUndoServerVersionChange = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (serverId: string) => {
			return rpc.servers.version_change.undo_server_version_change(serverId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
			void queryClient.invalidateQueries({ queryKey: serverVersionChangeKeys.all })
		},
	})
}
