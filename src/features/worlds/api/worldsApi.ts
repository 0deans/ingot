import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type WorldDimension } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const worldKeys = {
	all: ["worlds"] as const,
	listAll: () => {
		return [...worldKeys.all, "listAll"] as const
	},
	byInstance: (instanceId: string) => {
		return [...worldKeys.all, "instance", instanceId] as const
	},
	backups: (instanceId: string, folderName: string | null) => {
		return [...worldKeys.all, "backups", instanceId, folderName] as const
	},
}

export const allWorldsQueryOptions = () => {
	return queryOptions({
		queryKey: worldKeys.listAll(),
		queryFn: () => {
			return rpc.instances.worlds.get_all_worlds()
		},
	})
}

export const instanceWorldsFullQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: worldKeys.byInstance(instanceId),
		queryFn: () => {
			return rpc.instances.worlds.get_instance_worlds_full(instanceId)
		},
		enabled: instanceId.length > 0,
	})
}

export const worldBackupsQueryOptions = (instanceId: string, folderName: string | null) => {
	return queryOptions({
		queryKey: worldKeys.backups(instanceId, folderName),
		queryFn: () => {
			return rpc.instances.worlds.list_world_backups(instanceId, folderName)
		},
		enabled: instanceId.length > 0,
	})
}

export const useAllWorlds = () => {
	return useQuery(allWorldsQueryOptions())
}

export const useInstanceWorldsFull = (instanceId: string) => {
	return useQuery(instanceWorldsFullQueryOptions(instanceId))
}

export const useWorldBackups = (instanceId: string, folderName: string | null) => {
	return useQuery(worldBackupsQueryOptions(instanceId, folderName))
}

export const useBackupWorld = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; folderName: string }) => {
			return rpc.instances.worlds.backup_world(input.instanceId, input.folderName)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: worldKeys.backups(variables.instanceId, variables.folderName),
			})
		},
	})
}

export const useDeleteWorldBackup = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; fileName: string }) => {
			return rpc.instances.worlds.delete_world_backup(input.instanceId, input.fileName)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: worldKeys.all })
		},
	})
}

export const useExportWorld = () => {
	return useMutation({
		mutationFn: (input: { instanceId: string; folderName: string; dest: string | null }) => {
			return rpc.instances.worlds.export_world(input.instanceId, input.folderName, input.dest)
		},
	})
}

export const useImportWorld = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; zipPath: string }) => {
			return rpc.instances.worlds.import_world(input.instanceId, input.zipPath)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({ queryKey: worldKeys.listAll() })
			void queryClient.invalidateQueries({ queryKey: worldKeys.byInstance(variables.instanceId) })
		},
	})
}

export const useResetWorldDimension = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; folderName: string; dimension: WorldDimension }) => {
			return rpc.instances.worlds.reset_world_dimension(
				input.instanceId,
				input.folderName,
				input.dimension,
			)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: worldKeys.all })
		},
	})
}

export const useOpenWorldFolder = () => {
	return useMutation({
		mutationFn: (input: { instanceId: string; folderName: string }) => {
			return rpc.instances.worlds.open_world_folder(input.instanceId, input.folderName)
		},
	})
}
