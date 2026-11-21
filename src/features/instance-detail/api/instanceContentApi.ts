import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type ContentCategory, type ContentUpdateCandidate } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const instanceContentKeys = {
	all: ["instanceContent"] as const,
	list: (instanceId: string, category: ContentCategory) => {
		return [...instanceContentKeys.all, "list", instanceId, category] as const
	},
	updates: (instanceId: string, category: ContentCategory) => {
		return [...instanceContentKeys.all, "updates", instanceId, category] as const
	},
}

export const instanceContentQueryOptions = (instanceId: string, category: ContentCategory) => {
	return queryOptions({
		queryKey: instanceContentKeys.list(instanceId, category),
		queryFn: () => {
			return rpc.instances.content.list_instance_content(instanceId, category)
		},
		enabled: instanceId.length > 0,
	})
}

export const useInstanceContent = (instanceId: string, category: ContentCategory) => {
	return useQuery(instanceContentQueryOptions(instanceId, category))
}

export const useCheckContentUpdates = () => {
	return useMutation({
		mutationFn: (input: { instanceId: string; category: ContentCategory }) => {
			return rpc.instances.content.check_content_updates(input.instanceId, input.category)
		},
	})
}

export const useApplyContentUpdates = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			instanceId: string
			category: ContentCategory
			selections: ContentUpdateCandidate[]
		}) => {
			return rpc.instances.content.apply_content_updates(
				input.instanceId,
				input.category,
				input.selections,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: instanceContentKeys.list(variables.instanceId, variables.category),
			})
		},
	})
}

export const useIdentifyInstanceContent = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; category: ContentCategory }) => {
			return rpc.instances.content.identify_instance_content(input.instanceId, input.category)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: instanceContentKeys.list(variables.instanceId, variables.category),
			})
		},
	})
}

export const useSetContentEnabled = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			instanceId: string
			category: ContentCategory
			fileName: string
			enabled: boolean
		}) => {
			return rpc.instances.content.set_content_enabled(
				input.instanceId,
				input.category,
				input.fileName,
				input.enabled,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: instanceContentKeys.list(variables.instanceId, variables.category),
			})
		},
	})
}

export const useSetContentEnabledBulk = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			instanceId: string
			category: ContentCategory
			fileNames: string[]
			enabled: boolean
		}) => {
			return rpc.instances.content.set_content_enabled_bulk(
				input.instanceId,
				input.category,
				input.fileNames,
				input.enabled,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: instanceContentKeys.list(variables.instanceId, variables.category),
			})
		},
	})
}

export const useDeleteContent = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; category: ContentCategory; fileNames: string[] }) => {
			return rpc.instances.content.delete_content(input.instanceId, input.category, input.fileNames)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: instanceContentKeys.list(variables.instanceId, variables.category),
			})
		},
	})
}

export const useOpenContentFolder = () => {
	return useMutation({
		mutationFn: (input: {
			instanceId: string
			category: ContentCategory
			world: string | null
		}) => {
			return rpc.instances.content.open_content_folder(
				input.instanceId,
				input.category,
				input.world,
			)
		},
	})
}
