import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { instanceKeys } from "@/features/play/api/instancesApi"
import { STATIC_STALE_TIME_MS } from "@/shared/api/queryClient"
import { rpc } from "@/shared/api/rpc"

export interface ContentSearchParams {
	source: string
	projectType: string
	query: string | null
	gameVersion: string | null
	loader: string | null
	sort: string | null
	page: number
	pageSize: number
}

export const contentKeys = {
	all: ["content"] as const,
	search: (params: ContentSearchParams) => {
		return [...contentKeys.all, "search", params] as const
	},
	details: (source: string, projectId: string) => {
		return [...contentKeys.all, "details", source, projectId] as const
	},
}

export const contentSearchQueryOptions = (params: ContentSearchParams) => {
	return queryOptions({
		queryKey: contentKeys.search(params),
		queryFn: () => {
			return rpc.content.search_content(
				params.source,
				params.projectType,
				params.query,
				params.gameVersion,
				params.loader,
				params.sort,
				params.page,
				params.pageSize,
			)
		},
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const contentDetailsQueryOptions = (source: string, projectId: string) => {
	return queryOptions({
		queryKey: contentKeys.details(source, projectId),
		queryFn: () => {
			return rpc.content.get_content_details(source, projectId)
		},
		enabled: source.length > 0 && projectId.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const useContentSearch = (params: ContentSearchParams) => {
	return useQuery(contentSearchQueryOptions(params))
}

export const useContentDetails = (source: string, projectId: string) => {
	return useQuery(contentDetailsQueryOptions(source, projectId))
}

export const useInstallContentFile = () => {
	return useMutation({
		mutationFn: (input: {
			instanceId: string
			projectType: string
			downloadUrl: string
			filename: string
		}) => {
			return rpc.content.install_content_file(
				input.instanceId,
				input.projectType,
				input.downloadUrl,
				input.filename,
			)
		},
	})
}

export const useInstallModpackInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			name: string
			source: string
			downloadUrl: string
			filename: string
		}) => {
			return rpc.content.install_modpack_instance(
				input.name,
				input.source,
				input.downloadUrl,
				input.filename,
			)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		},
	})
}
