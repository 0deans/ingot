import type { ApiError, TauRpcResult } from "@/bindings"
import type { QuitExitIntent } from "@/services/quit-service"
import { unwrapRpc } from "../lib/rpc-result.ts"

interface QuitExitEffects {
	stopServers: () => Promise<TauRpcResult<null, ApiError>>
	quit: () => Promise<TauRpcResult<null, ApiError>>
	dismiss: () => void
}

export const stopServersAndExit = async (
	intent: QuitExitIntent,
	effects: QuitExitEffects,
): Promise<void> => {
	await unwrapRpc(effects.stopServers())
	if (intent.mode === "quit") {
		await unwrapRpc(effects.quit())
		return
	}
	await intent.proceed()
	effects.dismiss()
}

export const dismissCurrentPrompt = (
	expected: QuitExitIntent,
	current: QuitExitIntent | null,
	dismiss: () => void,
): void => {
	if (current !== expected) {
		return
	}
	dismiss()
}
