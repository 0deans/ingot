import { useQueryClient } from "@tanstack/react-query"
import { useEffect } from "react"
import { instanceKeys } from "@/features/play/api/instancesApi"
import { serverKeys } from "@/features/servers/api/serversApi"
import { settingsKeys } from "@/features/settings/api/settingsApi"
import { rpc } from "@/shared/api/rpc"
import { isTauriRuntime } from "@/shared/lib/tauriCommands"

export const useBackendEventBridge = () => {
	const queryClient = useQueryClient()

	useEffect(() => {
		if (!isTauriRuntime()) {
			return
		}

		let cancelled = false
		const unlisteners: (() => void)[] = []

		const register = async () => {
			try {
				const subs = await Promise.all([
					rpc.events.on_instance_status_changed.on(() => {
						void queryClient.invalidateQueries({ queryKey: instanceKeys.running() })
						void queryClient.invalidateQueries({ queryKey: instanceKeys.list() })
					}),
					rpc.events.on_server_status_changed.on(() => {
						void queryClient.invalidateQueries({ queryKey: serverKeys.running() })
						void queryClient.invalidateQueries({ queryKey: serverKeys.list() })
					}),
					rpc.events.on_memory_changed.on((nextMemory) => {
						queryClient.setQueryData(settingsKeys.memory(), nextMemory)
					}),
				])

				if (cancelled) {
					for (const unlisten of subs) {
						unlisten()
					}
					return
				}

				unlisteners.push(...subs)
			} catch (error: unknown) {
				console.error("[Ingot] Failed to subscribe to backend event bridge:", error)
			}
		}

		void register()

		return () => {
			cancelled = true
			for (const unlisten of unlisteners) {
				unlisten()
			}
		}
	}, [queryClient])
}
