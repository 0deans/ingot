import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type ExportOptions, type ImportInstanceOptions } from "@/bindings"
import { instanceKeys } from "@/features/play/api/instancesApi"
import { rpc } from "@/shared/api/rpc"

export const instanceTransferKeys = {
	all: ["instanceTransfer"] as const,
	exportPreview: (instanceId: string) => {
		return [...instanceTransferKeys.all, "exportPreview", instanceId] as const
	},
	detectedLaunchers: () => {
		return [...instanceTransferKeys.all, "detectedLaunchers"] as const
	},
	launcherInstances: (launcherId: string, customPath: string | null) => {
		return [...instanceTransferKeys.all, "launcherInstances", launcherId, customPath] as const
	},
}

export const exportPreviewQueryOptions = (instanceId: string) => {
	return queryOptions({
		queryKey: instanceTransferKeys.exportPreview(instanceId),
		queryFn: () => {
			return rpc.instances.export.preview_instance_export(instanceId)
		},
		enabled: instanceId.length > 0,
	})
}

export const detectedLaunchersQueryOptions = () => {
	return queryOptions({
		queryKey: instanceTransferKeys.detectedLaunchers(),
		queryFn: () => {
			return rpc.instances.imports.get_detected_launchers()
		},
	})
}

export const launcherInstancesQueryOptions = (launcherId: string, customPath: string | null) => {
	return queryOptions({
		queryKey: instanceTransferKeys.launcherInstances(launcherId, customPath),
		queryFn: () => {
			return rpc.instances.imports.get_launcher_instances(launcherId, customPath)
		},
		enabled: launcherId.length > 0,
	})
}

export const useInstanceExportPreview = (instanceId: string) => {
	return useQuery(exportPreviewQueryOptions(instanceId))
}

export const useExportInstance = () => {
	return useMutation({
		mutationFn: (input: { instanceId: string; options: ExportOptions; dest: string | null }) => {
			return rpc.instances.export.export_instance(input.instanceId, input.options, input.dest)
		},
	})
}

export const useDetectedLaunchers = () => {
	return useQuery(detectedLaunchersQueryOptions())
}

export const useLauncherInstances = (launcherId: string, customPath: string | null) => {
	return useQuery(launcherInstancesQueryOptions(launcherId, customPath))
}

export const useDetectCustomInstance = () => {
	return useMutation({
		mutationFn: (path: string) => {
			return rpc.instances.imports.detect_custom_instance(path)
		},
	})
}

export const useImportInstance = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (options: ImportInstanceOptions) => {
			return rpc.instances.imports.import_instance(options)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
		},
	})
}
