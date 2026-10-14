import { useEffect } from "react"
import { create } from "zustand"
import type { BedrockClientStatus } from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

interface BedrockStore {
	status: BedrockClientStatus | null
	isLoading: boolean
	error: string | null
	isLaunching: boolean
}

const useBedrockStore = create<BedrockStore>(() => {
	return {
		status: null,
		isLoading: true,
		error: null,
		isLaunching: false,
	}
})

let refreshPromise: Promise<BedrockClientStatus | null> | null = null

export const bedrockService = {
	async getStatus(): Promise<BedrockClientStatus | null> {
		return this.refreshStatus()
	},

	async refreshStatus(): Promise<BedrockClientStatus | null> {
		if (refreshPromise) {
			return refreshPromise
		}

		refreshPromise = (async () => {
			try {
				useBedrockStore.setState({ isLoading: true, error: null })
				const status = await unwrapRpc(rpc.bedrock.get_bedrock_client_status())
				useBedrockStore.setState({ status, isLoading: false })
				return status
			} catch (err) {
				console.error("Failed to detect Bedrock status:", err)
				useBedrockStore.setState({
					isLoading: false,
					error: err instanceof Error ? err.message : String(err),
				})
				return null
			} finally {
				refreshPromise = null
			}
		})()

		return refreshPromise
	},

	async launch(): Promise<void> {
		useBedrockStore.setState({ isLaunching: true })
		try {
			await unwrapRpc(rpc.bedrock.launch_bedrock_client())
		} finally {
			setTimeout(() => {
				useBedrockStore.setState({ isLaunching: false })
			}, 3000)
		}
	},

	async install(): Promise<void> {
		await unwrapRpc(rpc.bedrock.install_bedrock_client())
	},

	async openFolder(
		kind: "worlds" | "resource_packs" | "behavior_packs" | "screenshots" | "root",
	): Promise<void> {
		await unwrapRpc(rpc.bedrock.open_bedrock_folder(kind))
	},
}

export function useBedrockStatus() {
	const status = useBedrockStore((s) => {
		return s.status
	})
	const isLoading = useBedrockStore((s) => {
		return s.isLoading
	})
	const isLaunching = useBedrockStore((s) => {
		return s.isLaunching
	})
	const error = useBedrockStore((s) => {
		return s.error
	})

	useEffect(() => {
		if (!status && isLoading) {
			bedrockService.refreshStatus()
		}
	}, [status, isLoading])

	return {
		status,
		isLoading,
		isLaunching,
		error,
		refresh: () => {
			return bedrockService.refreshStatus()
		},
		launch: () => {
			return bedrockService.launch()
		},
		install: () => {
			return bedrockService.install()
		},
		openFolder: (kind: "worlds" | "resource_packs" | "behavior_packs" | "screenshots" | "root") => {
			return bedrockService.openFolder(kind)
		},
	}
}

export { rpc }
