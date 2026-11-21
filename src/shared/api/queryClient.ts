import { QueryClient } from "@tanstack/react-query"

export const DEFAULT_STALE_TIME_MS = 30_000
export const STATIC_STALE_TIME_MS = 5 * 60_000

export const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			retry: false,
			refetchOnWindowFocus: false,
			staleTime: DEFAULT_STALE_TIME_MS,
		},
		mutations: {
			retry: false,
		},
	},
})
