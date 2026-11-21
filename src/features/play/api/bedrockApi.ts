import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { rpc } from "@/shared/api/rpc"

export const bedrockKeys = {
	all: ["bedrock"] as const,
	clientStatus: () => {
		return [...bedrockKeys.all, "clientStatus"] as const
	},
}

export const bedrockClientStatusQueryOptions = () => {
	return queryOptions({
		queryKey: bedrockKeys.clientStatus(),
		queryFn: () => {
			return rpc.bedrock.get_bedrock_client_status()
		},
	})
}

export const useBedrockClientStatus = () => {
	return useQuery(bedrockClientStatusQueryOptions())
}

export const useInstallBedrockClient = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: () => {
			return rpc.bedrock.install_bedrock_client()
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: bedrockKeys.clientStatus() })
		},
	})
}

export const useLaunchBedrockClient = () => {
	return useMutation({
		mutationFn: () => {
			return rpc.bedrock.launch_bedrock_client()
		},
	})
}

export const useOpenBedrockFolder = () => {
	return useMutation({
		mutationFn: (kind: string) => {
			return rpc.bedrock.open_bedrock_folder(kind)
		},
	})
}
