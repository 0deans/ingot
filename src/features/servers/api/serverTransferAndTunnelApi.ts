import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { type ExportMode } from "@/bindings"
import { serverKeys } from "@/features/servers/api/serversApi"
import { rpc } from "@/shared/api/rpc"

export const serverTunnelKeys = {
	all: ["serverTunnel"] as const,
	playitStatus: () => {
		return [...serverTunnelKeys.all, "playitStatus"] as const
	},
}

export const playitStatusQueryOptions = () => {
	return queryOptions({
		queryKey: serverTunnelKeys.playitStatus(),
		queryFn: () => {
			return rpc.servers.tunnel.get_playit_status()
		},
		staleTime: 5_000,
	})
}

export const usePlayitStatus = () => {
	return useQuery(playitStatusQueryOptions())
}

export const useStartPlayitTunnel = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (secretKey: string | null) => {
			return rpc.servers.tunnel.start_playit_tunnel(secretKey)
		},
		onSuccess: (status) => {
			queryClient.setQueryData(serverTunnelKeys.playitStatus(), status)
		},
	})
}

export const useStopPlayitTunnel = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: () => {
			return rpc.servers.tunnel.stop_playit_tunnel()
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverTunnelKeys.playitStatus() })
		},
	})
}

export const useDuplicateServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			serverId: string
			name: string
			gameVersion: string | null
			buildNumber: string | null
		}) => {
			return rpc.servers.transfer.duplicate_server(
				input.serverId,
				input.name,
				input.gameVersion,
				input.buildNumber,
			)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
		},
	})
}

export const useExportServer = () => {
	return useMutation({
		mutationFn: (input: { serverId: string; mode: ExportMode; dest: string | null }) => {
			return rpc.servers.transfer.export_server(input.serverId, input.mode, input.dest)
		},
	})
}

export const useImportServerUploadChunk = () => {
	return useMutation({
		mutationFn: (input: { uploadId: string; dataBase64: string; first: boolean }) => {
			return rpc.servers.transfer.import_upload_chunk(input.uploadId, input.dataBase64, input.first)
		},
	})
}

export const useImportServer = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (uploadId: string) => {
			return rpc.servers.transfer.import_server(uploadId)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
		},
	})
}
