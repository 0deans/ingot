import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import type { ContentCategory, ContentListResult, InstalledContent } from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

export const instanceContentService = {
	listContent: async (
		instanceId: string,
		category: ContentCategory,
	): Promise<ContentListResult> => {
		return unwrapRpc(rpc.instances.content.list_instance_content(instanceId, category))
	},

	setContentEnabled: async (
		instanceId: string,
		category: ContentCategory,
		fileName: string,
		enabled: boolean,
	): Promise<string> => {
		return unwrapRpc(
			rpc.instances.content.set_content_enabled(instanceId, category, fileName, enabled),
		)
	},

	setContentEnabledBulk: async (
		instanceId: string,
		category: ContentCategory,
		fileNames: string[],
		enabled: boolean,
	): Promise<string[]> => {
		return unwrapRpc(
			rpc.instances.content.set_content_enabled_bulk(instanceId, category, fileNames, enabled),
		)
	},

	deleteContent: async (
		instanceId: string,
		category: ContentCategory,
		fileNames: string[],
	): Promise<string[]> => {
		return unwrapRpc(rpc.instances.content.delete_content(instanceId, category, fileNames))
	},

	openContentFolder: async (
		instanceId: string,
		category: ContentCategory,
		world?: string,
	): Promise<null> => {
		return unwrapRpc(rpc.instances.content.open_content_folder(instanceId, category, world ?? null))
	},
}

export const useInstanceContent = (instanceId: string, category: ContentCategory) => {
	const query = useQuery({
		queryKey: ["instance-content", instanceId, category],
		queryFn: () => {
			return instanceContentService.listContent(instanceId, category)
		},
		staleTime: 1000 * 15,
	})

	return query
}

export const useToggleContentMutation = (instanceId: string, category: ContentCategory) => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: async ({ fileName, enabled }: { fileName: string; enabled: boolean }) => {
			return instanceContentService.setContentEnabled(instanceId, category, fileName, enabled)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({
				queryKey: ["instance-content", instanceId, category],
			})
		},
	})
}

export const useBulkToggleContentMutation = (instanceId: string, category: ContentCategory) => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: async ({ fileNames, enabled }: { fileNames: string[]; enabled: boolean }) => {
			return instanceContentService.setContentEnabledBulk(instanceId, category, fileNames, enabled)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({
				queryKey: ["instance-content", instanceId, category],
			})
		},
	})
}

export const useDeleteContentMutation = (instanceId: string, category: ContentCategory) => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: async (fileNames: string[]) => {
			return instanceContentService.deleteContent(instanceId, category, fileNames)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({
				queryKey: ["instance-content", instanceId, category],
			})
		},
	})
}

export type { InstalledContent }
