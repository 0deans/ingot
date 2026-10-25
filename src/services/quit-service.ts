import { create } from "zustand"
import type { QuitRequest } from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"
import { dismissCurrentPrompt } from "./quit-flow"

interface QuitApplicationIntent {
	mode: "quit"
}

interface QuitUpdateIntent {
	mode: "update"
	proceed: () => Promise<void>
}

interface QuitRestartIntent {
	mode: "restart"
	proceed: () => Promise<void>
}

export type QuitExitIntent = QuitApplicationIntent | QuitUpdateIntent | QuitRestartIntent
export type QuitMode = QuitExitIntent["mode"]

interface QuitPromptRequest {
	request: QuitRequest
}

export type QuitPrompt = QuitExitIntent & QuitPromptRequest

/** The prompt the quit dialog is showing, if any */
export const useQuitPrompt = create<QuitPrompt | null>(() => {
	return null
})

export const quitService = {
	ask(prompt: QuitPrompt) {
		useQuitPrompt.setState(prompt, true)
	},

	dismiss() {
		useQuitPrompt.setState(null, true)
	},

	dismissCurrent(prompt: QuitPrompt) {
		dismissCurrentPrompt(prompt, useQuitPrompt.getState(), quitService.dismiss)
	},

	/**
	 * Runs `action` (which ends Ingot) right away when no server would be left behind,
	 * otherwise asks to stop the servers first. A running server's console lives in
	 * Ingot: without it the server keeps running out of reach.
	 */
	async guard(mode: Exclude<QuitMode, "quit">, action: () => Promise<void>): Promise<void> {
		const request = await unwrapRpc(rpc.app.get_quit_blockers())
		if (request.servers.length === 0) {
			return action()
		}
		quitService.ask({ mode, request, proceed: action })
	},
}
