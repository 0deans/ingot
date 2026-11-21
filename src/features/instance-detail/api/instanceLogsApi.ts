import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type LogFileKind, type PasteProvider } from "@/bindings"
import { rpc } from "@/shared/api/rpc"

export const instanceLogKeys = {
	all: ["instanceLogs"] as const,
	live: (instanceId: string) => {
		return [...instanceLogKeys.all, "live", instanceId] as const
	},
	files: (instanceId: string) => {
		return [...instanceLogKeys.all, "files", instanceId] as const
	},
	fileContent: (instanceId: string, kind: LogFileKind, fileName: string) => {
		return [...instanceLogKeys.all, "fileContent", instanceId, kind, fileName] as const
	},
}

export const liveInstanceLogsQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceLogKeys.live(instanceId),
		queryFn: () => {
			return rpc.instances.logs.get_live_instance_logs(instanceId)
		},
		enabled: instanceId.length > 0,
		staleTime: 0,
	})
}

export const instanceLogFilesQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceLogKeys.files(instanceId),
		queryFn: () => {
			return rpc.instances.logs.list_instance_log_files(instanceId)
		},
		enabled: instanceId.length > 0,
	})
}

export const instanceLogFileContentQueryOptions = (
	instanceId: string,
	kind: LogFileKind,
	fileName: string,
) => {
	return queryOptions({
		queryKey: instanceLogKeys.fileContent(instanceId, kind, fileName),
		queryFn: () => {
			return rpc.instances.logs.read_instance_log_file(instanceId, kind, fileName)
		},
		enabled: instanceId.length > 0 && fileName.length > 0,
	})
}

export const useLiveInstanceLogs = (instanceId: string) => {
	return useQuery(liveInstanceLogsQueryOptions(instanceId))
}

export const useInstanceLogFiles = (instanceId: string) => {
	return useQuery(instanceLogFilesQueryOptions(instanceId))
}

export const useInstanceLogFileContent = (
	instanceId: string,
	kind: LogFileKind,
	fileName: string,
) => {
	return useQuery(instanceLogFileContentQueryOptions(instanceId, kind, fileName))
}

export const useClearLiveInstanceLogs = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (instanceId: string) => {
			return rpc.instances.logs.clear_live_instance_logs(instanceId)
		},
		onSuccess: (_data, instanceId) => {
			queryClient.setQueryData(instanceLogKeys.live(instanceId), [])
		},
	})
}

export const useOpenInstanceLogLocation = () => {
	return useMutation({
		mutationFn: (input: {
			instanceId: string
			kind: "log" | "crashReport" | null
			fileName: string | null
		}) => {
			return rpc.instances.logs.open_instance_log_location(
				input.instanceId,
				input.kind,
				input.fileName,
			)
		},
	})
}

export const useSanitizeLogText = () => {
	return useMutation({
		mutationFn: (content: string) => {
			return rpc.instances.logs.sanitize_log_text(content)
		},
	})
}

export const useUploadLogPaste = () => {
	return useMutation({
		mutationFn: (input: { content: string; provider: PasteProvider }) => {
			return rpc.instances.logs.upload_log_paste(input.content, input.provider)
		},
	})
}
