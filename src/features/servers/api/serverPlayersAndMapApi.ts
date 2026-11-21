import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { rpc } from "@/shared/api/rpc"

export const serverWorldDataKeys = {
	all: ["serverWorldData"] as const,
	knownPlayers: (serverId: string) => {
		return [...serverWorldDataKeys.all, serverId, "knownPlayers"] as const
	},
	onlinePlayers: (serverId: string) => {
		return [...serverWorldDataKeys.all, serverId, "onlinePlayers"] as const
	},
	onlinePlayerNames: (serverId: string) => {
		return [...serverWorldDataKeys.all, serverId, "onlinePlayerNames"] as const
	},
	playerDetails: (serverId: string, name: string) => {
		return [...serverWorldDataKeys.all, serverId, "playerDetails", name] as const
	},
	mapDimensions: (serverId: string) => {
		return [...serverWorldDataKeys.all, serverId, "mapDimensions"] as const
	},
	mapTile: (serverId: string, dimension: string, x: number, z: number) => {
		return [...serverWorldDataKeys.all, serverId, "mapTile", dimension, x, z] as const
	},
	bedrockPacks: (serverId: string) => {
		return [...serverWorldDataKeys.all, serverId, "bedrockPacks"] as const
	},
}

export const knownPlayersQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.knownPlayers(serverId),
		queryFn: () => {
			return rpc.servers.players.get_known_players(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const onlinePlayersQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.onlinePlayers(serverId),
		queryFn: () => {
			return rpc.servers.players.get_online_players(serverId)
		},
		enabled: serverId.length > 0,
		staleTime: 5_000,
	})
}

export const serverOnlinePlayerNamesQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.onlinePlayerNames(serverId),
		queryFn: () => {
			return rpc.servers.players.get_server_online_players(serverId)
		},
		enabled: serverId.length > 0,
		staleTime: 5_000,
	})
}

export const playerDetailsQueryOptions = (serverId: string, name: string) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.playerDetails(serverId, name),
		queryFn: () => {
			return rpc.servers.players.get_player_details(serverId, name)
		},
		enabled: serverId.length > 0 && name.length > 0,
	})
}

export const mapDimensionsQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.mapDimensions(serverId),
		queryFn: () => {
			return rpc.servers.map.get_map_dimensions(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const mapTileQueryOptions = (serverId: string, dimension: string, x: number, z: number) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.mapTile(serverId, dimension, x, z),
		queryFn: () => {
			return rpc.servers.map.get_map_tile(serverId, dimension, x, z)
		},
		enabled: serverId.length > 0 && dimension.length > 0,
	})
}

export const bedrockPacksQueryOptions = (serverId: string) => {
	return queryOptions({
		queryKey: serverWorldDataKeys.bedrockPacks(serverId),
		queryFn: () => {
			return rpc.servers.packs.list_bedrock_packs(serverId)
		},
		enabled: serverId.length > 0,
	})
}

export const useKnownPlayers = (serverId: string) => {
	return useQuery(knownPlayersQueryOptions(serverId))
}

export const useOnlinePlayers = (serverId: string) => {
	return useQuery(onlinePlayersQueryOptions(serverId))
}

export const useServerOnlinePlayerNames = (serverId: string) => {
	return useQuery(serverOnlinePlayerNamesQueryOptions(serverId))
}

export const usePlayerDetails = (serverId: string, name: string) => {
	return useQuery(playerDetailsQueryOptions(serverId, name))
}

export const useMapDimensions = (serverId: string) => {
	return useQuery(mapDimensionsQueryOptions(serverId))
}

export const useMapTile = (serverId: string, dimension: string, x: number, z: number) => {
	return useQuery(mapTileQueryOptions(serverId, dimension, x, z))
}

export const useBedrockPacks = (serverId: string) => {
	return useQuery(bedrockPacksQueryOptions(serverId))
}

export const useInstallBedrockPack = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; downloadUrl: string; filename: string }) => {
			return rpc.servers.packs.install_bedrock_pack(
				input.serverId,
				input.downloadUrl,
				input.filename,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverWorldDataKeys.bedrockPacks(variables.serverId),
			})
		},
	})
}

export const useSetBedrockPackEnabled = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: {
			serverId: string
			packId: string
			packType: string
			enabled: boolean
		}) => {
			return rpc.servers.packs.set_bedrock_pack_enabled(
				input.serverId,
				input.packId,
				input.packType,
				input.enabled,
			)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverWorldDataKeys.bedrockPacks(variables.serverId),
			})
		},
	})
}

export const useDeleteBedrockPack = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { serverId: string; folderName: string; packType: string }) => {
			return rpc.servers.packs.delete_bedrock_pack(input.serverId, input.folderName, input.packType)
		},
		onSuccess: (_data, variables) => {
			void queryClient.invalidateQueries({
				queryKey: serverWorldDataKeys.bedrockPacks(variables.serverId),
			})
		},
	})
}

export const useOpenBedrockServerSubfolder = () => {
	return useMutation({
		mutationFn: (input: { serverId: string; subfolder: string }) => {
			return rpc.servers.packs.open_bedrock_server_subfolder(input.serverId, input.subfolder)
		},
	})
}
