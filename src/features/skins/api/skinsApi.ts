import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { accountKeys } from "@/features/settings/api/accountsApi"
import { STATIC_STALE_TIME_MS } from "@/shared/api/queryClient"
import { rpc } from "@/shared/api/rpc"

export interface ElySkinsQueryParams {
	page: number
	query: string | null
	sort: string | null
	model: string | null
	uploader: string | null
}

export const skinKeys = {
	all: ["skins"] as const,
	elyCatalog: (params: ElySkinsQueryParams) => {
		return [...skinKeys.all, "elyCatalog", params] as const
	},
	dataUrl: (skinUrl: string) => {
		return [...skinKeys.all, "dataUrl", skinUrl] as const
	},
	elyCredentials: (accountId: string) => {
		return [...skinKeys.all, "elyCredentials", accountId] as const
	},
}

export const elySkinsQueryOptions = (params: ElySkinsQueryParams) => {
	return queryOptions({
		queryKey: skinKeys.elyCatalog(params),
		queryFn: () => {
			return rpc.skins.get_ely_skins(
				params.page,
				params.query,
				params.sort,
				params.model,
				params.uploader,
			)
		},
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const skinDataUrlQueryOptions = (skinUrl: string) => {
	return queryOptions({
		queryKey: skinKeys.dataUrl(skinUrl),
		queryFn: () => {
			return rpc.skins.get_skin_data_url(skinUrl)
		},
		enabled: skinUrl.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const hasElyWebCredentialsQueryOptions = (accountId: string) => {
	return queryOptions({
		queryKey: skinKeys.elyCredentials(accountId),
		queryFn: () => {
			return rpc.skins.has_ely_web_credentials(accountId)
		},
		enabled: accountId.length > 0,
	})
}

export const useElySkins = (params: ElySkinsQueryParams) => {
	return useQuery(elySkinsQueryOptions(params))
}

export const useSkinDataUrl = (skinUrl: string) => {
	return useQuery(skinDataUrlQueryOptions(skinUrl))
}

export const useHasElyWebCredentials = (accountId: string) => {
	return useQuery(hasElyWebCredentialsQueryOptions(accountId))
}

export const useApplyMicrosoftSkin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { accountId: string; skinUrl: string; isSlim: boolean }) => {
			return rpc.skins.apply_microsoft_skin(input.accountId, input.skinUrl, input.isSlim)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useUploadMicrosoftSkin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { accountId: string; imageBase64: string; isSlim: boolean }) => {
			return rpc.skins.upload_microsoft_skin(input.accountId, input.imageBase64, input.isSlim)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useApplyElySkin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { accountId: string; skinId: number; password: string | null }) => {
			return rpc.skins.apply_ely_skin(input.accountId, input.skinId, input.password)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useUploadElySkin = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { accountId: string; imageBase64: string; password: string | null }) => {
			return rpc.skins.upload_ely_skin(input.accountId, input.imageBase64, input.password)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: accountKeys.list() })
		},
	})
}

export const useSaveSkinToDownloads = () => {
	return useMutation({
		mutationFn: (input: { username: string; skinUrl: string }) => {
			return rpc.skins.save_skin_to_downloads(input.username, input.skinUrl)
		},
	})
}
