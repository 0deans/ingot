import { queryOptions, useQuery } from "@tanstack/react-query"
import { type ModLoaderType, type ServerCoreType } from "@/bindings"
import { STATIC_STALE_TIME_MS } from "@/shared/api/queryClient"
import { rpc } from "@/shared/api/rpc"

export const versionKeys = {
	all: ["versions"] as const,
	game: () => {
		return [...versionKeys.all, "game"] as const
	},
	horizon: () => {
		return [...versionKeys.all, "horizon"] as const
	},
	loader: (gameVersion: string, loader: ModLoaderType) => {
		return [...versionKeys.all, "loader", gameVersion, loader] as const
	},
	serverCore: (core: ServerCoreType) => {
		return [...versionKeys.all, "serverCore", core] as const
	},
}

export const gameVersionsQueryOptions = () => {
	return queryOptions({
		queryKey: versionKeys.game(),
		queryFn: () => {
			return rpc.versions.get_available_game_versions()
		},
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const horizonReleasesQueryOptions = () => {
	return queryOptions({
		queryKey: versionKeys.horizon(),
		queryFn: () => {
			return rpc.versions.get_available_horizon_releases()
		},
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const loaderVersionsQueryOptions = (gameVersion: string, loader: ModLoaderType) => {
	return queryOptions({
		queryKey: versionKeys.loader(gameVersion, loader),
		queryFn: () => {
			return rpc.versions.get_available_loader_versions(gameVersion, loader)
		},
		enabled: gameVersion.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const serverCoreVersionsQueryOptions = (core: ServerCoreType) => {
	return queryOptions({
		queryKey: versionKeys.serverCore(core),
		queryFn: () => {
			return rpc.versions.get_available_server_core_versions(core)
		},
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const useGameVersions = () => {
	return useQuery(gameVersionsQueryOptions())
}

export const useHorizonReleases = () => {
	return useQuery(horizonReleasesQueryOptions())
}

export const useLoaderVersions = (gameVersion: string, loader: ModLoaderType) => {
	return useQuery(loaderVersionsQueryOptions(gameVersion, loader))
}

export const useServerCoreVersions = (core: ServerCoreType) => {
	return useQuery(serverCoreVersionsQueryOptions(core))
}
