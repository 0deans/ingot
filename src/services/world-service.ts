import { useQuery, useQueryClient } from "@tanstack/react-query"
import { useCallback } from "react"
import type { WorldBackupInfo, WorldDimension, WorldResetResult, WorldSummary } from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

export const worldService = {
	async getAllWorlds(): Promise<WorldSummary[]> {
		return unwrapRpc(rpc.instances.worlds.get_all_worlds())
	},

	async getInstanceWorlds(instanceId: string): Promise<WorldSummary[]> {
		return unwrapRpc(rpc.instances.worlds.get_instance_worlds_full(instanceId))
	},

	async resetDimension(
		instanceId: string,
		folderName: string,
		dimension: WorldDimension,
	): Promise<WorldResetResult> {
		return unwrapRpc(rpc.instances.worlds.reset_world_dimension(instanceId, folderName, dimension))
	},

	async exportWorld(instanceId: string, folderName: string, dest: string | null): Promise<string> {
		return unwrapRpc(rpc.instances.worlds.export_world(instanceId, folderName, dest))
	},

	async backupWorld(instanceId: string, folderName: string): Promise<WorldBackupInfo> {
		return unwrapRpc(rpc.instances.worlds.backup_world(instanceId, folderName))
	},

	async listBackups(instanceId: string, folderName: string | null): Promise<WorldBackupInfo[]> {
		return unwrapRpc(rpc.instances.worlds.list_world_backups(instanceId, folderName))
	},

	async deleteBackup(instanceId: string, fileName: string): Promise<null> {
		return unwrapRpc(rpc.instances.worlds.delete_world_backup(instanceId, fileName))
	},

	async importWorld(instanceId: string, zipPath: string): Promise<WorldSummary> {
		return unwrapRpc(rpc.instances.worlds.import_world(instanceId, zipPath))
	},

	async openWorldFolder(instanceId: string, folderName: string): Promise<null> {
		return unwrapRpc(rpc.instances.worlds.open_world_folder(instanceId, folderName))
	},
}

export const useWorlds = (instanceId?: string) => {
	const queryClient = useQueryClient()
	const targetInstance = instanceId && instanceId !== "all" ? instanceId : undefined

	const query = useQuery({
		queryKey: ["worlds", targetInstance ?? "all"],
		queryFn: () => {
			if (targetInstance) {
				return worldService.getInstanceWorlds(targetInstance)
			}
			return worldService.getAllWorlds()
		},
		staleTime: 1000 * 30,
	})

	const invalidate = useCallback(() => {
		return queryClient.invalidateQueries({ queryKey: ["worlds"] })
	}, [queryClient])

	return {
		worlds: query.data ?? [],
		isLoading: query.isLoading,
		isFetching: query.isFetching,
		error: query.error,
		refetch: query.refetch,
		invalidate,
	}
}
