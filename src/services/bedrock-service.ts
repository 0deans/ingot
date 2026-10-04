import { useEffect } from "react"
import { create } from "zustand"
import type { BedrockClientStatus } from "@/bindings"
import { rpc } from "@/lib/rpc"

interface BedrockStore {
	status: BedrockClientStatus | null
	isLoading: boolean
	error: string | null
	isLaunching: boolean
}

const useBedrockStore = create<BedrockStore>(() => ({
	status: null,
	isLoading: true,
	error: null,
	isLaunching: false,
}))

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
				const status = await rpc.get_bedrock_client_status()
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
			await rpc.launch_bedrock_client()
		} finally {
			setTimeout(() => {
				useBedrockStore.setState({ isLaunching: false })
			}, 3000)
		}
	},

	async install(): Promise<void> {
		await rpc.install_bedrock_client()
	},

	async openFolder(
		kind: "worlds" | "resource_packs" | "behavior_packs" | "screenshots" | "root",
	): Promise<void> {
		await rpc.open_bedrock_folder(kind)
	},
}

export function useBedrockStatus() {
	const status = useBedrockStore((s) => s.status)
	const isLoading = useBedrockStore((s) => s.isLoading)
	const isLaunching = useBedrockStore((s) => s.isLaunching)
	const error = useBedrockStore((s) => s.error)

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
		refresh: () => bedrockService.refreshStatus(),
		launch: () => bedrockService.launch(),
		install: () => bedrockService.install(),
		openFolder: (kind: "worlds" | "resource_packs" | "behavior_packs" | "screenshots" | "root") =>
			bedrockService.openFolder(kind),
	}
}

export { rpc }
